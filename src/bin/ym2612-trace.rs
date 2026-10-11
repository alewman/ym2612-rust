//! `ym2612-trace <manifest.json> [--out FILE]`: run a conformance manifest and write
//! its trace, for `python -m ym2612_python.conformance diff manifest.json -`.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

use ym2612_rust::conformance::{load_manifest, trace_manifest};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (path, out) = match args.as_slice() {
        [path] => (path, None),
        [path, flag, out] if flag == "--out" => (path, Some(out)),
        _ => {
            eprintln!("usage: ym2612-trace <manifest.json> [--out FILE]");
            return ExitCode::from(2);
        }
    };
    let manifest = match load_manifest(path) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };
    let result = match out {
        Some(out) => File::create(out).and_then(|file| {
            let mut writer = BufWriter::with_capacity(1 << 20, file);
            trace_manifest(&manifest, &mut writer)?;
            writer.flush()
        }),
        None => {
            let stdout = io::stdout();
            let mut writer = BufWriter::with_capacity(1 << 20, stdout.lock());
            trace_manifest(&manifest, &mut writer).and_then(|()| writer.flush())
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        // The diff stops reading at the first difference; a closed pipe is its answer.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
