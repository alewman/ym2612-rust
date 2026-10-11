#!/usr/bin/env python
"""A gate that cannot fail proves nothing: break this core one rule at a time and
check that the conformance examples catch every break.

    python scripts/seeded_faults.py [--jobs 8]

Each fault is one textual change to ``src/chip.rs``, applied to a copy of the
repository under ``external/faults/<name>/`` (the working tree is never touched) and
judged by ``cargo test --release --test conformance``: the thirteen examples against
the reference's traces. A fault the examples miss is then run through rung 3's and
rung 5's manifests (``external/manifests/synthetic`` and ``random``, written by
``scripts/rung3.sh`` and ``scripts/rung5.sh``; the reference must be fetched), and the
number of manifests that differ is reported. ym2612-python's ``scripts/mutation.py``
records one gap of its examples, the status decay, which needs 300,000 cycles
without a read.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

#: name -> (text in src/chip.rs, replacement). Each text must occur exactly once.
FAULTS = {
    "ladder-no-plus-one": ("            if out >= 0 {\n                out += 1;", "            if out >= 0 {\n                out += 0;"),
    "busy-16-cycles": ("(self.write_busy_cnt + self.write_busy) & 0x1F", "(self.write_busy_cnt + self.write_busy) & 0x0F"),
    "detune-sign-swapped": ("            basefreq - detune\n        } else {\n            basefreq + detune", "            basefreq + detune\n        } else {\n            basefreq - detune"),
    "feedback-one-step-up": ("modulation >> (10 - fb)", "modulation >> (9 - fb)"),
    "lfo-slowest-reload": ("[108, 77, 71, 67, 62, 44, 8, 5]", "[109, 77, 71, 67, 62, 44, 8, 5]"),
    "ssg-eg-twice-not-four-times": ("                        inc <<= 2; // SSG-EG", "                        inc <<= 1; // SSG-EG"),
    "timer-b-every-15": ("self.timer_b_subcnt == 0x10 &&", "self.timer_b_subcnt == 0x0F &&"),
    "ic-no-pause-carry": ("pub const IC_PAUSE_CARRY: i64 = 1;", "pub const IC_PAUSE_CARRY: i64 = 0;"),
    "dac-eight-bit-sign": ("sign_extend(8, self.dacdata)", "sign_extend(7, self.dacdata)"),
    "control-delay-one-late": ("Some((data, self.control_delay - 1))", "Some((data, self.control_delay))"),
    "key-on-operators-2-3-swapped": ("self.mode_kon[channel + 12] = operator[1];\n            self.mode_kon[channel + 6] = operator[2];", "self.mode_kon[channel + 12] = operator[2];\n            self.mode_kon[channel + 6] = operator[1];"),
    "status-decay-one-early": ("[300_000, 40_000_000]", "[299_999, 40_000_000]"),
}


def judge(name: str, old: str, new: str) -> tuple[str, bool, str]:
    copy = ROOT / "external" / "faults" / name
    copy.mkdir(parents=True, exist_ok=True)
    for item in ("Cargo.toml", "Cargo.lock", "src", "tests"):
        source = ROOT / item
        if source.is_dir():
            shutil.copytree(source, copy / item, dirs_exist_ok=True)
        else:
            shutil.copy2(source, copy / item)
    chip = copy / "src" / "chip.rs"
    text = (ROOT / "src" / "chip.rs").read_text()
    if text.count(old) != 1:
        return name, False, f"fault text found {text.count(old)} times"
    chip.write_text(text.replace(old, new))
    result = subprocess.run(
        ["nice", "cargo", "test", "--release", "-q", "--test", "conformance", "every_example"],
        cwd=copy,
        capture_output=True,
        text=True,
    )
    if result.returncode == 0:
        return name, False, escalate(copy)
    detail = next((line for line in result.stdout.splitlines() + result.stderr.splitlines() if "line " in line and ":" in line), "")
    return name, result.returncode != 0, detail.strip()


def escalate(copy: Path) -> str:
    """Rungs 3 and 5 for a fault the examples missed: how many manifests differ."""
    subprocess.run(["nice", "cargo", "build", "--release", "-q"], cwd=copy, check=True)
    found = []
    for rung, folder in (("rung 3", "synthetic"), ("rung 5", "random")):
        manifests = sorted((ROOT / "external" / "manifests" / folder).glob("*.json"))
        if not manifests:
            found.append(f"{rung}: no manifests")
            continue
        result = subprocess.run(
            [str(ROOT / "scripts" / "diff_all.sh"), *map(str, manifests)],
            env={**os.environ, "TRACE_BIN": str(copy / "target/release/ym2612-trace"), "JOBS": "4"},
            capture_output=True,
            text=True,
        )
        total = result.stdout.strip().splitlines()[-1] if result.stdout.strip() else result.stderr.strip()
        found.append(f"{rung}: {total.replace('TOTAL: ', '')}")
    return "; ".join(found)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--jobs", type=int, default=8)
    args = parser.parse_args()
    with ThreadPoolExecutor(args.jobs) as pool:
        results = list(pool.map(lambda item: judge(item[0], *item[1]), FAULTS.items()))
    caught = 0
    for name, failed, detail in results:
        caught += failed
        print(f"{'caught ' if failed else 'examples missed'} {name}: {detail[:130]}")
    print(f"{caught} of {len(FAULTS)} faults caught by the examples")
    return 0


if __name__ == "__main__":
    sys.exit(main())
