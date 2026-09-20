use crate::{invalid, Digest, Result};
use sha2::{Digest as _, Sha256};
use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Component, Path},
};
use zip::ZipArchive;
const MAX_ARCHIVE: u64 = 128 * 1024 * 1024;
const MAX_ENTRY: u64 = 128 * 1024 * 1024;
const MAX_TOTAL: u64 = 256 * 1024 * 1024;
pub(crate) struct Wheel {
    archive: ZipArchive<File>,
}
pub(crate) fn hash_file(path: &Path) -> Result<Digest> {
    let mut file = File::open(path)?;
    hash_reader(&mut file, u64::MAX)
}
pub(crate) fn hash_reader(reader: &mut impl Read, limit: u64) -> Result<Digest> {
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut count = 0u64;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        count = count
            .checked_add(n as u64)
            .ok_or_else(|| invalid("size overflow"))?;
        if count > limit {
            return Err(invalid("size limit exceeded"));
        }
        hash.update(&buf[..n]);
    }
    Ok(Digest::from_hash(hash))
}
impl Wheel {
    pub(crate) fn open(path: &Path, expected: &Digest) -> Result<Self> {
        let mut file = File::open(path)?;
        if file.metadata()?.len() > MAX_ARCHIVE {
            return Err(invalid("archive too large"));
        }
        if hash_reader(&mut file, MAX_ARCHIVE)? != *expected {
            return Err(invalid("official archive SHA-256 mismatch"));
        }
        let declared = entry_count(&mut file)?;
        file.seek(SeekFrom::Start(0))?;
        let mut archive = ZipArchive::new(file)?;
        if archive.len() != declared {
            return Err(invalid("duplicate ZIP names or invalid directory count"));
        }
        if archive.len() > 1024 {
            return Err(invalid("too many archive entries"));
        }
        let mut names = HashSet::new();
        let mut total = 0u64;
        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            valid_name(entry.name())?;
            if !names.insert(entry.name().to_owned()) {
                return Err(invalid("duplicate archive destination"));
            }
            if entry
                .unix_mode()
                .is_some_and(|mode| !matches!(mode & 0o170000, 0 | 0o100000 | 0o040000))
            {
                return Err(invalid("links and special archive entries are unsupported"));
            }
            total = total
                .checked_add(entry.size())
                .ok_or_else(|| invalid("archive size overflow"))?;
            if entry.size() > MAX_ENTRY || total > MAX_TOTAL {
                return Err(invalid("expanded archive exceeds size limit"));
            }
        }
        Ok(Self { archive })
    }
    pub(crate) fn read(&mut self, name: &str, expected: &Digest) -> Result<Vec<u8>> {
        valid_name(name)?;
        let entry = self.archive.by_name(name)?;
        let size = usize::try_from(entry.size()).map_err(|_| invalid("entry size overflow"))?;
        let mut bytes = Vec::with_capacity(size);
        entry.take(MAX_ENTRY + 1).read_to_end(&mut bytes)?;
        if bytes.len() != size || bytes.len() as u64 > MAX_ENTRY || Digest::of(&bytes) != *expected
        {
            return Err(invalid(format!(
                "archive entry checksum/size mismatch: {name}"
            )));
        }
        Ok(bytes)
    }
}
pub(crate) fn valid_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.contains('\\')
        || name.contains(':')
        || name.starts_with('/')
        || name
            .split('/')
            .any(|p| p == ".." || p == "." || p.is_empty())
        || Path::new(name)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(invalid("unsafe archive path"));
    }
    Ok(())
}
#[cfg(test)]
mod tests;

fn entry_count(file: &mut File) -> Result<usize> {
    let length = file.metadata()?.len();
    let tail_len = length.min(65557) as usize;
    file.seek(SeekFrom::End(-(tail_len as i64)))?;
    let mut tail = vec![0; tail_len];
    file.read_exact(&mut tail)?;
    for index in (0..tail.len().saturating_sub(21)).rev() {
        if &tail[index..index + 4] != b"PK\x05\x06" {
            continue;
        }
        let word = |offset| u16::from_le_bytes([tail[index + offset], tail[index + offset + 1]]);
        if index + 22 + usize::from(word(20)) != tail.len() {
            continue;
        }
        if word(4) != 0 || word(6) != 0 || word(8) != word(10) || word(10) == u16::MAX {
            return Err(invalid("multipart and ZIP64 archives are unsupported"));
        }
        return Ok(usize::from(word(10)));
    }
    Err(invalid("missing ZIP directory footer"))
}
