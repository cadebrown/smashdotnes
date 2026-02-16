# nessy

`nessy` is the Rust tooling crate for SmashDotNES.

It provides:
- library APIs for spec loading, CHR conversion, state-graph compilation, and pipeline generation
- CLI commands for generation, ROM build/inspect orchestration, emulator tests, and web sync
- automation scripts used by `make test-emu` and `make test-web`

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
cargo run --package nessy -- web sync
```

`web sync` is the source of truth for publish data staging. It copies:
- `out/smash.nes` -> `web/public/rom/smash.nes`
- `docs/` -> `web/public/docs/` (+ generated HTML + clean routes)
- `out/snaps/` -> `web/public/snaps/`
- generated metadata -> `web/public/info/build.json`

## Tests

```bash
cargo test --package nessy
```

## Tools Directory

- `tools/emulator/suite/*.lua`: headless emulator integration cases
- `tools/web/test_web.sh`: web bundle verification and screenshot automation
