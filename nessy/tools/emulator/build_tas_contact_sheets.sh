#!/usr/bin/env bash
set -euo pipefail

tool_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$tool_dir/../../.." && pwd)
artifact_root=${1:-"$root/out/reports/emulator/artifacts"}
output_dir="$artifact_root/tas-contact-sheets"
scratch=$(mktemp -d /tmp/smashdotnes-tas-sheets.XXXXXX)
trap 'rm -rf "$scratch"' EXIT

command -v magick >/dev/null || {
    echo "missing required command: magick" >&2
    exit 1
}

mkdir -p "$output_dir"
rm -f "$output_dir"/*.png

all_frames=()
for fighter in mario simon tommy rizer; do
    fighter_dir="$artifact_root/tas_$fighter"
    frames=()
    while IFS= read -r frame; do
        frames+=("$frame")
    done < <(printf '%s\n' "$fighter_dir"/*.png | sort)

    expected=16
    [[ "$fighter" == rizer ]] && expected=17
    if [[ ${#frames[@]} -ne $expected ]]; then
        echo "$fighter TAS has ${#frames[@]} screenshots, expected $expected" >&2
        exit 1
    fi

    rows=()
    for ((offset = 0; offset < ${#frames[@]}; offset += 4)); do
        row="$scratch/$fighter-row-$(printf '%02d' "$((offset / 4))").png"
        magick "${frames[@]:offset:4}" +append \
            -background '#111111' -gravity west -extent 1024x240 "$row"
        rows+=("$row")
    done
    magick "${rows[@]}" -append "$output_dir/$fighter.png"
    all_frames+=("${frames[@]}")
done

all_rows=()
for ((offset = 0; offset < ${#all_frames[@]}; offset += 8)); do
    row="$scratch/all-row-$(printf '%02d' "$((offset / 8))").png"
    magick "${all_frames[@]:offset:8}" +append \
        -background '#111111' -gravity west -extent 2048x240 "$row"
    all_rows+=("$row")
done
magick "${all_rows[@]}" -append "$output_dir/all-fighters.png"

echo "built TAS contact sheets in $output_dir"
