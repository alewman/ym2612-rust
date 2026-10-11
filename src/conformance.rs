//! The conformance kit's Rust half: read a manifest, run it, write the trace.
//!
//! A transcription of ym2612-python's `src/ym2612_python/conformance.py`
//! (`trace_manifest`, `manifest_from_dict`); the formats are its
//! `docs/conformance.md`. The diff stays the reference's: a trace written here is
//! judged by `python -m ym2612_python.conformance diff manifest.json -`.

use std::io::{self, Write};

use serde_json::{Map, Value};

use crate::chip::{
    FMVariant, IC_HOLD_PAST_RELEASE, MCLK_PER_CYCLE, YM2612, YM2612_DISCRETE, YM3438_ASIC,
};

pub const MANIFEST_SCHEMA_VERSION: i64 = 1;
/// The first line of every trace, so a trace of another version is refused, not misread.
pub const TRACE_HEADER: &str = "# ym2612-trace 1";

/// The manifest's variant names.
pub fn variant(name: &str) -> Option<FMVariant> {
    match name {
        "ym2612" => Some(YM2612_DISCRETE),
        "ym3438" => Some(YM3438_ASIC),
        _ => None,
    }
}

/// One thing the host does to the chip, at tick `t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `write(port, data)`.
    Write { port: u8, data: u8 },
    /// `read(port)`, whose value is traced.
    Read { port: u8 },
    /// `reset_line(held, Some(t), lag)`.
    Ic { held: bool, lag: i64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub t: i64,
    pub kind: Kind,
}

/// A run: the variant, the events in tick order, and how many ticks to run.
#[derive(Clone, Debug)]
pub struct Manifest {
    pub name: String,
    pub variant: String,
    pub events: Vec<Event>,
    pub ticks: i64,
    pub enabled: bool,
    pub control_delay: i32,
}

fn int(value: &Value, what: &str) -> Result<i64, String> {
    value
        .as_i64()
        .ok_or_else(|| format!("{what} is not an integer"))
}

fn event_from(index: usize, value: &Value) -> Result<Event, String> {
    let item: &Map<String, Value> = value
        .as_object()
        .ok_or_else(|| format!("event {index} is not an object"))?;
    let mut extra: Vec<&String> = item
        .keys()
        .filter(|k| !["t", "kind", "port", "data", "held", "lag"].contains(&k.as_str()))
        .collect();
    if !extra.is_empty() {
        extra.sort();
        return Err(format!("event {index}: unknown keys {extra:?}"));
    }
    let get = |key: &str, default: i64| -> Result<i64, String> {
        item.get(key)
            .map_or(Ok(default), |v| int(v, &format!("event {index} {key}")))
    };
    let t = int(
        item.get("t")
            .ok_or_else(|| format!("event {index} has no t"))?,
        &format!("event {index} t"),
    )?;
    let port = get("port", 0)?;
    let data = get("data", 0)?;
    let lag = get("lag", 3)?;
    let held = match item.get("held") {
        None => false,
        Some(v) => v
            .as_bool()
            .ok_or_else(|| format!("event {index} held is not a boolean"))?,
    };
    if t < 0 || !(0..=3).contains(&port) || !(0..=0xFF).contains(&data) {
        return Err(format!("event {index} out of range"));
    }
    let kind = match item.get("kind").and_then(Value::as_str) {
        Some("write") => Kind::Write {
            port: port as u8,
            data: data as u8,
        },
        Some("read") => Kind::Read { port: port as u8 },
        Some("ic") => Kind::Ic { held, lag },
        other => return Err(format!("unknown event kind {other:?}")),
    };
    Ok(Event { t, kind })
}

