//! `ym2612-tables`: the SHA-256 input of the chip's two ROM tables, for rung 1.
//!
//! `ym2612-tables log_sin` or `ym2612-tables exp` writes that table packed as 256
//! little-endian unsigned 16-bit integers, the form whose SHA-256 ym2612-python's
//! `docs/contract.md` pins; `scripts/rung1.sh` hashes it.

use std::io::{self, Write};
use std::process::ExitCode;

use ym2612_rust::{exp_rom, log_sin_rom};

fn main() -> ExitCode {
    let table = match std::env::args().nth(1).as_deref() {
        Some("log_sin") => log_sin_rom(),
        Some("exp") => exp_rom(),
        _ => {
            eprintln!("usage: ym2612-tables log_sin|exp");
            return ExitCode::from(2);
        }
    };
    let bytes: Vec<u8> = table.iter().flat_map(|v| v.to_le_bytes()).collect();
    match io::stdout().lock().write_all(&bytes) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
