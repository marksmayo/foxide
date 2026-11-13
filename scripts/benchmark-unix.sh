#!/usr/bin/env bash
set -euo pipefail
# Usage: bash scripts/benchmark-unix.sh mpv /absolute/path/to/mpv media/generated/full [--tuned]
player=${1:?player mpv or vlc required}
executable=${2:?absolute player executable required}
media=${3:-media/generated/full}
output=${FOXIDE_RESULTS:-results/local/$(uname -s)}
python_bin=${FOXIDE_PYTHON:-python3}
export FOXIDE_PYTHON="$python_bin"
duration=${FOXIDE_DURATION:-630}
warmup=${FOXIDE_WARMUP:-30}
repeat=${FOXIDE_REPEAT:-5}
extra=()
if [[ ${4:-} == --tuned ]]; then extra+=(--tuned); fi
cargo build --release --locked -p player-bench
"$python_bin" scripts/generate-media.py --verify-only --output "$media"
mapfile_compat="$output/fixtures.txt"
mkdir -p "$output"
"$python_bin" - "$media" "$duration" > "$mapfile_compat" <<'PY'
import json, pathlib, sys
directory = pathlib.Path(sys.argv[1])
m = json.loads((directory / 'manifest.json').read_text())
if m['seconds'] < float(sys.argv[2]):
    raise SystemExit('Fixture duration must exceed benchmark duration')
for item in m['fixtures']:
    print(directory / item['path'])
PY
while IFS= read -r fixture; do
    target/release/player-bench run --player "$player" --exe "$executable" --media "$fixture" \
        --duration "$duration" --warmup "$warmup" --repeat "$repeat" --cache warm --output "$output" "${extra[@]}"
done < "$mapfile_compat"
target/release/player-bench report "$output" --output "$output/comparison.md"
