#!/usr/bin/env python
"""Write the reference's register streams as conformance manifests, both variants.

    python scripts/stream_manifests.py --reference external/ym2612-python --synthetic --out DIR
    python scripts/stream_manifests.py --reference ... --streams ../ym2612-python/validation/streams --out DIR

A stream is the reference's ``(cycle, port, data)`` list (``scripts/run_fm_streams.py``).
Its ``replay`` writes each event before the cycle it names runs, from power-on, and
then runs ``--tail`` cycles more (12,000, its default). The manifest does the same:
a write at tick ``cycle * 42``, which ``run`` reaches having completed exactly
``cycle`` cycles, and ``ticks`` at the last cycle's end. So a trace identical to the
reference's here is the run the reference compared with Nuked-OPN2, cycle for cycle.

Run with the reference's interpreter: it imports the reference's scripts. Board
streams are derived from ROMs, so their manifests stay local like the streams.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

TAIL = 24 * 500
TICKS_PER_CYCLE = 42


def slug(name: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", name.lower()).strip("-")


def write(out: Path, name: str, events: list[tuple[int, int, int]], tail: int) -> int:
    cycles = (events[-1][0] if events else 0) + tail
    for variant in ("ym2612", "ym3438"):
        manifest = {
            "version": 1,
            "name": f"{name}-{variant}",
            "variant": variant,
            "ticks": cycles * TICKS_PER_CYCLE,
            "events": [
                {"t": cycle * TICKS_PER_CYCLE, "kind": "write", "port": port, "data": data}
                for cycle, port, data in events
            ],
        }
        (out / f"{name}-{variant}.json").write_text(json.dumps(manifest), encoding="utf-8")
    return cycles


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--synthetic", action="store_true", help="the seven synthetic streams")
    parser.add_argument("--streams", type=Path, help="a directory of exported board streams")
    parser.add_argument("--tail", type=int, default=TAIL)
    parser.add_argument(
        "--bench",
        action="store_true",
        help="the benchmark's stream: the seed-1 random walk, 40,000 writes (benchmarks/bench.py)",
    )
    args = parser.parse_args(argv)
    sys.path.insert(0, str(args.reference / "scripts"))
    import referees
    import run_fm_streams as fm

    args.out.mkdir(parents=True, exist_ok=True)
    streams: list[tuple[str, list[tuple[int, int, int]]]] = []
    if args.synthetic:
        streams += [
            ("random-walk-seed-1", fm.stream_random(1, 4000)),
            ("random-walk-seed-2", fm.stream_random(2, 4000)),
            ("ch3-special-and-csm", fm.stream_ch3_csm()),
            ("ssg-eg-eight-shapes", fm.stream_ssg_eg()),
            ("lfo-speeds-pms-ams", fm.stream_lfo()),
            ("both-timers", fm.stream_timers()),
            ("dac", fm.stream_dac()),
        ]
    if args.bench:
        streams.append(("bench-random-walk-seed-1-40000", fm.stream_random(1, 40_000)))
    if args.streams:
        files = sorted(args.streams.glob("*.bin"))
        if not files:
            print(f"{args.streams}: no exported streams (scripts/export_board_streams.py)")
            return 1
        streams += [(slug(path.stem), referees.read_events(path)) for path in files]
    total = 0
    for name, events in streams:
        cycles = write(args.out, name, events, args.tail)
        total += cycles
        print(f"{name}: {len(events):,} port writes, {cycles:,} cycles")
    print(f"{len(streams)} streams, {total:,} cycles each variant, in {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
