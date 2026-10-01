use std::collections::BTreeMap;
use std::time::Instant;

use super::MapPlayer;

#[derive(Default)]
pub(super) struct JavaMapQueryController {
    pub pending: Option<JavaMapQuery>,
    pub last_requested: Option<Instant>,
    pub sequence: u64,
}

pub(super) struct JavaMapQuery {
    pub server_id: String,
    pub generation: u64,
    pub started: Instant,
    pub records: BTreeMap<String, JavaMapRecord>,
}

#[derive(Default)]
pub(super) struct JavaMapRecord {
    pub pos: Option<[f64; 3]>,
    pub rotation: Option<[f64; 2]>,
    pub dimension: Option<String>,
}

impl JavaMapRecord {
    pub fn into_player(self, name: String) -> Option<MapPlayer> {
        let pos = self.pos?;
        let rotation = self.rotation?;
        Some(MapPlayer {
            id: name.clone(),
            name,
            dimension: self.dimension?,
            x: pos[0],
            y: pos[1],
            z: pos[2],
            yaw: rotation[0],
            pitch: rotation[1],
        })
    }
}

pub(super) enum EntityField {
    Position([f64; 3]),
    Rotation([f64; 2]),
    Dimension(String),
}

fn message(line: &str) -> &str {
    let text = line.rsplit_once("]: ").map_or(line, |(_, tail)| tail);
    text.strip_prefix("System chat: ").unwrap_or(text)
}

fn numbers<const N: usize>(value: &str) -> Option<[f64; N]> {
    let body = value.strip_prefix('[')?.strip_suffix(']')?;
    let parts = body.split(',').collect::<Vec<_>>();
    if parts.len() != N {
        return None;
    }
    let mut result = [0.0; N];
    for (index, part) in parts.into_iter().enumerate() {
        let part = part.trim().trim_end_matches(['d', 'D', 'f', 'F']);
        let value = part.parse::<f64>().ok()?;
        if !value.is_finite() {
            return None;
        }
        result[index] = value;
    }
    Some(result)
}

pub(super) fn entity_field(line: &str) -> Option<(String, EntityField)> {
    let text = message(line);
    let (name, value) =
        if let Some((name, value)) = text.split_once(" has the following entity data: ") {
            (name, value)
        } else if let Some((name, value)) = text
            .strip_prefix("Data for entity ")
            .and_then(|text| text.split_once(" is: "))
        {
            (name, value)
        } else if let Some((name, value)) = text
            .strip_prefix("The data of ")
            .and_then(|text| text.split_once(" is: "))
        {
            (name, value)
        } else {
            return None;
        };
    let name = name.trim();
    if name.is_empty() || name.len() > 128 {
        return None;
    }
    let value = value.trim();
    let field = if let Some(pos) = numbers::<3>(value) {
        EntityField::Position(pos)
    } else if let Some(rotation) = numbers::<2>(value) {
        EntityField::Rotation(rotation)
    } else {
        let dimension = value.trim_matches('"');
        if dimension.len() > 128 || !dimension.contains(':') || dimension.contains(' ') {
            return None;
        }
        EntityField::Dimension(dimension.to_string())
    };
    Some((name.to_string(), field))
}

pub(super) fn online_names(line: &str) -> Option<Vec<String>> {
    let text = message(line);
    let (count, names) = text.split_once(" players online:")?;
    let count = count
        .strip_prefix("There are ")?
        .split_whitespace()
        .next()?
        .parse::<usize>()
        .ok()?;
    if count > 100 {
        return None;
    }
    if count == 0 {
        return Some(Vec::new());
    }
    let names = names
        .trim()
        .split(',')
        .map(|name| name.trim().to_string())
        .collect::<Vec<_>>();
    (names.len() == count && names.iter().all(|name| !name.is_empty())).then_some(names)
}
