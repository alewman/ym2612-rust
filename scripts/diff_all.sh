#!/usr/bin/env bash
# Diff every manifest given against the reference, in parallel: this core writes the
# trace, the reference's kit judges it through a pipe. Prints one line per manifest
# and a total; exits 1 if any differ.
# Usage: scripts/diff_all.sh manifest.json...   (JOBS=16 by default)
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
REF=${YM2612_PYTHON:-"$ROOT/external/ym2612-python"}
PY="$REF/.venv/bin/python"
[ -x "$PY" ] || { echo "no reference at $REF: run scripts/fetch_ym2612python.sh" >&2; exit 2; }
# TRACE_BIN: another build to judge (scripts/seeded_faults.py uses it).
if [ -z "${TRACE_BIN:-}" ]; then
    cargo build --release -q --manifest-path "$ROOT/Cargo.toml"
    TRACE_BIN="$ROOT/target/release/ym2612-trace"
fi
export ROOT PY TRACE_BIN
one() {
    "$TRACE_BIN" "$1" | nice "$PY" -m ym2612_python.conformance diff "$1" - 2>&1 | tail -3
}
export -f one
results=$(printf '%s\0' "$@" | xargs -0 -n1 -P "${JOBS:-16}" bash -c 'one "$0"')
echo "$results"
same=$(grep -c "traces are identical" <<<"$results" || true)
echo "TOTAL: $same of $# manifests identical"
[ "$same" -eq "$#" ]
