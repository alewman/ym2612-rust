#!/usr/bin/env bash
# Rung 3: the seven synthetic streams of the reference's scripts/run_fm_streams.py,
# as manifests in both variants, every cycle. Equal to the reference here and the
# reference equal to Nuked-OPN2 on the same streams compose.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
REF=${YM2612_PYTHON:-"$ROOT/external/ym2612-python"}
OUT=${1:-"$ROOT/external/manifests/synthetic"}
"$REF/.venv/bin/python" "$ROOT/scripts/stream_manifests.py" --reference "$REF" --synthetic --out "$OUT"
exec "$ROOT/scripts/diff_all.sh" "$OUT"/*.json
