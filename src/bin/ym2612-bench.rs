//! `ym2612-bench <manifest.json> [--seconds S] [--repeat N]`: internal cycles per
//! second and the fraction of real time, measured as the reference's
//! `benchmarks/bench.py` measures it.
//!
//! The manifest's writes (`scripts/bench.sh` makes the reference's stream, the seed-1
//! random walk of 40,000 writes) are driven through `run()` as a board drives the
//! chip, for the first `--seconds` of emulated time, with the generator on and then
//! off (the quiet path). Real time is 53,693,175 / 42 = 1,278,409 internal cycles a
//! second (NTSC). Reports the best of `--repeat` runs, since the machine is shared.

use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

use ym2612_rust::chip::MCLK_PER_CYCLE;
use ym2612_rust::conformance::{load_manifest, Kind, Manifest};
use ym2612_rust::{YM2612, YM2612_DISCRETE};

const REAL_TIME: f64 = 53_693_175.0 / MCLK_PER_CYCLE as f64;

fn run(manifest: &Manifest, cycles: i64, enabled: bool) -> f64 {
    let mut chip = YM2612::new(YM2612_DISCRETE, enabled);
    let started = Instant::now();
    for event in &manifest.events {
        if event.t >= cycles * MCLK_PER_CYCLE {
            break;
        }
        if let Kind::Write { port, data } = event.kind {
            chip.run(event.t);
            chip.write(port, data);
        }
    }
    chip.run(cycles * MCLK_PER_CYCLE);
    black_box(chip.take_samples());
    started.elapsed().as_secs_f64()
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(path) = args.first() else {
        eprintln!("usage: ym2612-bench <manifest.json> [--seconds S] [--repeat N]");
        return ExitCode::from(2);
    };
    let option = |name: &str, default: f64| -> f64 {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let seconds = option("--seconds", 2.0);
    let repeat = option("--repeat", 10.0) as usize;
    let manifest = match load_manifest(path) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };
    let cycles = (REAL_TIME * seconds) as i64;
    println!("ym2612-rust, {cycles} cycles, best of {repeat}");
    for (label, enabled) in [("generator on", true), ("quiet path", false)] {
        let best = (0..repeat)
            .map(|_| run(&manifest, cycles, enabled))
            .fold(f64::INFINITY, f64::min);
        let rate = cycles as f64 / best;
        println!(
            "  {label:13} {rate:>14.0} cycles/s  {:8.2}x real time  ({best:.3} s)",
            rate / REAL_TIME
        );
    }
    ExitCode::SUCCESS
}
