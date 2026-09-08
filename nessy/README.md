# nessy

`nessy` is the Rust tooling crate for SmashDotNES.

It provides:
- spec loading, palette-safe CHR conversion, state-graph compilation, music compilation, and roster generation
- ROM build/header inspection plus PCM and screenshot inspection
- manifest-driven MAME gameplay scenarios plus an independent FCEUX-native boot probe
- static web sync used by the Playwright browser drive

## CLI Overview

```bash
cargo run --package nessy -- --help
```

Top-level subcommands:
- `generate`
- `rom`
- `emulator`
- `test`
- `web`
- `doctor`

## Common Commands

```bash
cargo run --package nessy -- generate all
cargo run --package nessy -- rom build
cargo run --package nessy -- rom inspect --rom out/smash.nes
cargo run --package nessy -- test emulator --backend mame --rom out/smash.nes
cargo run --package nessy -- test screenshots --dir out/reports/web/screens
cargo run --package nessy -- web sync
```

`web sync` is the source of truth for publish data staging. It copies:
- `out/smash.nes` -> `web/public/rom/smash.nes`
- `docs/` -> `web/public/docs/` (+ generated HTML + clean routes)
- `out/snaps/` -> `web/public/snaps/` when native captures exist
- generated metadata -> `web/public/info/build.json`

## Tests

```bash
cargo nextest run --package nessy
cargo test --package nessy --doc
```

## Tools Directory

- `tools/emulator/suite/suite.toml`: discovered scenario manifest
- `tools/emulator/suite/*.lua`: MAME integration cases
- `tools/emulator/fceux/`: FCEUX-native suite and Lua probe
- `tools/emulator/capture.lua`: named gameplay screenshot driver
- `tools/emulator/build_tas_contact_sheets.sh`: deterministic per-fighter and combined TAS sheets
- `tools/assets/rebuild_fighters.sh`: hash-locked source-sheet extraction
- `tools/assets/capture_kung_fu_reference.sh`: optional original-ROM comparison TAS
- `tools/music/rebuild_music.py`: hash-locked, source-speed Melee Menu 1 port
- `tools/web/test_web.sh`: Playwright drive plus decoded screenshot gate
