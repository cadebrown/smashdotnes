#!/usr/bin/env bash
set -euo pipefail

tool_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
root=$(cd "$tool_dir/../../.." && pwd)
cache=${1:-"$root/out/reference-sprites"}
lock="$root/nessy/assets/fighter_sources.lock"

for command in curl magick shasum; do
    command -v "$command" >/dev/null || {
        echo "missing required command: $command" >&2
        exit 1
    }
done

mkdir -p "$cache"
while IFS=$'\t' read -r filename sha256 url; do
    [[ -z "$filename" || "$filename" == \#* ]] && continue
    target="$cache/$filename"
    if [[ ! -f "$target" ]]; then
        curl -fsSL --retry 3 --max-time 30 "$url" -o "$target"
    fi
    actual=$(shasum -a 256 "$target" | awk '{print $1}')
    if [[ "$actual" != "$sha256" ]]; then
        echo "$filename hash mismatch: expected $sha256, got $actual" >&2
        exit 1
    fi
done < "$lock"

scratch=$(mktemp -d /tmp/smashdotnes-fighters.XXXXXX)
trap 'rm -rf "$scratch"' EXIT
mkdir -p "$scratch/mario" "$scratch/simon" "$scratch/tommy" "$scratch/rizer"

canvas_width=32
canvas_height=40

place_frame() {
    local source=$1
    local output=$2
    magick "$source" -alpha on -channel RGB -fx 'a == 0 ? 0 : u' +channel \
        -gravity southwest -background none -extent "${canvas_width}x${canvas_height}" \
        -depth 8 "$output"
}

place_trimmed_frame() {
    local source=$1
    local output=$2
    magick "$source" -trim +repage -alpha on \
        -channel RGB -fx 'a == 0 ? 0 : u' +channel \
        -gravity southwest -background none -extent "${canvas_width}x${canvas_height}" \
        -depth 8 "$output"
}

compose_sheet() {
    local fighter=$1
    local output="$root/src/chars/$fighter/sheet.png"
    local frames=("$scratch/$fighter"/frame-*.png)
    local expected_width=$((${#frames[@]} * canvas_width))
    magick "${frames[@]}" +append +repage "$output"
    dimensions=$(magick identify -format '%wx%h' "$output")
    colors=$(magick identify -format '%k' "$output")
    if [[ "$dimensions" != "${expected_width}x${canvas_height}" || "$colors" -gt 4 ]]; then
        echo "$fighter sheet is $dimensions with $colors colors; expected ${expected_width}x${canvas_height} and <=4" >&2
        exit 1
    fi
}

normalize_alpha() {
    local source=$1
    local output=$2
    magick "$source" -alpha on -channel RGB -fx 'a == 0 ? 0 : u' +channel "$output"
}

magick "$cache/smb3_mario_walk.gif" -coalesce "$scratch/mario/walk-source-%02d.png"
for source in "$scratch/mario"/walk-source-*.png; do
    normalize_alpha "$source" "${source/source-/}"
done
normalize_alpha "$cache/smb3_mario_jump.png" "$scratch/mario/jump.png"
normalize_alpha "$cache/smb3_mario_jump_fall.png" "$scratch/mario/fall.png"
normalize_alpha "$cache/smb3_super_mario_kick_raw.png" "$scratch/mario/kick.png"
normalize_alpha "$cache/smb3_mario_skid.png" "$scratch/mario/skid.png"

place_frame "$scratch/mario/walk-00.png" "$scratch/mario/frame-00.png"
place_frame "$scratch/mario/walk-00.png" "$scratch/mario/frame-01.png"
place_frame "$scratch/mario/walk-01.png" "$scratch/mario/frame-02.png"
place_frame "$scratch/mario/walk-02.png" "$scratch/mario/frame-03.png"
place_frame "$scratch/mario/jump.png" "$scratch/mario/frame-04.png"
cp "$scratch/mario/frame-04.png" "$scratch/mario/frame-05.png"
place_frame "$scratch/mario/fall.png" "$scratch/mario/frame-06.png"
cp "$scratch/mario/frame-02.png" "$scratch/mario/frame-07.png"
place_frame "$scratch/mario/kick.png" "$scratch/mario/frame-08.png"
cp "$scratch/mario/frame-01.png" "$scratch/mario/frame-09.png"
cp "$scratch/mario/frame-00.png" "$scratch/mario/frame-10.png"
cp "$scratch/mario/frame-03.png" "$scratch/mario/frame-11.png"
cp "$scratch/mario/frame-00.png" "$scratch/mario/frame-12.png"
place_frame "$scratch/mario/skid.png" "$scratch/mario/frame-13.png"

magick "$cache/simon_walk.gif" -coalesce "$scratch/simon/walk-%02d.png"
magick "$cache/simon_strong_whip.gif" -coalesce "$scratch/simon/whip-%02d.png"
magick "$cache/castlevania_simon_full.png" -alpha on -fill none \
    -opaque '#008000' -opaque '#747474' "$scratch/simon/source.png"

crop_simon() {
    local geometry=$1
    local index=$2
    local flip=${3:-}
    magick "$scratch/simon/source.png" -crop "$geometry" +repage \
        "$scratch/simon/crop-$index.png"
    if [[ "$flip" == "flip" ]]; then
        magick "$scratch/simon/crop-$index.png" -flop "$scratch/simon/crop-$index.png"
    fi
    place_frame "$scratch/simon/crop-$index.png" "$scratch/simon/frame-$index.png"
}

place_frame "$scratch/simon/walk-01.png" "$scratch/simon/frame-00.png"
place_frame "$scratch/simon/walk-00.png" "$scratch/simon/frame-01.png"
place_frame "$scratch/simon/walk-01.png" "$scratch/simon/frame-02.png"
place_frame "$scratch/simon/walk-02.png" "$scratch/simon/frame-03.png"
crop_simon 16x32+84+21 04 flip
cp "$scratch/simon/frame-04.png" "$scratch/simon/frame-05.png"
cp "$scratch/simon/frame-04.png" "$scratch/simon/frame-06.png"

extract_simon_attack_pose() {
    local source_index=$1
    local output_index=$2
    magick "$scratch/simon/whip-$source_index.png" -crop 16x31+16+0 +repage \
        "$scratch/simon/attack-$output_index.png"
    place_frame "$scratch/simon/attack-$output_index.png" \
        "$scratch/simon/frame-$output_index.png"
}

extract_simon_attack_pose 06 07
extract_simon_attack_pose 20 08
extract_simon_attack_pose 23 09
extract_simon_attack_pose 06 10
extract_simon_attack_pose 13 11
extract_simon_attack_pose 23 12
crop_simon 16x32+275+21 13

magick "$cache/ThomasR.gif" -coalesce "$scratch/tommy/walk-source-%02d.png"
normalize_tommy() {
    local source=$1
    local output=$2
    magick "$source" -alpha on \
        -fill '#FC9838' -opaque '#FFA044' \
        -fill '#FCFCFC' -opaque '#FFFFFF' \
        -channel RGB -fx 'a == 0 ? 0 : u' +channel "$output"
}
for source in "$scratch/tommy"/walk-source-*.png; do
    normalize_tommy "$source" "${source/source-/}"
done

crop_thomas() {
    local geometry=$1
    local index=$2
    magick "$cache/kung-fu-characters.gif" -crop "$geometry" +repage \
        -alpha on -fill '#FC9838' -opaque '#FFA044' \
        -fill '#FCFCFC' -opaque '#FFFFFF' \
        -channel RGB -fx 'a == 0 ? 0 : u' +channel \
        -gravity southwest -background none -extent "${canvas_width}x${canvas_height}" \
        "$scratch/tommy/frame-$index.png"
}
place_frame "$scratch/tommy/walk-00.png" "$scratch/tommy/frame-00.png"
place_frame "$scratch/tommy/walk-00.png" "$scratch/tommy/frame-01.png"
place_frame "$scratch/tommy/walk-01.png" "$scratch/tommy/frame-02.png"
place_frame "$scratch/tommy/walk-00.png" "$scratch/tommy/frame-03.png"
crop_thomas 32x40+235+131 04
crop_thomas 32x40+261+131 05
crop_thomas 32x40+296+131 06
crop_thomas 32x40+270+53 07
crop_thomas 32x40+247+53 08
crop_thomas 32x40+270+53 09
crop_thomas 32x40+270+53 10
crop_thomas 32x40+212+53 11
crop_thomas 32x40+270+53 12
crop_thomas 32x40+235+131 13

normalize_rizer() {
    local source=$1
    local output=$2
    magick "$source" -alpha on \
        -fill none -opaque 'srgba(64,64,64,0)' -opaque 'srgba(255,255,255,0)' \
        -fill '#D82800' -opaque '#4240FF' -opaque '#B53120' \
        -fill '#FCBCB0' -opaque '#F7D8A5' -opaque '#FECCC5' \
        -channel RGB -fx 'a == 0 ? 0 : u' +channel "$output"
}

for id in 08 09 0d 0e 0f; do
    normalize_rizer "$cache/contra_sprite_$id.png" "$scratch/rizer/sprite-$id.png"
done
normalize_rizer "$cache/BillStandingR.png" "$scratch/rizer/standing.png"

place_frame "$cache/BillStandingR.png" "$scratch/rizer/frame-00.png"
place_frame "$scratch/rizer/sprite-0d.png" "$scratch/rizer/frame-01.png"
place_frame "$scratch/rizer/sprite-0e.png" "$scratch/rizer/frame-02.png"
place_frame "$scratch/rizer/sprite-0f.png" "$scratch/rizer/frame-03.png"
place_frame "$scratch/rizer/sprite-08.png" "$scratch/rizer/frame-04.png"
place_frame "$scratch/rizer/sprite-09.png" "$scratch/rizer/frame-05.png"
magick "$scratch/rizer/frame-04.png" -flip -flop "$scratch/rizer/frame-06.png"
place_frame "$scratch/rizer/sprite-0d.png" "$scratch/rizer/frame-07.png"
magick "$scratch/rizer/standing.png" -crop 14x9+9+8 +repage \
    -flip -flop "$scratch/rizer/gun-butt-weapon.png"
magick "$scratch/rizer/standing.png" -fill none \
    -draw 'rectangle 9,8 22,16' "$scratch/rizer/gun-butt-body.png"
magick "$scratch/rizer/gun-butt-body.png" "$scratch/rizer/gun-butt-weapon.png" \
    -geometry +9+8 -composite "$scratch/rizer/gun-butt-active.png"
place_frame "$scratch/rizer/gun-butt-active.png" "$scratch/rizer/frame-08.png"
place_frame "$scratch/rizer/sprite-0f.png" "$scratch/rizer/frame-09.png"
place_frame "$scratch/rizer/sprite-0d.png" "$scratch/rizer/frame-10.png"
place_frame "$scratch/rizer/sprite-0e.png" "$scratch/rizer/frame-11.png"
place_frame "$scratch/rizer/sprite-0f.png" "$scratch/rizer/frame-12.png"
magick "$cache/contra_bill_lance_full.png" -crop 64x64+911+139 +repage \
    -alpha on -fill none -opaque '#1C6691' -opaque '#002F57' \
    -fill '#D82800' -opaque '#923404' -opaque '#4B24DE' \
    -fill '#FCBCB0' -opaque '#E2CDA7' -opaque '#F7CEC3' \
    "$scratch/rizer/hurt.png"
place_trimmed_frame "$scratch/rizer/hurt.png" "$scratch/rizer/frame-13.png"
magick "$scratch/rizer/frame-05.png" -flip -flop "$scratch/rizer/frame-14.png"

for fighter in mario simon tommy rizer; do
    compose_sheet "$fighter"
done

echo "rebuilt fighter sheets from locked references in $cache"
