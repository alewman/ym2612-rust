# ym2612-rust

A Rust Yamaha **YM2612** (OPN2) and **YM3438** core transcribed from
[ym2612-python](https://github.com/alewman/ym2612-python) and proven equivalent to
it on every internal cycle: whether the chip ran or was held, the slot it served,
both DAC pins, the status byte, /IRQ, and the value of every read. ym2612-python is
in turn equal to **Nuked-OPN2**, a transcription of the YM3438's die, on every cycle
of the same 18 streams (663,015,150 values, 0 different), so the two results compose.

`src/chip.rs` is `chip.py` function for function, in the same order, with the same
names and the same citations of the die models. `src/conformance.rs` is the
reference's manifest reader and trace writer. Nothing is redesigned: where Rust
forced a difference, the module header says what it is. Speed is the job of a
`-fast` fork, as z80-rust-fast and m68000-rust-fast are for their CPUs.

## Certification

Pinned oracles:

| What | Version |
| --- | --- |
| ym2612-python (reference core and conformance kit) | `2c2ab235aed619b395ba152111cd79a6193390b3` |
| Manifest and trace formats | manifest version 1, `# ym2612-trace 1` |

The ladder is the reference's own (its `docs/conformance.md`, "Certification ladder
for a port"). Every rung was reproduced with the script named (`scripts/`) on 2026-10-11:
Linux x86_64, rustc 1.93.1, the reference on CPython 3.14.4.

| Rung | What | Result | Script |
| --- | --- | --- | --- |
| 1 | The two ROM tables, packed as 256 little-endian u16, against the SHA-256 the reference's `docs/contract.md` pins | `log_sin` `a099b16d…`, `exp` `a0ca448c…`: both pinned | `rung1.sh` |
| 2 | Every example manifest in the reference's `examples/conformance/` | 13 of 13 identical | `rung2.sh` |
| 3 | The seven synthetic streams of the reference's `scripts/run_fm_streams.py` (two random walks of 4,000 writes, channel 3's special mode and CSM, SSG-EG's eight shapes, the LFO at 8 speeds × 8 PMS × 4 AMS, both timers, the DAC), both variants | 14 of 14 identical, 6,124,839 cycles each variant | `rung3.sh` |
| 4 | The register streams megadrive-python's board fed its chip: Nemesis's six test ROMs and five cartridges (Sonic the Hedgehog, Streets of Rage 2, Gunstar Heroes, Castlevania Bloodlines, Phantasy Star IV), both variants | 22 of 22 identical, 104,377,686 cycles each variant (3 min 47 s) | `rung4.sh` |
| 5 | 128 seeded random manifests: six sounding voices, then 116,335 port writes at any tick, 1,899 reads on every port, 91 /IC pulses at both lags with the voices reprogrammed after each, 14 on the quiet path, 58 with the $27 delay, 20 probes of the driven value's decay; both variants | 128 of 128 identical, 18.5 M cycles | `rung5.sh` |

Rungs 3 and 4 together are every cycle of the reference's Nuked-OPN2 record
(110,502,525 per variant), as a trace identical to the reference's here is the run
the reference compared with the die model. Each stream's manifest writes at
`cycle × 42` ticks, which is where the reference's `replay` writes.

**A gate that cannot fail proves nothing.** `scripts/seeded_faults.py` breaks this
core in twelve places, one at a time, in a copy of the repository: the ladder's +1,
BUSY's length, detune's sign, feedback, the slowest LFO reload, SSG-EG's rate, timer
B's period, /IC's pause carry, the DAC's sign bit, the $27 delay, key-on's operator
order, and the status decay. The thirteen examples catch nine. Rungs 3 and 5 catch
the other three:

- the LFO reload: 6 of 14 synthetic manifests differ, and 11 of the 128 random ones;
- key-on's operator order: 4 synthetic and 67 random;
- the status decay: only the random decay probes, 7 of them.

The reference records the decay gap in its own examples. The other two are gaps in
its examples that this port found (see "Notes for the reference").

CI (`.github/workflows/ci.yml`) runs `cargo fmt --check`, `cargo clippy -D
warnings`, `cargo test` and rung 1 on main and pull requests. `cargo test` replays
the thirteen example manifests against copies of the reference traces, without
Python. Rungs 2-5 need the reference checked out, which is a private repository, so
they run locally.

### Reproducing

```text
scripts/fetch_ym2612python.sh     # the reference at the pin into external/, with a venv
scripts/rung1.sh
scripts/rung2.sh
scripts/rung3.sh                  # writes the manifests into external/manifests/synthetic
scripts/rung4.sh [streams dir]    # needs the reference's exported board streams, below
scripts/rung5.sh [count] [seed]   # 128 from seed 1 by default
python3 scripts/seeded_faults.py  # after rungs 3 and 5, which write the manifests it reuses
scripts/bench.sh
```

Each rung writes this core's trace and pipes it to the reference's own
`python -m ym2612_python.conformance diff manifest.json -`, 16 at a time
(`scripts/diff_all.sh`). Rung 4's streams come from ROMs, so neither repository
commits them, nor the manifests made from them (`external/`, ignored). The reference
exports them with
`python scripts/export_board_streams.py --board ../megadrive-python --roms --games`.

## Using the core

```rust
use ym2612_rust::{YM2612, YM2612_DISCRETE};

let mut chip = YM2612::new(YM2612_DISCRETE, true);
let mut tick = 0i64;
let mut write = |chip: &mut YM2612, register: u8, value: u8| {
    for (port, byte) in [(0, register), (1, value)] {
        chip.run(tick); // bring the chip to the moment of the access
        chip.write(port, byte);
        tick += 24 * 42; // leave the chip a sample between writes
    }
};
for (register, value) in [(0x30, 0x01), (0x40, 0x00), (0x50, 0x1F), (0x80, 0x0F),
                          (0xB0, 0x07), (0xB4, 0xC0), (0xA4, 0x22), (0xA0, 0x69),
                          (0x28, 0x10)] {
    write(&mut chip, register, value);
}
chip.run(tick + 53_693_175 / 10); // a tenth of a second (ticks are the MD's MCLK)
let samples = chip.take_samples(); // (left, right) at 53,267 Hz
```

The contract is the reference's `docs/contract.md`, in Rust form:

- `run(t)` takes **ticks**, seven to an input clock, which on the Mega Drive is the
  master clock; it completes every internal cycle that ends by `t`.
- `write(port, value)` and `read(port)` are the two ports.
- `reset_line(held, Some(t), lag)` is /IC (`lag` is the board's:
  `chip::IC_SLOT_LAG` for the reset button, `chip::IC_WRITE_SLOT_LAG` for a write
  to $A11200), and `irq()` is /IRQ.
- `mol` and `mor` are the DAC pins after the last cycle, and `take_samples()`
  returns their sums, one pair per 24 cycles.
- `enabled` (the constructor's second argument) is the generator switch, and
  `control_delay` is register $27's timer delay.

The pipeline's state is public so a debugger can look at it. It is not a contract.

## Binaries

- `ym2612-trace <manifest> [--out FILE]`: run a conformance manifest and write its
  trace (rungs 2-5).
- `ym2612-tables log_sin|exp`: a table as rung 1 hashes it.
- `ym2612-bench <manifest>`: the reference's benchmark, through `run()`.

## Speed

`scripts/bench.sh` runs the reference's benchmark: the first two seconds of the seed-1
random walk (40,000 writes) through `run()`, as a board drives the chip, best of 20.
Measured on one pinned core of an i9-13900K that other jobs were sharing (load average
12 on 32 threads), 2026-10-11:

| | Generator on | Quiet path (`enabled = false`) |
| --- | ---: | ---: |
| ym2612-rust | 18.4 M cycles/s, **14.4× real time** | 75.9 M cycles/s, 59.3× |
| ym2612-python, CPython 3.14.4 (its record) | 133 k cycles/s, 0.104× | 541 k cycles/s, 0.423× |
| ym2612-python, PyPy 7.3.20 (its record) | 35.6 k cycles/s, 0.028× | 161 k cycles/s, 0.126× |

Real time is 1,278,409 internal cycles a second (NTSC), so this core is 139 times
the reference on CPython. The library also builds for `wasm32-unknown-unknown`.
Running the ladder inside WebAssembly, where the tables come from Rust's own `libm`,
is the `-fast` fork's check, as it was for m68000-rust-fast.

## Notes for the reference

The seeded faults found two rules that none of ym2612-python's thirteen examples
reach: the slowest LFO speed's counter reload (108), and which of operators 2 and 3
a key-on bit reaches. Its synthetic streams catch both, so its Nuked-OPN2 record is
unaffected; only a port that stops at rung 2 would miss them. An example for each
would close the gap.

## Where the brief is

[docs/handoff-brief.md](docs/handoff-brief.md) is the request this repository was
built from and the plan read from it, and [docs/build-record.md](docs/build-record.md)
is what happened.

## License

MIT, like the reference. Nuked-OPN2 (LGPL-2.1) and YM2608-LLE (GPL-2.0) are cited
through the reference and never read into this repository.
