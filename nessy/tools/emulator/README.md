# Emulator Test Suite

`nessy test emulator` runs Lua scripts from `nessy/tools/emulator/suite/`.

## Runtime Contract

Scripts read runtime-exported state from CPU RAM at `$06E0`.
See `docs/architecture.md` for the offset map.

If this contract changes, update:
- `src/main.c` (`publish_test_state`)
- suite scripts in this directory
- architecture docs

## Cases

- `boot_smoke.lua`: boot + signature + title-state check
- `frame_progress.lua`: frame counter progression check
- `input_right.lua`: synthetic input and P1 movement check
- `long_stability.lua`: long-run signature/state/frame stability check

## Running

```bash
cargo run --package nessy -- test emulator --backend mame --rom out/smash.nes
```

Single case:

```bash
cargo run --package nessy -- test emulator --backend mame --case boot_smoke --rom out/smash.nes
```

## Reports

Each case writes a two-line report to `NESSY_TEST_OUT`:
1. `PASS` or `FAIL`
2. detail string

The suite also writes:
- `out/reports/emulator/summary.json`
