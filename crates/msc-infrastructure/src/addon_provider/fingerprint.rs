//! Candidate fingerprint plus exact byte identity from two bounded disk passes.
use msc_domain::curseforge_fingerprint::{Fingerprint, ignored};
use sha1::{Digest, Sha1};
use sha2::Sha512;
use std::io::{Read, Seek};
use std::path::Path;

pub struct FileIdentity {
    pub fingerprint: u32,
    pub sha1: String,
    pub sha512: String,
    pub byte_size: u64,
}

pub fn identify(path: &Path) -> Result<FileIdentity, String> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > 512 * 1024 * 1024 {
        return Err("Identification requires a regular file no larger than 512 MiB".into());
    }
    let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut buffer = [0u8; 128 * 1024];
    let mut filtered = 0u32;
    let mut first = Sha1::new();
    let mut size = 0u64;
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        size += count as u64;
        if size > 512 * 1024 * 1024 {
            return Err("File grew beyond the identification limit".into());
        }
        filtered += buffer[..count]
            .iter()
            .filter(|&&byte| !ignored(byte))
            .count() as u32;
        first.update(&buffer[..count]);
    }
    let first = format!("{:x}", first.finalize());
    file.rewind().map_err(|error| error.to_string())?;
    let mut fingerprint = Fingerprint::new(filtered);
    let mut second = Sha1::new();
    let mut sha512 = Sha512::new();
    let mut second_size = 0u64;
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        second_size += count as u64;
        if second_size > size {
            return Err("File changed during identification".into());
        }
        fingerprint.update(&buffer[..count]);
        second.update(&buffer[..count]);
        sha512.update(&buffer[..count]);
    }
    if second_size != size || first != format!("{:x}", second.finalize()) {
        return Err("File changed during identification".into());
    }
    Ok(FileIdentity {
        fingerprint: fingerprint.finish().map_err(str::to_string)?,
        sha1: first,
        sha512: format!("{:x}", sha512.finalize()),
        byte_size: size,
    })
}