/// A manifest from its JSON form (ym2612-python `docs/conformance.md`), checked as
/// strictly as the reference's `manifest_from_dict`.
pub fn manifest_from_value(value: &Value) -> Result<Manifest, String> {
    let object = value.as_object().ok_or("a manifest is a JSON object")?;
    let allowed = [
        "version",
        "name",
        "variant",
        "enabled",
        "control_delay",
        "events",
        "ticks",
    ];
    let mut unknown: Vec<&String> = object
        .keys()
        .filter(|k| !allowed.contains(&k.as_str()))
        .collect();
    if !unknown.is_empty() {
        unknown.sort();
        return Err(format!("unknown manifest keys: {unknown:?}"));
    }
    if object.get("version").and_then(Value::as_i64) != Some(MANIFEST_SCHEMA_VERSION) {
        return Err(format!(
            "unsupported manifest version {:?}",
            object.get("version")
        ));
    }
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .ok_or("manifest has no name")?;
    let variant_name = object
        .get("variant")
        .and_then(Value::as_str)
        .ok_or("manifest has no variant")?;
    if variant(variant_name).is_none() {
        return Err(format!(
            "unknown variant {variant_name:?} (ym2612 or ym3438)"
        ));
    }
    let ticks = int(object.get("ticks").ok_or("manifest has no ticks")?, "ticks")?;
    let enabled = match object.get("enabled") {
        None => true,
        Some(v) => v.as_bool().ok_or("enabled is not a boolean")?,
    };
    let control_delay = match object.get("control_delay") {
        None => 0,
        Some(v) => int(v, "control_delay")? as i32,
    };
    let mut events = Vec::new();
    if let Some(list) = object.get("events") {
        let list = list.as_array().ok_or("events is not a list")?;
        for (index, item) in list.iter().enumerate() {
            events.push(event_from(index, item)?);
        }
    }
    if events.windows(2).any(|w| w[1].t < w[0].t) {
        return Err("events must be in tick order".into());
    }
    if events.last().is_some_and(|e| e.t > ticks) {
        return Err("an event falls after the end of the run".into());
    }
    Ok(Manifest {
        name: name.to_string(),
        variant: variant_name.to_string(),
        events,
        ticks,
        enabled,
        control_delay,
    })
}

pub fn load_manifest(path: &str) -> Result<Manifest, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let value: Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
    manifest_from_value(&value)
}

/// The status byte as the chip holds it, without a read's side effect.
fn status(chip: &YM2612) -> i32 {
    (chip.busy << 7) | (chip.timer_b_overflow_flag << 1) | chip.timer_a_overflow_flag
}

fn advance(chip: &mut YM2612, t: i64, out: &mut impl Write) -> io::Result<()> {
    while chip.mclk + MCLK_PER_CYCLE <= t {
        let held = chip.in_reset || (chip.ic_hold() != 0 && IC_HOLD_PAST_RELEASE);
        chip.run(chip.mclk + MCLK_PER_CYCLE);
        if held {
            writeln!(out, "H {}", chip.cycles)?;
        } else {
            // `cycles` has already moved on to the next slot; the line names the slot
            // the cycle served.
            writeln!(
                out,
                "C {} {} {} {} {}",
                (chip.cycles + 23) % 24,
                chip.mol,
                chip.mor,
                status(chip),
                chip.irq() as i32
            )?;
        }
    }
    Ok(())
}

/// Run `manifest` and write its trace: `C slot mol mor status irq` after every
/// internal cycle the chip runs, `H slot` after every cycle it is held instead, and
/// `R port value` for every read (the reference's `trace_manifest`).
pub fn trace_manifest(manifest: &Manifest, out: &mut impl Write) -> io::Result<()> {
    let mut chip = YM2612::new(
        variant(&manifest.variant).expect("checked when loaded"),
        manifest.enabled,
    );
    chip.control_delay = manifest.control_delay;
    writeln!(out, "{TRACE_HEADER}")?;
    writeln!(out, "# {} {}", manifest.name, manifest.variant)?;
    for event in &manifest.events {
        advance(&mut chip, event.t, out)?;
        match event.kind {
            Kind::Write { port, data } => chip.write(port, data),
            Kind::Read { port } => {
                let value = chip.read(port);
                writeln!(out, "R {port} {value}")?;
            }
            Kind::Ic { held, lag } => chip.reset_line(held, Some(event.t), lag),
        }
    }
    advance(&mut chip, manifest.ticks, out)?;
    writeln!(
        out,
        "# end {} cycles {} writes",
        chip.cycles_run, chip.writes
    )
}
