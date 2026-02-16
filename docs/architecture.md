# Architecture

SmashDotNES is built to feel simple from the outside and strict on the inside:
fast build loops, deterministic behavior, and one content model that scales as characters are added.

## 1. What We Are Building

This project has three products that move together:

- a real NES ROM (`out/smash.nes`)
- a Rust toolchain (`nessy/`) that generates data and validates outputs
- a web bundle (`web/`) that runs the same ROM in-browser

The goal is not just a playable prototype. The goal is a repeatable system where new character content
can be added without touching core tool code.

## 2. System Boundaries

`src/` is the game runtime boundary:
- C and 6502 assembly
- character specs + spritesheets under `src/chars/<id>/`
- generated include artifacts consumed by cc65

`nessy/` is the automation boundary:
- spec parsing + validation
- PNG -> CHR and state graph generation
- ROM build/inspect/test entrypoints

`web/` is the publish boundary:
- browser play view
- docs and metadata routes
- static hosting output
- clean public routes (`/`, `/play`, `/docs`, `/info`)

`infra/` is the deployment boundary:
- OpenTofu config for Cloudflare Pages project/domain
- build command contract used by Pages (`infra/cloudflare/build.sh`)

## 3. Build Flow

The build path is intentionally short and explicit:

1. `nessy generate all`
2. `nessy rom build`
3. `nessy rom inspect --rom out/smash.nes`
4. `nessy test emulator --backend mame --rom out/smash.nes`
5. `nessy web sync`

Each step is deterministic and scriptable. This is what keeps CI behavior and local behavior aligned.
`make build`, `make test-emu`, `make test-web`, and `make web-sync` are thin wrappers over this flow.

Runtime/build outputs are intentionally isolated from source:
- `out/` for ROMs and reports
- `cfg/` and `nvram/` for emulator-generated state
- `src/generated/` and `src/chars/*/generated/` for generated includes

## 4. Runtime Contract

The runtime exports deterministic state at `$06E0` (`TEST_RAM`) every frame.  
Automation reads this memory to verify boot progress, state transitions, movement, and long-run stability.

Treat this memory surface as an API:
- changing offsets should be deliberate
- docs and emulator scripts should be updated in the same change
- contract stability is more important than convenience
