use crate::{invalid, Digest, Result};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
    time::Duration,
};
pub(crate) fn download(destination: &Path, release: &crate::catalog::Release) -> Result<()> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .https_only(true)
        .max_redirects(5)
        .timeout_global(Some(Duration::from_secs(600)))
        .build()
        .into();
    fetch(
        &agent,
        &release.url,
        destination,
        &release.wheel_sha256,
        release.limits.archive,
    )
}
fn fetch(
    agent: &ureq::Agent,
    url: &str,
    destination: &Path,
    expected: &Digest,
    limit: u64,
) -> Result<()> {
    let mut last = None;
    for attempt in 0..3 {
        let result = (|| {
            let mut response = agent
                .get(url)
                .call()
                .map_err(|e| invalid(format!("download: {e}")))?;
            let reader = response.body_mut().as_reader();
            copy_checked(reader, File::create(destination)?, expected, limit)
        })();
        match result {
            Ok(()) => return Ok(()),
            Err(error) => last = Some(error),
        }
        if attempt < 2 {
            std::thread::sleep(Duration::from_millis(100 * (attempt + 1)));
        }
    }
    let _ = std::fs::remove_file(destination);
    Err(last.unwrap_or_else(|| invalid("download failed")))
}
fn copy_checked(
    mut reader: impl Read,
    mut output: File,
    expected: &Digest,
    limit: u64,
) -> Result<()> {
    use sha2::{Digest as _, Sha256};
    let mut hash = Sha256::new();
    let mut count = 0u64;
    let mut bytes = [0; 65536];
    loop {
        let n = reader.read(&mut bytes)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > limit {
            return Err(invalid("download exceeds size limit"));
        }
        hash.update(&bytes[..n]);
        output.write_all(&bytes[..n])?;
    }
    output.sync_all()?;
    if Digest::from_hash(hash) != *expected {
        return Err(invalid("download checksum mismatch"));
    }
    Ok(())
}
#[cfg(test)]
mod tests;
