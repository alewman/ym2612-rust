# The request and the plan

## The request

2026-10-11, after megadrive-python adopted ym2612-python (its PR #3): "go ahead with
ym2612-rust".

The context, from the owner earlier the same day: megadrive-python is frozen at
efcc6bb and is now used to test the ROM cartridges being built, in preference to
BlastEm, but its Python FM chip is far too slow to play (0.104x real time on CPython
3.14, 0.028x on PyPy). The point of the Rust forms -- this transcription, then a
`-fast` fork, then a WebAssembly build -- is to see the emulator played. The FM chip
is the board's dominant cost, so it goes first.

ym2612-python's brief (`/data/emu/ym2612-python-HANDOFF.md`, "Later phases") names
this step: "`ym2612-rust`, transcribed and proven against the kit; `ym2612-rust-fast`;
a WebAssembly check of the kind m68000-rust-fast has (`scripts/wasm-check.sh`)."

## The plan, read from the family's shape

The shape is z80-rust's and m68000-rust's: a transcription of the reference, module
for module and function for function, with the reference's names, comments and
citations, proven equivalent through the reference's conformance kit, and nothing
redesigned. Speed is the `-fast` fork's job, measured against this one.

1. `src/chip.rs` from `chip.py`, `src/conformance.rs` from `conformance.py` (the
   manifest reader and the trace writer; the diff stays the reference's).
2. The ladder is the reference's own (`docs/conformance.md`, "Certification ladder for
   a port"):

   | Rung | What |
   | --- | --- |
   | 1 | The two tables' SHA-256 equal `docs/contract.md`'s |
   | 2 | Every example in `examples/conformance/` diffs identical |
   | 3 | The seven synthetic streams as manifests, both variants |
   | 4 | The exported board streams (Nemesis's six test ROMs, five cartridges) |
   | 5 | Seeded random manifests with /IC and reads, both variants |

3. A gate that cannot fail proves nothing: seeded faults in this core must fail the
   rungs.
4. Speed measured as the reference's `benchmarks/bench.py` measures it.
5. Constraints: never copy GPL/LGPL code (Nuked-OPN2 and YM2608-LLE are cited through
   the reference, never read into this one); never commit anything derived from ROMs
   (the board streams' manifests stay in `external/`, ignored); a private GitHub
   repository and a push only when the owner asks.
