#!/usr/bin/env bash
# Rung 5: seeded random manifests -- writes at any tick, reads on every port, /IC
# pulses of every length at both lags, the quiet path, the $27 delay -- in both
# variants, where no fixed stream reaches.
# Usage: scripts/rung5.sh [count, default 128] [first seed, default 1]
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT="$ROOT/external/manifests/random"
python3 "$ROOT/scripts/random_manifests.py" --count "${1:-128}" --seed "${2:-1}" --out "$OUT"
exec "$ROOT/scripts/diff_all.sh" "$OUT"/*.json
