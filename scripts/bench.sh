#!/usr/bin/env bash
# The reference's benchmark on this core: its stream (the seed-1 random walk of
# 40,000 writes) as a manifest, driven through run() by ym2612-bench.
# Usage: scripts/bench.sh [--seconds S] [--repeat N]
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
REF=${YM2612_PYTHON:-"$ROOT/external/ym2612-python"}
OUT="$ROOT/external/manifests/bench"
"$REF/.venv/bin/python" "$ROOT/scripts/stream_manifests.py" --reference "$REF" --bench --out "$OUT" > /dev/null
cargo build --release -q --manifest-path "$ROOT/Cargo.toml"
exec "$ROOT/target/release/ym2612-bench" "$OUT/bench-random-walk-seed-1-40000-ym2612.json" "$@"
