#!/usr/bin/env bash
# Check out the ym2612-python reference at the pinned commit and install it into a
# virtual environment, so the rung 2-5 diffs run against exactly that core and kit.
# Usage: scripts/fetch_ym2612python.sh [destination]
# YM2612_PYTHON_SOURCE overrides where it is cloned from (a local clone will do).
set -euo pipefail
COMMIT=${YM2612_PYTHON_COMMIT:-2c2ab235aed619b395ba152111cd79a6193390b3}
SOURCE=${YM2612_PYTHON_SOURCE:-https://github.com/alewman/ym2612-python.git}
DEST=${1:-"$(dirname "$0")/../external/ym2612-python"}
PYTHON=${PYTHON:-python3}
if [ ! -d "$DEST/.git" ]; then
    git clone -q "$SOURCE" "$DEST"
fi
git -C "$DEST" fetch -q origin
git -C "$DEST" checkout -q --detach "$COMMIT"
if [ ! -x "$DEST/.venv/bin/python" ]; then
    "$PYTHON" -m venv "$DEST/.venv"
fi
"$DEST/.venv/bin/python" -m pip install -q -e "$DEST"
echo "ym2612-python at $(git -C "$DEST" rev-parse --short HEAD) installed in $DEST/.venv"
