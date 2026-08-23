//! Read-only Bedrock LevelDB table and write-ahead-log decoding.
//!
//! The reader only opens and reads `.ldb`/`.log` files.  It does not acquire a
//! LevelDB lock, write a manifest, compact records, or mutate the live world.
//! Corrupt and unsupported inputs are explicit errors for callers that need to
//! distinguish "no player database" from "database could not be read".

use flate2::read::DeflateDecoder;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

pub const MAX_LEVELDB_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_LEVELDB_VALUE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_LEVELDB_KEYS: usize = 100_000;
pub const MAX_LEVELDB_BLOCK_BYTES: usize = 16 * 1024 * 1024;
const LOG_BLOCK_SIZE: usize = 32_768;
const LOG_HEADER_SIZE: usize = 7;
const MAGIC_LO: u32 = 0x8b80fb57;
const MAGIC_HI: u32 = 0xdb477524;

#[derive(Debug)]
pub enum LevelDbError {
    Unavailable(io::Error),
    Corrupt(&'static str),
    Unsupported(&'static str),
    LimitExceeded(&'static str),
}

impl fmt::Display for LevelDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(error) => write!(f, "LevelDB unavailable: {error}"),
            Self::Corrupt(reason) => write!(f, "corrupt LevelDB: {reason}"),
            Self::Unsupported(reason) => write!(f, "unsupported LevelDB: {reason}"),
            Self::LimitExceeded(reason) => write!(f, "LevelDB limit exceeded: {reason}"),
        }
    }
}

impl std::error::Error for LevelDbError {}

pub fn read_player_data(path: &Path) -> Result<BTreeMap<String, Vec<u8>>, LevelDbError> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries
            .collect::<Result<Vec<_>, _>>()
            .map_err(LevelDbError::Unavailable)?,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(LevelDbError::Unavailable(error)),
    };
    let mut result = BTreeMap::new();
    let mut logs = Vec::new();
    for entry in entries {
        let file_type = entry.file_type().map_err(LevelDbError::Unavailable)?;
        if !file_type.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".ldb") {
            parse_sst(&entry.path(), &mut result)?;
        } else if name.ends_with(".log") {
            logs.push(entry.path());
        }
    }
    logs.sort();
    for path in logs {
        parse_log(&path, &mut result)?;
    }
    Ok(result)
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, LevelDbError> {
    let metadata = fs::metadata(path).map_err(LevelDbError::Unavailable)?;
    if metadata.len() > MAX_LEVELDB_FILE_BYTES {
        return Err(LevelDbError::LimitExceeded("database file bytes"));
    }
    fs::read(path).map_err(LevelDbError::Unavailable)
}

fn parse_sst(path: &Path, result: &mut BTreeMap<String, Vec<u8>>) -> Result<(), LevelDbError> {
    let data = read_bounded(path)?;
    if data.len() < 48 {
        return Err(LevelDbError::Corrupt("table shorter than footer"));
    }
    let magic = data.len() - 8;
    if read_le32(&data, magic) != MAGIC_LO || read_le32(&data, magic + 4) != MAGIC_HI {
        return Err(LevelDbError::Corrupt("bad table magic"));
    }
    let mut cursor = data.len() - 48;
    let _ = read_varint(&data, &mut cursor)?;
    let _ = read_varint(&data, &mut cursor)?;
    let index_offset = read_varint(&data, &mut cursor)?;
    let index_size = read_varint(&data, &mut cursor)?;
    let index = read_block(&data, index_offset, index_size)?;
    for (offset, size) in parse_index_block(&index)? {
        let block = read_block(&data, offset, size)?;
        parse_data_block(&block, result)?;
    }
    Ok(())
}

