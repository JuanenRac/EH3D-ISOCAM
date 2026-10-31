#!/usr/bin/env bash
# Runs eh3d-isocam with its sample. Extra arguments replace the sample: ./run.sh [ARGUMENTS]
set -u
cd "$(dirname "$0")"
pause() { [ -n "${NOPAUSE:-}" ] || { read -r -p "Press Enter to close..." _ || true; }; }
trap pause EXIT
if [ "$#" -eq 0 ]; then set -- isolate examples/sample.gtl examples/sample.nc --passes 2; fi
cargo run --release --quiet -- "$@"
