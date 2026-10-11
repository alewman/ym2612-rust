//! The reference's thirteen example manifests, replayed against copies of its
//! reference traces (`tests/data/conformance/`, from ym2612-python's
//! `examples/conformance/` at the pinned commit) without Python. `scripts/rung2.sh`
//! makes the same check through the reference's own diff.

use std::fs;
use std::path::Path;

use serde_json::json;
use ym2612_rust::conformance::{load_manifest, manifest_from_value, trace_manifest};
use ym2612_rust::{YM2612, YM2612_DISCRETE, YM3438_ASIC};

#[test]
fn every_example_trace_is_the_reference_trace() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/conformance");
    let mut checked = 0;
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    for path in paths {
        let manifest = load_manifest(path.to_str().unwrap()).unwrap();
        let mut trace = Vec::new();
        trace_manifest(&manifest, &mut trace).unwrap();
        let reference = fs::read_to_string(path.with_extension("trace")).unwrap();
        let ours = String::from_utf8(trace).unwrap();
        if let Some((line, (a, b))) = reference
            .lines()
            .zip(ours.lines())
            .enumerate()
            .find(|(_, (a, b))| a != b)
        {
            panic!(
                "{}: line {}: reference {a:?}, ours {b:?}",
                manifest.name,
                line + 1
            );
        }
        assert_eq!(
            reference.lines().count(),
            ours.lines().count(),
            "{}",
            manifest.name
        );
        checked += 1;
    }
    assert_eq!(checked, 13);
}

#[test]
fn bad_manifests_are_refused_as_the_reference_refuses_them() {
    let good = json!({"version": 1, "name": "x", "variant": "ym2612", "ticks": 100, "events": []});
    assert!(manifest_from_value(&good).is_ok());
    let cases = [
        json!([]),
        json!({"version": 2, "name": "x", "variant": "ym2612", "ticks": 100}),
        json!({"version": 1, "name": "x", "variant": "opl3", "ticks": 100}),
        json!({"version": 1, "name": "x", "variant": "ym2612", "ticks": 100, "extra": 1}),
        json!({"version": 1, "name": "x", "variant": "ym2612", "ticks": 100,
               "events": [{"t": 5, "kind": "poke"}]}),
        json!({"version": 1, "name": "x", "variant": "ym2612", "ticks": 100,
               "events": [{"t": 5, "kind": "write", "port": 4}]}),
        json!({"version": 1, "name": "x", "variant": "ym2612", "ticks": 100,
               "events": [{"t": 5, "kind": "write", "data": 256}]}),
        json!({"version": 1, "name": "x", "variant": "ym2612", "ticks": 100,
               "events": [{"t": 5, "kind": "read", "colour": 1}]}),
        json!({"version": 1, "name": "x", "variant": "ym2612", "ticks": 100,
               "events": [{"t": 50, "kind": "read"}, {"t": 5, "kind": "read"}]}),
        json!({"version": 1, "name": "x", "variant": "ym2612", "ticks": 100,
               "events": [{"t": 500, "kind": "read"}]}),
    ];
    for case in cases {
        assert!(manifest_from_value(&case).is_err(), "{case}");
    }
}

#[test]
fn the_readme_example_sounds() {
    // The reference README's minimal host: one operator of channel 1, keyed on.
    let mut chip = YM2612::new(YM2612_DISCRETE, true);
    let mut tick = 0i64;
    let mut write = |chip: &mut YM2612, register: u8, value: u8| {
        for (port, byte) in [(0, register), (1, value)] {
            chip.run(tick);
            chip.write(port, byte);
            tick += 24 * 42;
        }
    };
    for (register, value) in [
        (0x30, 0x01),
        (0x40, 0x00),
        (0x50, 0x1F),
        (0x80, 0x0F),
        (0xB0, 0x07),
        (0xB4, 0xC0),
        (0xA4, 0x22),
        (0xA0, 0x69),
        (0x28, 0x10),
    ] {
        write(&mut chip, register, value);
    }
    chip.run(tick + 53_693_175 / 10);
    let samples = chip.take_samples();
    assert_eq!(samples.len() as i64, (tick + 53_693_175 / 10) / (24 * 42));
    let loud = samples.iter().map(|&(l, _)| l.abs()).max().unwrap();
    // One channel at full level: its value on one cycle of four, x3, plus the signs.
    assert!(loud > 500, "{loud}");
}

#[test]
fn only_the_asic_answers_on_ports_1_to_3() {
    for (variant, answers) in [(YM2612_DISCRETE, false), (YM3438_ASIC, true)] {
        let mut chip = YM2612::new(variant, true);
        chip.run(42 * 10);
        chip.write(1, 0); // a data write sets BUSY
        chip.run(42 * 12);
        assert_eq!(chip.read(1) & 0x80 != 0, answers, "{}", variant.name);
        assert_eq!(chip.read(0) & 0x80, 0x80, "{}", variant.name);
    }
}
