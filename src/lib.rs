//! ym2612-rust: a Rust YM2612 / YM3438 (OPN2) core transcribed from, and proven
//! equivalent to, [ym2612-python](https://github.com/alewman/ym2612-python).
//!
//! The chip is [`YM2612`]; its two variants are [`YM2612_DISCRETE`] (the NMOS part,
//! with the ladder) and [`YM3438_ASIC`]. The contract -- the pins, the time unit, the
//! outputs -- is the reference's `docs/contract.md`. [`conformance`] runs the
//! reference's manifests and writes its trace format.

pub mod chip;
pub mod conformance;

pub use chip::{exp_rom, log_sin_rom, FMVariant, YM2612, YM2612_DISCRETE, YM3438_ASIC};
