#!/usr/bin/env bash
set -euo pipefail

tool_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$tool_dir/../../.." && pwd)
rom=${KUNG_FU_ROM:?set KUNG_FU_ROM to a locally owned Kung Fu NES ROM}
output=${KUNG_FU_CAPTURE_OUTPUT:-"$root/out/reference-sprites/kung-fu-original"}
artifact_dir="$output/frames"
report="$output/report.txt"
runtime="$output/runtime"

for command in mame magick; do
    command -v "$command" >/dev/null || {
        echo "missing required command: $command" >&2
        exit 1
    }
done

[[ -f "$rom" ]] || {
    echo "Kung Fu ROM not found: $rom" >&2
    exit 1
}

mkdir -p "$artifact_dir" "$runtime/cfg" "$runtime/nvram"
rm -f "$artifact_dir"/*.png "$report" "$output/contact-sheet.png"

SDL_VIDEODRIVER=dummy \
SDL_AUDIODRIVER=dummy \
KUNG_FU_CAPTURE_DIR="$artifact_dir" \
KUNG_FU_CAPTURE_REPORT="$report" \
mame nes -cart "$rom" \
    -video none -sound none -nothrottle -seconds_to_run 35 \
    -autoboot_delay 1 -autoboot_script "$tool_dir/reference_kung_fu_tas.lua" \
    -cfg_directory "$runtime/cfg" \
    -nvram_directory "$runtime/nvram" \
    -snapshot_directory "$artifact_dir"

rg -q '^PASS$' "$report"
mapfile=()
while IFS= read -r frame; do
    mapfile+=("$frame")
done < <(printf '%s\n' "$artifact_dir"/*.png | sort)

if [[ ${#mapfile[@]} -ne 44 ]]; then
    echo "expected 44 reference frames, found ${#mapfile[@]}" >&2
    exit 1
fi

for frame in "${mapfile[@]}"; do
    dimensions=$(magick identify -format '%wx%h' "$frame")
    if [[ "$dimensions" != "256x240" ]]; then
        echo "$frame is $dimensions, expected native 256x240" >&2
        exit 1
    fi
done

rm -f "$runtime"/contact-row-*.png
rows=()
for ((offset = 0; offset < ${#mapfile[@]}; offset += 6)); do
    row="$runtime/contact-row-$(printf '%02d' "$((offset / 6))").png"
    magick "${mapfile[@]:offset:6}" +append \
        -background '#111111' -gravity west -extent 1536x240 "$row"
    rows+=("$row")
done
magick "${rows[@]}" -append "$output/contact-sheet.png"

echo "captured original Kung Fu reference to $output"
