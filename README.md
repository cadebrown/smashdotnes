# SmashDotNES

SmashDotNES is a retro-fitted Super Smash Bros-style NES project with a modern, automated toolchain.

The repository is organized for deterministic, headless development:
- NES runtime source and game data in `src/`
- reusable Rust tooling and CLI in `nessy/`
- a Rust + Dioxus web surface in `web/`
- publish-facing technical docs in `docs/`

## Highlights

- Unified per-character layout: one folder per fighter (`src/chars/<id>/`) containing spec, art, and generated outputs.
- Spec-driven content pipeline: new fighters are authored with TOML + PNG, then compiled without changing Rust source.
- Headless emulator automation: scripted MAME/FCEUX runs for smoke, progression, input, and long-run stability checks.
- Publish-ready web package: clean endpoints (`/play`, `/docs`, `/info`), ROM hosting, docs export, and automated screenshots.

## Repository Layout

```text
.
|- src/
|  |- main.c
|  |- chr.s
|  |- ppubuf_stub.s
|  |- nessmash.cfg
|  |- chars/
|  |  |- _template.toml
|  |  |- mario/
|  |  |- simon/
|  |  |- tommy/
|  |  `- rizer/
|  `- generated/
|- nessy/
|  |- src/
|  |- tests/
|  `- tools/
|- web/
|  |- src/
|  `- public/
|- docs/
|- Makefile
`- Cargo.toml
```

## Prerequisites

Required:
- Rust stable (`cargo`)
- `cl65` from cc65

Optional by workflow:
- `mame` for headless emulator suite and local runs
- `fceux` as alternate emulator backend
- Dioxus CLI (`dx`) for web build/dev (`0.7.2` to match crate dependencies)
- `firefox` (headless) for web screenshot automation

Run environment checks:

```bash
cargo run --package nessy -- doctor
```

## Quick Start

```bash
make doctor
make build
make test
```

This performs generation, ROM build + inspect, Rust tests, emulator suite, and web automation.

## Make Targets

- `make generate`: generate character + roster artifacts from `src/chars/*/character.toml`
- `make build`: generate assets, build `out/smash.nes`, run iNES inspection
- `make inspect`: inspect `out/smash.nes`
- `make run`: build and launch emulator (`EMULATOR=mame|fceux`)
- `make smoke`: run single emulator smoke case
- `make test-emu`: run full headless emulator suite
- `make test-web`: build + verify + screenshot the web interface
- `make test`: run workspace tests, emulator suite, and web suite
- `make web-sync`: sync ROM/docs/screenshots to `web/public`
- `make web-dev`: sync and launch Dioxus web dev server
- `make clean`: remove generated artifacts and output directories

## Web Routes and Controls

Primary web routes:
- `/`: landing page
- `/play`: emulator page
- `/docs`: four-guide documentation index
- `/info`: build metadata and links

Default `/play` keyboard bindings:
- `WASD`: movement
- `K`: A
- `J`: B
- `Enter`: Start
- `'`: Select

## Generated Runtime Directories

These directories are local runtime/build artifacts and are gitignored:

- `out/` (ROMs, reports, screenshots)
- `cfg/` and `nvram/` (emulator runtime state/config)
- `target/`, `dist/`, `site/`, `test-results/`
- generated includes under `src/generated/` and `src/chars/*/generated/`
- synced web outputs under `web/public/docs/`, `web/public/snaps/`, `web/public/rom/smash.nes`, `web/public/info/build.json`

## Add a New Fighter

1. Create a character folder:

```bash
mkdir -p src/chars/<id>
```

2. Add a sprite sheet and spec:
- `src/chars/<id>/sheet.png`
- `src/chars/<id>/character.toml` (start from `src/chars/_template.toml`)

3. Generate artifacts and build:

```bash
make generate
make build
```

Generated files appear in:
- `src/chars/<id>/generated/`
- `src/generated/`

No Rust code changes are required for standard character onboarding.

## Documentation

The docs are intentionally focused into four high-level guides:

- `docs/architecture.md`
- `docs/automation.md`
- `docs/spec_reference.md`
- `docs/implementation_notes.md`

The web app serves documentation from `/docs` (plus generated per-doc HTML pages) after `make web-sync`.

## Release Checklist

1. `make test`
2. `make web-sync`
3. Review outputs in `out/reports/emulator/summary.json` and `out/reports/web/summary.json`
4. Verify `out/smash.nes` and publish assets from `target/dx/smash-web/.../public`

## GitHub Actions Deploy

An automated build + deploy workflow is configured at:

- `.github/workflows/pages.yml`

Behavior:

- Pull requests: build-only validation (no deploy)
- `main` pushes + manual dispatch: build and deploy to GitHub Pages

Repository setting required:

- In GitHub `Settings -> Pages`, set `Source` to `GitHub Actions`

## License

MIT (workspace metadata in `Cargo.toml`).
