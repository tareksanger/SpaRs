use super::*;
use std::{
    fs,
    io::{Cursor, Write},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct ArchiveFixture {
    directory: PathBuf,
    path: PathBuf,
    digest: Digest,
}

impl ArchiveFixture {
    fn new(bytes: &[u8]) -> Self {
        let directory = loop {
            let id = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let directory = std::env::temp_dir()
                .join(format!("spars-archive-test-{}-{id}", std::process::id()));
            match fs::create_dir(&directory) {
                Ok(()) => break directory,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create archive fixture directory: {error}"),
            }
        };
        let path = directory.join("fixture.whl");
        fs::write(&path, bytes).unwrap();
        Self {
            directory,
            path,
            digest: Digest::of(bytes),
        }
    }

    fn open(&self) -> Result<Wheel> {
        Wheel::open(&self.path, &self.digest, Limits::default())
    }
}

impl Drop for ArchiveFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, bytes) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn error<T>(result: Result<T>) -> crate::Error {
    match result {
        Err(error) => error,
        Ok(_) => panic!("malformed archive was accepted"),
    }
}

#[test]
fn release_limits_apply_to_archive_entries_and_expanded_total() {
    let fixture = ArchiveFixture::new(&archive(&[("one", b"abc"), ("two", b"def")]));
    let limits = Limits {
        archive: fs::metadata(&fixture.path).unwrap().len(),
        entry: 3,
        expanded: 6,
        installed_file: 6,
    };
    assert!(Wheel::open(&fixture.path, &fixture.digest, limits).is_ok());
    for smaller in [
        Limits {
            archive: limits.archive - 1,
            ..limits
        },
        Limits { entry: 2, ..limits },
        Limits {
            expanded: 5,
            ..limits
        },
    ] {
        assert!(Wheel::open(&fixture.path, &fixture.digest, smaller).is_err());
    }
}

#[test]
fn rejects_paths_and_size_limit() {
    for value in ["../x", "/x", "a/../b", "a\\b", "C:x", "a//b", "a/./b", ""] {
        assert!(valid_name(value).is_err(), "{value}");
    }
    assert!(valid_name("model/vectors").is_ok());
    assert!(hash_reader(&mut &b"abc"[..], 2).is_err());
    assert_eq!(
        hash_reader(&mut &b"abc"[..], 3).unwrap(),
        Digest::of(b"abc")
    );
}

#[test]
fn archive_and_entry_digests_are_checked_independently() {
    let fixture = ArchiveFixture::new(&archive(&[("model/weights", b"tensor bytes")]));
    let wrong_digest = Digest::of(b"wrong archive");
    assert!(
        error(Wheel::open(&fixture.path, &wrong_digest, Limits::default()))
            .to_string()
            .contains("SHA-256")
    );

    let mut wheel = fixture.open().unwrap();
    assert!(
        error(wheel.read("model/weights", &Digest::of(b"wrong entry")))
            .to_string()
            .contains("checksum")
    );
    assert_eq!(
        wheel
            .read("model/weights", &Digest::of(b"tensor bytes"))
            .unwrap(),
        b"tensor bytes"
    );
    assert!(wheel
        .read("model/missing", &Digest::of(b"anything"))
        .is_err());
}

#[test]
fn rejects_duplicate_archive_destinations_before_reading() {
    let mut bytes = archive(&[("model/a.bin", b"first"), ("model/b.bin", b"second")]);
    // ZipWriter refuses duplicates. Change both local and central-directory
    // names without changing their lengths or file payload checksums.
    let positions = bytes
        .windows(b"model/b.bin".len())
        .enumerate()
        .filter_map(|(offset, value)| (value == b"model/b.bin").then_some(offset))
        .collect::<Vec<_>>();
    assert_eq!(positions.len(), 2);
    for offset in positions {
        bytes[offset..offset + b"model/a.bin".len()].copy_from_slice(b"model/a.bin");
    }
    let fixture = ArchiveFixture::new(&bytes);
    assert!(
        fixture.open().is_err(),
        "duplicate ZIP names must not silently replace a destination"
    );
}

#[test]
fn rejects_unsafe_names_in_real_zip_entries() {
    for name in [
        "../outside",
        "model/../../outside",
        "/outside",
        "model\\outside",
        "C:outside",
    ] {
        let fixture = ArchiveFixture::new(&archive(&[(name, b"payload")]));
        assert!(
            error(fixture.open())
                .to_string()
                .contains("unsafe archive path"),
            "{name}"
        );
    }
}

#[test]
fn rejects_symbolic_link_zip_entries() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .add_symlink("model/link", "../outside", SimpleFileOptions::default())
        .unwrap();
    let fixture = ArchiveFixture::new(&writer.finish().unwrap().into_inner());
    assert!(error(fixture.open()).to_string().contains("links"));
}

#[test]
fn rejects_truncated_zip_even_with_matching_archive_digest() {
    let bytes = archive(&[("model/weights", b"tensor bytes")]);
    for length in [0, 4, 20, bytes.len() - 10] {
        // The supplied digest covers the truncated bytes. This exercises ZIP
        // validation rather than stopping at the outer checksum check.
        let fixture = ArchiveFixture::new(&bytes[..length]);
        assert!(fixture.open().is_err(), "accepted archive length {length}");
    }
}
