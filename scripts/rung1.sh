#!/usr/bin/env bash
# Rung 1: the two ROM tables, packed as 256 little-endian u16 each, against the
# SHA-256 that ym2612-python's docs/contract.md pins ("The tables").
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cargo build --release -q --manifest-path "$ROOT/Cargo.toml"
status=0
check() {  # table, pinned hash
    local got
    got=$("$ROOT/target/release/ym2612-tables" "$1" | sha256sum | cut -d' ' -f1)
    if [ "$got" = "$2" ]; then echo "$1: $got (pinned)"; else echo "$1: $got, pinned $2: DIFFERENT"; status=1; fi
}
check log_sin a099b16dd3aa2f56cabc1fee590db31ee412ee361da4f0c67497092cf57c4776
check exp a0ca448cb3178dd131cd23425b2adc8ee155d1905f2fa083d570954a47f3ede0
exit $status
