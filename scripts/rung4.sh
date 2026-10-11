#!/usr/bin/env bash
# Rung 4: the register streams megadrive-python's board fed its chip (Nemesis's six
# test ROMs, five cartridges), exported by the reference's
# scripts/export_board_streams.py. Derived from ROMs, so local only: pass the
# directory they were exported to.
# Usage: scripts/rung4.sh [streams dir, default ../ym2612-python/validation/streams]
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
REF=${YM2612_PYTHON:-"$ROOT/external/ym2612-python"}
STREAMS=${1:-"$ROOT/../ym2612-python/validation/streams"}
OUT="$ROOT/external/manifests/board"
"$REF/.venv/bin/python" "$ROOT/scripts/stream_manifests.py" --reference "$REF" --streams "$STREAMS" --out "$OUT"
exec "$ROOT/scripts/diff_all.sh" "$OUT"/*.json
