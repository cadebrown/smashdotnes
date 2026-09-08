# Spec Reference

Character data is authored as TOML so content can scale without rewriting Rust generators.

## 1. Character Package

Each fighter lives in one folder:

- `src/chars/<id>/character.toml`
- `src/chars/<id>/sheet.png`
- `src/chars/<id>/generated/` (generated output)

That folder is the source of truth for runtime fighter art, semantic frame mapping, authored animation timing, and exported state metadata. Combat mechanics and state transitions are still implemented explicitly in `src/main.c`.

## 2. TOML Surface

The spec is organized around five concepts:

- `[character]`: identity and display fields
- `[spritesheet]` + `[[spritesheet.frames]]`: frame slicing from PNG
- `[[animations]]`: frame sequences with timing
- `[state_graph]` + `[[state_graph.states]]`: runtime state definitions
- `[[state_graph.states.transitions]]`: event-driven state edges

Validation guarantees that frame/animation/state references are consistent before code generation runs.

`character.runtime_slot` is required, unique, and contiguous from zero. Runtime sprites use a fixed 32x40 canvas: four NES tiles per row and five rows. Transparent pixels reserve palette index zero; every sheet may use at most three visible colors, and the compiler preserves one palette mapping across all frames.

Every runtime slot defines the 14 base poses: idle, three walk poses, three jump poses, three jab poses, three special poses, and hurt. Extra named poses are allowed; Rizer adds a fourth curl. Walk and jump tables contain four entries, while jab and special contain three. `minimum_distinct_frames` may explicitly permit an authentic held pose while keeping those runtime phases intact. The generated atlas omits fully transparent cells and shares flipped duplicates through per-cell hardware attributes; `0xFF` marks a skipped cell in the frame map.

For `jab` and `special`, `frame_duration` defines an equal three-phase timeline. `active_start` and `active_end` on the matching state are inclusive elapsed ticks; when omitted, the middle visual phase is active. The generator rejects reversed or out-of-range windows and emits the total duration, phase boundaries, and active window into `roster_runtime.h`. `cancel_start` and `cancel_end` remain exported graph metadata and are not interpreted by the current ROM.

## 3. Music Score

`nessy/assets/music.toml` is the tracked score consumed by the build. It is generated from the hash-locked Melee Menu 1 MIDI by `nessy/tools/music/rebuild_music.py`; edit that script's voice-routing policy and run `make rebuild-music` instead of hand-editing the expanded events. Each `[[song]]` sets millibeats-per-minute, pulse duty and volume, arrangement `order`, and `loop_order`, which is an index into that order rather than a raw generated event. Each nested `[[song.pattern]]` supplies equal-length arrays for source ticks, `pulse1`, `pulse2`, `triangle`, and `noise`.

Pitched voices accept note names such as `D5`, `F#4`, and `Bb2`; `=` holds the current note and `-` silences the voice. Noise events are `kick`, `snare`, `hat`, `rest`, or `hold`. The compiler converts cumulative source ticks into variable one-frame-or-longer NTSC durations, preserving 154.113 BPM to within one hardware frame. It rejects invalid notes, zero tempo or ticks, mismatched voice lengths, unknown or unused patterns, out-of-range APU controls, sub-frame events, and arrangements longer than the 16-bit runtime index.

The runtime plays the generated `menu` song continuously while its debug scene still follows title, lobby, and battle state. `make capture-theme` records the complete score from the actual MAME APU implementation and verifies the 926-event loop rather than substituting a desktop synthesizer.

## 4. Generated Outputs

`nessy generate all` emits:

- per-character includes and metadata under `src/chars/<id>/generated/`
- `src/generated/roster_chr.inc`, consumed directly by `src/chr.s`
- `src/generated/roster_runtime.h`, consumed directly by `src/main.c`
- roster inspection metadata under `src/generated/`

These files are machine outputs. They should be regenerated, not hand-edited.

The generated state-graph include is validated and inspectable, but the current ROM does not interpret arbitrary transition graphs. Do not treat a successful state-graph export as evidence that a new mechanic exists in gameplay.

## 5. Edit a Runtime Slot

Recommended flow:

1. update an existing slot's `character.toml` or start from `src/chars/_template.toml`
2. add aligned semantic frames in `sheet.png`, or update and run `nessy/tools/assets/rebuild_fighters.sh`
3. run `make generate`
4. run `make build` and `make test-emu`
5. run `make tas` and inspect the 16 screenshots under each `tas_<fighter>` directory, plus Rizer's seventeenth curl pose

Changing art, animation frame order, or walk timing is data-only. Adding another slot is not: the lobby grid, palettes, character constants, and CHR/OAM budgets require deliberate runtime changes.