fn read_block(data: &[u8], offset: u64, size: u64) -> Result<Vec<u8>, LevelDbError> {
    let offset =
        usize::try_from(offset).map_err(|_| LevelDbError::Corrupt("block offset overflow"))?;
    let size = usize::try_from(size).map_err(|_| LevelDbError::Corrupt("block size overflow"))?;
    if size > MAX_LEVELDB_BLOCK_BYTES {
        return Err(LevelDbError::LimitExceeded("block bytes"));
    }
    let end = offset
        .checked_add(size)
        .and_then(|end| end.checked_add(5))
        .ok_or(LevelDbError::Corrupt("block range overflow"))?;
    if end > data.len() {
        return Err(LevelDbError::Corrupt("truncated block"));
    }
    let raw = &data[offset..offset + size];
    match data[offset + size] {
        0 => Ok(raw.to_vec()),
        4 => {
            let mut decoder = DeflateDecoder::new(raw);
            let mut output = Vec::new();
            decoder
                .by_ref()
                .take((MAX_LEVELDB_BLOCK_BYTES + 1) as u64)
                .read_to_end(&mut output)
                .map_err(|_| LevelDbError::Corrupt("raw deflate block"))?;
            if output.len() > MAX_LEVELDB_BLOCK_BYTES {
                return Err(LevelDbError::LimitExceeded("decompressed block bytes"));
            }
            Ok(output)
        }
        _ => Err(LevelDbError::Unsupported("block compression")),
    }
}

fn parse_index_block(block: &[u8]) -> Result<Vec<(u64, u64)>, LevelDbError> {
    let restart_start = restart_array_start(block)?;
    let mut cursor = 0;
    let mut previous = Vec::new();
    let mut handles = Vec::new();
    while cursor < restart_start {
        let shared = usize::try_from(read_varint(block, &mut cursor)?)
            .map_err(|_| LevelDbError::Corrupt("shared-key length overflow"))?;
        let non_shared = usize::try_from(read_varint(block, &mut cursor)?)
            .map_err(|_| LevelDbError::Corrupt("key length overflow"))?;
        let value_length = usize::try_from(read_varint(block, &mut cursor)?)
            .map_err(|_| LevelDbError::Corrupt("value length overflow"))?;
        if shared > previous.len()
            || cursor.checked_add(non_shared + value_length).is_none()
            || cursor + non_shared + value_length > restart_start
        {
            return Err(LevelDbError::Corrupt("index entry bounds"));
        }
        previous.truncate(shared);
        previous.extend_from_slice(&block[cursor..cursor + non_shared]);
        cursor += non_shared;
        let mut handle_cursor = cursor;
        let offset = read_varint(block, &mut handle_cursor)?;
        let size = read_varint(block, &mut handle_cursor)?;
        handles.push((offset, size));
        cursor += value_length;
    }
    Ok(handles)
}

fn parse_data_block(
    block: &[u8],
    result: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), LevelDbError> {
    let restart_start = restart_array_start(block)?;
    let mut cursor = 0;
    let mut previous = Vec::new();
    while cursor < restart_start {
        let shared = usize::try_from(read_varint(block, &mut cursor)?)
            .map_err(|_| LevelDbError::Corrupt("shared-key length overflow"))?;
        let non_shared = usize::try_from(read_varint(block, &mut cursor)?)
            .map_err(|_| LevelDbError::Corrupt("key length overflow"))?;
        let value_length = usize::try_from(read_varint(block, &mut cursor)?)
            .map_err(|_| LevelDbError::Corrupt("value length overflow"))?;
        if shared > previous.len() || cursor + non_shared + value_length > restart_start {
            return Err(LevelDbError::Corrupt("data entry bounds"));
        }
        previous.truncate(shared);
        previous.extend_from_slice(&block[cursor..cursor + non_shared]);
        cursor += non_shared;
        let value = &block[cursor..cursor + value_length];
        cursor += value_length;
        if previous.len() <= 8 {
            continue;
        }
        let type_and_sequence = read_le64(&previous, previous.len() - 8);
        if type_and_sequence & 0xff != 1 {
            continue;
        }
        let key = String::from_utf8_lossy(&previous[..previous.len() - 8]);
        if !is_player_key(&key) {
            continue;
        }
        if value.len() > MAX_LEVELDB_VALUE_BYTES {
            return Err(LevelDbError::LimitExceeded("player value bytes"));
        }
        result
            .entry(key.into_owned())
            .or_insert_with(|| value.to_vec());
        if result.len() > MAX_LEVELDB_KEYS {
            return Err(LevelDbError::LimitExceeded("player key count"));
        }
    }
    Ok(())
}

