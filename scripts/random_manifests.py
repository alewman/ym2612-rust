#!/usr/bin/env python
"""Seeded random conformance manifests: rung 5, where no fixed stream reaches.

    python scripts/random_manifests.py --count 128 --seed 1 --out DIR

Seed ``s`` makes the same manifest on every machine (Python's ``random.Random``,
version 2 seeding, is stable across versions). Each manifest picks a variant, the
generator switch (on for seven in eight), register $27's delay (0 or 2), and a run
of at least 2-10 million ticks (48,000-238,000 internal cycles, 38-186 ms), then
fills it with:

* first, all six channels programmed to sound (:func:`voices`): every operator a
  random voice with a total level low enough to reach the DAC, every algorithm,
  feedback, pan and LFO depth, channel 3's own frequencies, and keys on;
* then register writes, address then data, mostly a cycle or more apart and one in
  ten inside a cycle (where only the later byte is latched), aimed at the registers
  that do something: key-on, every per-slot and per-channel group in both parts,
  the timers, the LFO, the DAC, CSM and channel 3's mode, the test registers $21
  and $2C now and then, stray addresses (the SSG's $00-$0F, unused ones), lone
  address and data bytes;
* reads on every port at any tick, so BUSY, the flags, the decay and the
  discrete chip's silent ports are all seen;
* /IC pulses at both lags (3, the button; 0, a write to $A11200), from one tick to
  thousands, some shorter than the hold, and the voices programmed again after each,
  starting inside the hold now and then;
* on a quarter of the discrete chip's runs, a probe of the driven value's decay:
  four reads of the undriven ports straddling the 300,000th cycle after the last
  read of port 0.

Standard library only. The files are rewritten on each run; stale ``random-*.json``
in ``--out`` are removed so a smaller ``--count`` does not leave old ones behind.
"""

from __future__ import annotations

import argparse
import json
import random
import sys
from pathlib import Path

SLOT_GROUPS = (0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90)
CHANNEL_GROUPS = (0xA0, 0xA4, 0xA8, 0xAC, 0xB0, 0xB4)
#: The offsets of a part's slot registers that name an operator (x3, x7, xB, xF name
#: none) and of its channel registers.
SLOT_OFFSETS = (0, 1, 2, 4, 5, 6, 8, 9, 10, 12, 13, 14)
CHANNEL_OFFSETS = (0, 1, 2)


def voices(rng: random.Random) -> list[tuple[int, int]]:
    """Program all six channels to sound: every operator a random voice whose total
    level is low enough to reach the DAC, every algorithm, feedback, pan and depth,
    channel 3's own frequencies, the LFO, and keys on. As (register, value), part
    II's as register | 0x100."""
    writes: list[tuple[int, int]] = [(0x22, rng.choice((0, 8 | rng.randrange(8))))]
    for part in (0, 0x100):
        for offset in SLOT_OFFSETS:
            r = part | offset
            ssg = 8 | rng.randrange(8) if rng.random() < 0.2 else 0
            writes += [
                (r | 0x30, rng.randrange(128)),
                (r | 0x40, rng.randrange(0, 36)),
                (r | 0x50, (rng.randrange(4) << 6) | rng.randrange(14, 32)),
                (r | 0x60, (rng.randrange(2) << 7) | rng.randrange(32)),
                (r | 0x70, rng.randrange(32)),
                (r | 0x80, rng.randrange(256)),
                (r | 0x90, ssg),
            ]
        for offset in CHANNEL_OFFSETS:
            r = part | offset
            high = (rng.randrange(1, 7) << 3) | rng.randrange(8)
            writes += [
                (r | 0xB0, rng.randrange(64)),
                (r | 0xB4, rng.choice((0xC0, 0xC0, 0x80, 0x40, 0x00)) | rng.randrange(64)),
                (r | 0xA4, high),
                (r | 0xA0, rng.randrange(256)),
            ]
    for offset in CHANNEL_OFFSETS:  # channel 3's own frequencies, $A8-$AA
        writes += [(0xAC, (rng.randrange(1, 7) << 3) | rng.randrange(8)), (0xA8 + offset, rng.randrange(256))]
    writes += [(0x28, 0xF0 | channel) for channel in (0, 1, 2, 4, 5, 6)]
    return writes


