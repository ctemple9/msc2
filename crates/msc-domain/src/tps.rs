//! Pure, side-effect-free parsing of server TPS console lines.
//!
//! Ported from `TpsLineParser.swift`. Three console-reply shapes are
//! recognized:
//!   - Paper family: "TPS from last 1m, 5m, 15m: 20.0, 20.0, 20.0" — three
//!     rolling averages.
//!   - Legacy Forge / NeoForge (MC <=1.20) `forge tps` / `neoforge tps`:
//!     "Overall: Mean tick time: 3.456 ms. Mean TPS: 20.000" — one overall
//!     value, no 1m/5m/15m breakdown, so t5/t15 are `None`.
//!   - Modern NeoForge (MC >=1.21) `neoforge tps`: "Overall: 20.000 TPS
//!     (0.354 ms/tick)" — reworded at 1.21; same single-value shape.
//!   - Vanilla `/tick query` (MC >=1.20.3, for Vanilla/Fabric/Quilt, which
//!     have no loader TPS command): "Average time per tick: 0.7ms (Target:
//!     50.0ms)". No TPS figure in the output, so it's derived from the mean
//!     tick time as `min(20, 1000 / mspt)`. Single value, so t5/t15 are
//!     `None`.
//!
//! Plus spark's `/spark tps` header/values, used by Fabric/Quilt/Vanilla
//! servers running the spark mod.

use regex::Regex;

/// A parsed TPS sample. `t5`/`t15` are `None` for single-value flavors
/// (Forge) so downstream UI renders one number instead of stale trio values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub t1: f64,
    pub t5: Option<f64>,
    pub t15: Option<f64>,
}

/// Parse an already-sanitized console line. Paper format is tried first,
/// then legacy Forge, then modern NeoForge, then the vanilla `/tick query`
/// line; returns `None` when the line is none of them.
pub fn parse(clean: &str) -> Option<Sample> {
    parse_paper(clean)
        .or_else(|| parse_forge(clean))
        .or_else(|| parse_neoforge(clean))
        .or_else(|| parse_vanilla_tick(clean))
}

pub fn parse_paper(clean: &str) -> Option<Sample> {
    if !clean.contains("TPS from last 1m, 5m, 15m:") {
        return None;
    }
    let colon_index = clean.rfind(':')?;
    let numbers_part = clean[colon_index + 1..].trim();
    let parts: Vec<&str> = numbers_part.split(',').map(str::trim).collect();
    if parts.len() < 3 {
        return None;
    }
    let t1 = parts[0].parse::<f64>().ok()?;
    let t5 = parts[1].parse::<f64>().ok()?;
    let t15 = parts[2].parse::<f64>().ok()?;
    Some(Sample {
        t1,
        t5: Some(t5),
        t15: Some(t15),
    })
}

pub fn parse_forge(clean: &str) -> Option<Sample> {
    if !clean.contains("Mean tick time") {
        return None;
    }
    let regex =
        Regex::new(r"(?i)Overall:\s*Mean tick time:\s*[0-9.]+\s*ms\.?\s*Mean TPS:\s*([0-9.]+)")
            .expect("static regex");
    let captures = regex.captures(clean)?;
    let tps = captures.get(1)?.as_str().parse::<f64>().ok()?;
    // Forge reports one overall rolling mean, not Paper's 1m/5m/15m trio.
    Some(Sample {
        t1: tps,
        t5: None,
        t15: None,
    })
}

/// Modern NeoForge (MC 1.21+) reworded its `neoforge tps` reply to
/// "Overall: 20.000 TPS (0.354 ms/tick)" — no "Mean tick time"/"Mean TPS"
/// text, so `parse_forge` can't see it. Anchoring on "Overall:" avoids
/// matching the per-dimension lines ("minecraft:overworld: X TPS (...)")
pub fn parse_neoforge(clean: &str) -> Option<Sample> {
    if !clean.contains("Overall:") || !clean.contains("TPS") {
        return None;
    }
    let regex = Regex::new(r"(?i)Overall:\s*([0-9.]+)\s*TPS\b").expect("static regex");
    let captures = regex.captures(clean)?;
    let tps = captures.get(1)?.as_str().parse::<f64>().ok()?;
    // Single overall value, same shape as legacy Forge.
    Some(Sample {
        t1: tps,
        t5: None,
        t15: None,
    })
}

/// Vanilla `/tick query` (MC 1.20.3+) reports mean tick time, not TPS:
/// "Average time per tick: 0.7ms (Target: 50.0ms)". We derive TPS as
/// `min(20, 1000 / mspt)` — 20 is the vanilla target rate, and a server
/// that beats the tick budget still caps at 20.
pub fn parse_vanilla_tick(clean: &str) -> Option<Sample> {
    if !clean.contains("Average time per tick") {
        return None;
    }
    let regex = Regex::new(r"(?i)Average time per tick:\s*([0-9.]+)\s*ms").expect("static regex");
    let captures = regex.captures(clean)?;
    let mspt = captures.get(1)?.as_str().parse::<f64>().ok()?;
    if mspt <= 0.0 {
        return None;
    }
    let tps = (1000.0 / mspt).min(20.0);
    Some(Sample {
        t1: tps,
        t5: None,
        t15: None,
    })
}

// --- spark `/spark tps` (Fabric/Quilt/Vanilla with the spark mod) ---

/// True for spark's TPS section header: "TPS from last 5s, 10s, 1m, 5m,
/// 15m:". spark prints the five values on the FOLLOWING line, so the caller
/// arms on the header and parses the next line with `parse_spark_values`.
pub fn is_spark_tps_header(clean: &str) -> bool {
    clean.contains("TPS from last 5s, 10s, 1m, 5m, 15m")
}

/// Parses spark's TPS values line ("20.0, 20.0, 20.0, 20.0, 20.0" —
/// possibly with colour codes or a leading "*"), extracting the five
/// decimals for the 5s, 10s, 1m, 5m, 15m windows. Maps 1m/5m/15m onto
/// t1/t5/t15 so the UI renders the same trio as Paper. Requires decimals,
/// so a log timestamp ("[11:48:55]") can't be mistaken for a value. Returns
/// `None` if fewer than five values are present.
pub fn parse_spark_values(clean: &str) -> Option<Sample> {
    let regex = Regex::new(r"[0-9]+\.[0-9]+").expect("static regex");
    let nums: Vec<f64> = regex
        .find_iter(clean)
        .filter_map(|m| m.as_str().parse::<f64>().ok())
        .collect();
    if nums.len() < 5 {
        return None;
    }
    // spark order: 5s, 10s, 1m, 5m, 15m.
    Some(Sample {
        t1: nums[2],
        t5: Some(nums[3]),
        t15: Some(nums[4]),
    })
}