fn restart_array_start(block: &[u8]) -> Result<usize, LevelDbError> {
    if block.len() < 4 {
        return Err(LevelDbError::Corrupt("block has no restart count"));
    }
    let count = usize::try_from(read_le32(block, block.len() - 4))
        .map_err(|_| LevelDbError::Corrupt("restart count overflow"))?;
    let bytes = count
        .checked_mul(4)
        .ok_or(LevelDbError::Corrupt("restart array overflow"))?;
    block
        .len()
        .checked_sub(4 + bytes)
        .ok_or(LevelDbError::Corrupt("restart array bounds"))
}

fn parse_log(path: &Path, result: &mut BTreeMap<String, Vec<u8>>) -> Result<(), LevelDbError> {
    let data = read_bounded(path)?;
    let mut pending = Vec::new();
    let mut offset = 0;
    while offset + LOG_HEADER_SIZE <= data.len() {
        let into_block = offset % LOG_BLOCK_SIZE;
        if LOG_BLOCK_SIZE - into_block < LOG_HEADER_SIZE {
            offset += LOG_BLOCK_SIZE - into_block;
            continue;
        }
        let length = usize::from(read_le16(&data, offset + 4));
        let record_type = data[offset + 6];
        offset += LOG_HEADER_SIZE;
        if offset + length > data.len() {
            return Err(LevelDbError::Corrupt("truncated WAL record"));
        }
        let fragment = &data[offset..offset + length];
        offset += length;
        match record_type {
            1 => {
                apply_write_batch(fragment, result)?;
                pending.clear();
            }
            2 => {
                pending.clear();
                pending.extend_from_slice(fragment);
            }
            3 => pending.extend_from_slice(fragment),
            4 => {
                pending.extend_from_slice(fragment);
                apply_write_batch(&pending, result)?;
                pending.clear();
            }
            _ => pending.clear(),
        }
    }
    Ok(())
}

fn apply_write_batch(
    batch: &[u8],
    result: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), LevelDbError> {
    if batch.len() < 12 {
        return Err(LevelDbError::Corrupt("short write batch"));
    }
    let count = read_le32(batch, 8) as usize;
    if count > MAX_LEVELDB_KEYS {
        return Err(LevelDbError::LimitExceeded("write-batch operations"));
    }
    let mut cursor = 12;
    for _ in 0..count {
        let operation = *batch
            .get(cursor)
            .ok_or(LevelDbError::Corrupt("truncated write operation"))?;
        cursor += 1;
        let key_length = usize::try_from(read_varint(batch, &mut cursor)?)
            .map_err(|_| LevelDbError::Corrupt("key length overflow"))?;
        let key_end = cursor
            .checked_add(key_length)
            .ok_or(LevelDbError::Corrupt("key bounds"))?;
        if key_end > batch.len() {
            return Err(LevelDbError::Corrupt("truncated write key"));
        }
        let key = String::from_utf8_lossy(&batch[cursor..key_end]).into_owned();
        cursor = key_end;
        match operation {
            1 => {
                let value_length = usize::try_from(read_varint(batch, &mut cursor)?)
                    .map_err(|_| LevelDbError::Corrupt("value length overflow"))?;
                if value_length > MAX_LEVELDB_VALUE_BYTES {
                    return Err(LevelDbError::LimitExceeded("player value bytes"));
                }
                let value_end = cursor
                    .checked_add(value_length)
                    .ok_or(LevelDbError::Corrupt("value bounds"))?;
                if value_end > batch.len() {
                    return Err(LevelDbError::Corrupt("truncated write value"));
                }
                if is_player_key(&key) {
                    result.insert(key, batch[cursor..value_end].to_vec());
                }
                cursor = value_end;
            }
            0 => {
                if is_player_key(&key) {
                    result.remove(&key);
                }
            }
            _ => return Err(LevelDbError::Unsupported("write-batch operation")),
        }
    }
    Ok(())
}

fn is_player_key(key: &str) -> bool {
    key.starts_with("player_") || key == "~local_player"
}

fn read_varint(data: &[u8], cursor: &mut usize) -> Result<u64, LevelDbError> {
    let mut value = 0;
    for shift in (0..64).step_by(7) {
        let byte = *data
            .get(*cursor)
            .ok_or(LevelDbError::Corrupt("truncated varint"))?;
        *cursor += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(LevelDbError::Corrupt("varint overflow"))
}

fn read_le16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_le32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn read_le64(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap())
}
