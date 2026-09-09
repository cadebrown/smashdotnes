---
title: Assets
description: The ROM, captures, fonts, fighter sheets, and vendored browser runtime shipped by the site.
sidebar:
  label: Assets
  order: 2
---

The browser player receives the same `smash.nes` iNES ROM built in `out/`. `make web-sync` copies it to `web/public/rom/smash.nes`; Astro stages that file again for the final static site.

| Asset | Source | Format |
| --- | --- | --- |
| Playable ROM | `out/smash.nes` → `web/public/rom/smash.nes` | iNES `.nes` |
| Native captures | `web/public/assets/captures/` | 256×240 PNG |
| Fighter sheets | `src/chars/<id>/sheet.png` | PNG, 32×40 source frames |
| Pixel font | `web/public/assets/fonts/cartridge.woff2` | WOFF2 |
| Emulator runtime | `web/public/assets/emulatorjs/data/` | local JavaScript, CSS, JSON, and NES core data |
| Surface textures and icon | `web/public/assets/` | SVG |

`web/scripts/stage-assets.mjs` clears `web/.public/` before copying assets, so a removed source asset cannot survive a later build. It also places captures at `snaps/` for browser checks and stages fighter sheets at `assets/fighters/`.

The Cartridge font is generated from the NES `TILE1` glyphs in `src/chr.s`. Regenerate and validate it with:

```bash
uv run web/scripts/build-cartridge-font.py
```

Fighter sheets are authored data, not a web-only sprite sheet. Change a sheet or its `character.toml`, then run `make generate`; see [build](/docs/build/) and [testing](/docs/testing/).

## Fighter packages

Each `src/chars/<id>/` package pairs `sheet.png` with `character.toml`. The TOML is the source of truth for the fighter’s generated CHR, frame metadata, and animation timing; combat rules remain in `src/main.c`.

| Section | Required fields | Purpose |
| --- | --- | --- |
| `[character]` | `id`, `display_name`, `runtime_slot` | Identifies a contiguous runtime roster slot; `franchise` is optional metadata. |
| `[spritesheet]` | `path`, `frame_width`, `frame_height` | Locates the source PNG and its frame grid. |
| `[[spritesheet.frames]]` | `name`, `x`, `y` | Names a frame and gives its top-left pixel position; `w` and `h` are optional. |
| `[[animations]]` | `name`, `frames`, `frame_duration` | Selects named frames and their tick duration; `looping` defaults true and `minimum_distinct_frames` is optional. |
| `[state_graph]` / `[[state_graph.states]]` | `initial`; `name`, `animation` | Connects named animations to runtime states. Action states may set paired `active_start`/`active_end` or `cancel_start`/`cancel_end` ticks. |

Mario’s walk is the minimal pattern: names refer to declared frames, and repeated frame names produce the cadence.

```toml
[[animations]]
name = "walk"
frames = ["walk_a", "walk_b", "walk_c", "walk_b"]
frame_duration = 5
looping = true
```

The generator validates frame, animation, and state references, then writes generated files under `src/chars/<id>/generated/` and `src/generated/`. The full schema is in [`docs/spec_reference.md`](https://github.com/cadebrown/smashdotnes/blob/main/docs/spec_reference.md).

## Music source

`nessy/tools/music/rebuild_music.py` reduces the locked MIDI source into `nessy/assets/music.toml`. `make generate` compiles that TOML into `src/generated/music_data.h`; edit the reducer policy rather than hand-editing generated C data.

| TOML section | Key fields | Maps to |
| --- | --- | --- |
| `[timing]` | `ticks_per_beat`, `ntsc_frame_rate_num`, `ntsc_frame_rate_den` | MIDI tick-to-NTSC-frame conversion |
| `[[song]]` | `id`, `tempo_millibpm`, `order`, `loop_order` | A named arrangement and loop point |
| `[[song.pattern]]` | `id`, `ticks`, `pulse1`, `pulse2`, `triangle`, `noise` | Equal-length event streams for the four NES voices |

Pitch streams accept note names plus `=` to hold and `-` to rest. The noise stream accepts `kick`, `snare`, `hat`, `rest`, or `hold`. Run `make rebuild-music` to verify the locked source and regenerate `music.toml`, or `make check-music` for its offline check.
