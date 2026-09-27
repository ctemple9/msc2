//! Bounded edits to Java level.dat that preserve unrelated and mod-owned tags.

use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use msc_domain::nbt::NbtValue;
use std::collections::BTreeMap;
use std::io::{self, Read, Write};

const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_ITEMS: usize = 100_000;

fn invalid() -> io::Error {
    io::Error::other("Invalid or oversized Java level.dat")
}

pub fn update_level_dat(gzip: &[u8], fields: &BTreeMap<String, NbtValue>) -> io::Result<Vec<u8>> {
    let mut raw = Vec::new();
    GzDecoder::new(gzip)
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut raw)?;
    if raw.len() > MAX_BYTES || raw.first() != Some(&10) {
        return Err(invalid());
    }
    let mut cursor = 1;
    string_bytes(&raw, &mut cursor)?;
    let root = BTreeMap::from([("Data".into(), NbtValue::Compound(fields.clone()))]);
    let mut output = raw[..cursor].to_vec();
    output.extend(patch_compound(&raw[cursor..], &root, 0)?);
    if output.len() > MAX_BYTES {
        return Err(invalid());
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&output)?;
    encoder.finish()
}

fn take<'a>(raw: &'a [u8], cursor: &mut usize, count: usize) -> io::Result<&'a [u8]> {
    let end = cursor.checked_add(count).ok_or_else(invalid)?;
    let bytes = raw.get(*cursor..end).ok_or_else(invalid)?;
    *cursor = end;
    Ok(bytes)
}
fn byte(raw: &[u8], cursor: &mut usize) -> io::Result<u8> {
    Ok(take(raw, cursor, 1)?[0])
}
fn count(raw: &[u8], cursor: &mut usize) -> io::Result<usize> {
    let value = i32::from_be_bytes(take(raw, cursor, 4)?.try_into().unwrap());
    usize::try_from(value)
        .ok()
        .filter(|n| *n <= MAX_BYTES)
        .ok_or_else(invalid)
}
fn string_bytes<'a>(raw: &'a [u8], cursor: &mut usize) -> io::Result<&'a [u8]> {
    let length = u16::from_be_bytes(take(raw, cursor, 2)?.try_into().unwrap()) as usize;
    take(raw, cursor, length)
}
fn skip(raw: &[u8], cursor: &mut usize, tag: u8, depth: usize) -> io::Result<()> {
    if depth > 64 {
        return Err(invalid());
    }
    match tag {
        1 => {
            take(raw, cursor, 1)?;
        }
        2 => {
            take(raw, cursor, 2)?;
        }
        3 | 5 => {
            take(raw, cursor, 4)?;
        }
        4 | 6 => {
            take(raw, cursor, 8)?;
        }
        7 | 11 | 12 => {
            let items = count(raw, cursor)?;
            let width = match tag {
                11 => 4,
                12 => 8,
                _ => 1,
            };
            take(raw, cursor, items.checked_mul(width).ok_or_else(invalid)?)?;
        }
        8 => {
            string_bytes(raw, cursor)?;
        }
        9 => {
            let item_tag = byte(raw, cursor)?;
            let items = count(raw, cursor)?;
            if items > MAX_ITEMS {
                return Err(invalid());
            }
            for _ in 0..items {
                skip(raw, cursor, item_tag, depth + 1)?;
            }
        }
        10 => {
            let mut items = 0;
            loop {
                let tag = byte(raw, cursor)?;
                if tag == 0 {
                    break;
                }
                items += 1;
                if items > MAX_ITEMS {
                    return Err(invalid());
                }
                string_bytes(raw, cursor)?;
                skip(raw, cursor, tag, depth + 1)?;
            }
        }
        _ => return Err(invalid()),
    }
    Ok(())
}
fn patch_compound(
    raw: &[u8],
    changes: &BTreeMap<String, NbtValue>,
    depth: usize,
) -> io::Result<Vec<u8>> {
    if depth > 64 {
        return Err(invalid());
    }
    let mut cursor = 0;
    let mut output = Vec::new();
    let mut pending = changes.clone();
    let mut items = 0;
    loop {
        let start = cursor;
        let tag = byte(raw, &mut cursor)?;
        if tag == 0 {
            break;
        }
        items += 1;
        if items > MAX_ITEMS {
            return Err(invalid());
        }
        let name = string_bytes(raw, &mut cursor)?;
        let payload = cursor;
        skip(raw, &mut cursor, tag, depth)?;
        // Java strings use modified UTF-8; only match the ASCII field names we own.
        let change = std::str::from_utf8(name)
            .ok()
            .and_then(|name| pending.remove(name).map(|value| (name, value)));
        if let Some((name, value)) = change {
            if let NbtValue::Compound(children) = &value
                && tag == 10
            {
                output.extend_from_slice(&raw[start..payload]);
                output.extend(patch_compound(&raw[payload..cursor], children, depth + 1)?);
            } else {
                encode(&mut output, name, &value)?;
            }
        } else {
            output.extend_from_slice(&raw[start..cursor]);
        }
    }
    for (name, value) in pending {
        encode(&mut output, &name, &value)?;
    }
    output.push(0);
    Ok(output)
}
fn text(output: &mut Vec<u8>, value: &str) -> io::Result<()> {
    // Settings are ASCII IDs, booleans, numbers, and names from validation.
    let length = u16::try_from(value.len()).map_err(|_| invalid())?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}
fn encode(output: &mut Vec<u8>, name: &str, value: &NbtValue) -> io::Result<()> {
    let tag = match value {
        NbtValue::Byte(_) => 1,
        NbtValue::Int(_) => 3,
        NbtValue::String(_) => 8,
        NbtValue::List(_) => 9,
        NbtValue::Compound(_) => 10,
        _ => return Err(invalid()),
    };
    output.push(tag);
    text(output, name)?;
    match value {
        NbtValue::Byte(value) => output.push(*value as u8),
        NbtValue::Int(value) => output.extend_from_slice(&value.to_be_bytes()),
        NbtValue::String(value) => text(output, value)?,
        NbtValue::List(values) => {
            output.push(8);
            let count = i32::try_from(values.len()).map_err(|_| invalid())?;
            output.extend_from_slice(&count.to_be_bytes());
            for value in values {
                let NbtValue::String(value) = value else {
                    return Err(invalid());
                };
                text(output, value)?;
            }
        }
        NbtValue::Compound(values) => {
            for (name, value) in values {
                encode(output, name, value)?;
            }
            output.push(0);
        }
        _ => return Err(invalid()),
    }
    Ok(())
}
