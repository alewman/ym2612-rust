#!/usr/bin/env bash
# Rung 2: every example manifest in the reference's examples/conformance/ diffs
# identical (pins, /IC, reads).
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
REF=${YM2612_PYTHON:-"$ROOT/external/ym2612-python"}
exec "$ROOT/scripts/diff_all.sh" "$REF"/examples/conformance/*.json