def register(rng: random.Random) -> tuple[int, int]:
    """One change while the voices play: mostly the registers that do something,
    now and then the test registers and stray addresses."""
    roll = rng.random()
    part = 0x100 if rng.random() < 0.5 else 0
    if roll < 0.25:  # key on/off, sometimes for no channel
        return 0x28, (rng.randrange(16) << 4) | rng.randrange(8)
    if roll < 0.55:
        group = rng.choice(SLOT_GROUPS)
        value = rng.randrange(0, 40) if group == 0x40 else rng.randrange(256)
        if group == 0x90 and rng.random() < 0.6:
            value = 8 | rng.randrange(8)  # SSG-EG on
        return part | group | rng.choice(SLOT_OFFSETS + (3, 7)), value
    if roll < 0.75:
        return part | (rng.choice(CHANNEL_GROUPS) + rng.choice((0, 1, 2, 3))), rng.randrange(256)
    if roll < 0.90:
        reg = rng.choice((0x22, 0x24, 0x25, 0x26, 0x27, 0x27, 0x2A, 0x2B))
        value = rng.randrange(256)
        if reg == 0x27 and rng.random() < 0.7:
            value = (rng.choice((0, 1, 2)) << 6) | rng.randrange(64)
        return reg, value
    if roll < 0.94:  # the test registers, mostly single bits
        reg = rng.choice((0x21, 0x2C))
        value = rng.randrange(256) if rng.random() < 0.3 else (1 << rng.randrange(8))
        return reg, value
    # Stray: the SSG's range, unused addresses, part II's mode range.
    return part | rng.choice((rng.randrange(0x10), 0x20, 0x23, 0x29, 0x2D, 0x2E, 0x2F)), rng.randrange(256)


def manifest(seed: int) -> dict:
    rng = random.Random(seed)
    variant = rng.choice(("ym2612", "ym3438"))
    enabled = rng.random() < 7 / 8
    control_delay = rng.choice((0, 0, 2))
    ticks = rng.randrange(2_000_000, 10_000_000)
    events: list[dict] = []
    t = rng.randrange(0, 2_000)

    def put(reg: int, value: int, gap: int) -> None:
        nonlocal t
        port = 2 if reg & 0x100 else 0
        events.append({"t": t, "kind": "write", "port": port, "data": reg & 0xFF})
        t += gap
        events.append({"t": t, "kind": "write", "port": port + 1, "data": value})

    def program() -> None:
        nonlocal t
        for reg, value in voices(rng):
            put(reg, value, rng.choice((42, 63, 84, 126)))
            t += rng.choice((42, 84, 126, 300))

    program()
    while t < ticks - 100:
        roll = rng.random()
        if roll < 0.85:
            reg, value = register(rng)
            # Mostly a cycle or more apart; one in ten inside a cycle, where the chip
            # latches only the later byte.
            gap = rng.choice((0, 1, 13, 21)) if rng.random() < 0.1 else rng.choice((42, 84, 200, 500))
            if rng.random() < 0.97:
                put(reg, value, gap)
            else:  # the address alone
                events.append({"t": t, "kind": "write", "port": 2 if reg & 0x100 else 0, "data": reg & 0xFF})
        elif roll < 0.87:  # a lone data byte
            events.append({"t": t, "kind": "write", "port": rng.choice((1, 3)), "data": rng.randrange(256)})
        elif roll < 0.995:
            events.append({"t": t, "kind": "read", "port": rng.randrange(4)})
        else:  # /IC: a pulse, and the voices programmed again after it
            events.append({"t": t, "kind": "ic", "held": True, "lag": rng.choice((0, 3))})
            t += rng.choice((1, 20, 42, 56, 112, 140, 300, 2_000, 9_000))
            events.append({"t": t, "kind": "ic", "held": False, "lag": rng.choice((0, 3))})
            # Writes and reads may land just after the release, inside the hold.
            t += rng.choice((1, 42, 100, 1_000))
            program()
            continue
        # Mostly close together; now and then a long quiet stretch, so envelopes run
        # down, the LFO turns and the timers overflow between writes.
        t += rng.choice((1, 30, 42, 100, 400, 1_008, 3_000, 10_000, 30_000, 100_000, 400_000))
    if variant == "ym2612" and rng.random() < 0.25:
        # The decay of the driven value (STATUS_TIME): a data write sets BUSY, port 0
        # drives the status, and nothing reads port 0 again; ports 1-3, which the
        # discrete chip does not drive, read it back until it lapses 300,000 cycles
        # later. Four probes straddle the moment, each inside its cycle at random.
        events.append({"t": t, "kind": "write", "port": 1, "data": rng.randrange(256)})
        t += 42
        events.append({"t": t, "kind": "read", "port": 0})
        base = t
        for cycles in (299_998, 299_999, 300_000, 300_001):
            t = base + cycles * 42 + rng.randrange(42)
            events.append({"t": t, "kind": "read", "port": rng.choice((1, 2, 3))})
        t += 42
    return {
        "version": 1,
        "name": f"random-{seed:05d}-{variant}",
        "variant": variant,
        "enabled": enabled,
        "control_delay": control_delay,
        "ticks": max(ticks, t),
        "events": events,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--count", type=int, default=128)
    parser.add_argument("--seed", type=int, default=1, help="the first seed")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args(argv)
    args.out.mkdir(parents=True, exist_ok=True)
    for stale in args.out.glob("random-*.json"):
        stale.unlink()
    for seed in range(args.seed, args.seed + args.count):
        m = manifest(seed)
        (args.out / f"{m['name']}.json").write_text(json.dumps(m), encoding="utf-8")
    print(f"{args.count} manifests (seeds {args.seed}-{args.seed + args.count - 1}) in {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
