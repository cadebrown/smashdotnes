# SmashDotNES

SmashDotNES is a four-fighter platform brawler that runs as a real mapper-0 NES ROM. The repository also contains the content compiler, deterministic MAME drivers, audio and screenshot inspectors, and a static browser player that runs the same ROM through vendored EmulatorJS.

## What works

- Two-player, player-vs-CPU, and CPU-vs-CPU battles
- Mario, Simon, and Tommy with 14 editable poses; Rizer has a fifteenth curl pose
- Jabs, character specials, projectiles, jumping, knockback, damage, and KOs
- One-way platform landings, simultaneous melee trades, and CPU offstage recovery
- A source-speed four-voice NES port of Super Smash Bros. Melee's Menu 1 theme
- Contact-only hit sparks and synchronized simulation/render action frames
- Data-driven PNG/TOML fighter visuals and animation frame selection
- Deterministic native captures, PCM inspection, combat probes, and long-run checks
- Offline browser play with keyboard/touch controls and Playwright-driven screenshots

## Quick start

Required:

- Rust stable (`cargo`)
- cc65 (`cl65`)

For the full test drive:

- MAME (`mame`) for scripted emulator automation
- Node.js and npm for Playwright browser automation
- `uv` for the local static server
- ImageMagick (`magick`) for the title-direction comparison sheet

FCEUX is the independent native-core check. The full gameplay/TAS suite uses MAME's machine API; `make test-fceux` runs a separate FCEUX-native Lua boot and frame-progression probe.

```bash
make doctor
make build
make capture-audio
make test
```

`make capture-audio` drives seven named game states, validates every 256x240 PNG, records a WAV, and rejects silence or clipping. Reports land under `out/reports/`.

## Commands

- `make generate`: compile fighter PNG/TOML and music TOML into ROM inputs
- `make rebuild-fighters`: verify the locked sources and rebuild every fighter sheet
- `make capture-kung-fu-reference`: TAS a locally owned Kung Fu ROM for source comparison
- `make rebuild-music`: verify the locked Menu 1 MIDI and regenerate the event-driven NES port
- `make build`: generate, link `out/smash.nes`, and validate its iNES header
- `make run`: launch the ROM interactively (`EMULATOR=mame|fceux`)
- `make capture`: drive title, lobby, battle, walk, jump, jab, and special captures
- `make capture-audio`: run the same drive with validated PCM output
- `make capture-theme`: record the complete Menu 1 arrangement and prove its loop point
- `make title-iterations`: build, drive, and compare the world, challengers, and emblem title ROMs
- `make smoke`: run the manifest's boot case
- `make tas`: run 16-shot fighter drives, add Rizer's seventeenth curl, and build contact sheets
- `make test-emu`: run every case in `nessy/tools/emulator/suite/suite.toml`
- `make test-web`: drive desktop, mobile, docs, metadata, and a live EmulatorJS session
- `make test`: run Rust, native emulator, and browser gates
- `make web-dev`: sync and serve `web/public` at `http://127.0.0.1:8080`
- `make ci-pages`: stage the exact static deployment tree in `site/`

## Repository map

```text
src/
  main.c, music.c       NES runtime
  chars/<id>/           editable fighter TOML and 14/15-pose PNG sheets
  generated/            generated C/assembly inputs
nessy/
  assets/music.toml     generated NES score
  src/                  Rust compiler, ROM tools, emulator runner, inspectors
  tools/emulator/       capture driver and manifest-driven MAME scenarios
  tools/web/            browser gate entrypoint
web/
  public/               exact static publish source and vendored EmulatorJS
  tests/                Playwright interaction and screenshot tests
docs/                   architecture, automation, schema, implementation notes
infra/cloudflare/       Cloudflare Pages build/provisioning contract
```

## Fighter pipeline

Each runtime slot owns:

- `src/chars/<id>/sheet.png`: 14 or 15 aligned 32x40 frames
- `src/chars/<id>/character.toml`: frame regions, animations, timing, and state metadata

`make generate` emits:

- `src/generated/roster_chr.inc`: a flip-aware, deduplicated fighter atlas, consumed by `src/chr.s`
- `src/generated/roster_runtime.h`: packed OAM commands, hardware-flip attributes, semantic frame maps, and timing, consumed by `src/main.c`
- per-fighter CHR, state-graph, and metadata files for inspection

Idle and hurt use one pose. Walk uses an A-B-C-B four-step cadence; jab and special retain three timed phases. Non-looping jumps select rise, apex, and fall from vertical velocity, while Rizer cycles four source-authentic curl poses for his entire airborne state, including while firing. The current atlas packs 1,140 logical tile positions into 244 unique fighter tiles by skipping transparent tiles and sharing horizontal/vertical variants through NES sprite flips. Eight more tiles hold projectiles, whip segments, and the shared hit-spark/menu marker, leaving four sprite-pattern slots free. Mirrored fireball, axe, and curl phases use hardware flip bits instead of duplicate CHR.

The graph compiler emits each jab and special's total duration, phase boundaries, and inclusive active window from TOML. `src/main.c` uses those tables for both rendering and collision. Movement rules and state transitions remain explicit C mechanics; the exported graph is not a general runtime interpreter. Replacing art, frame order, or action timing is data-only. Adding a fifth slot still requires deliberate menu, palette, OAM, and CHR budget work.

To rebuild all four sheets from locked source images:

```bash
make rebuild-fighters
```

`nessy/assets/fighter_sources.lock` records each URL and SHA-256 digest. The script verifies every download, rebuilds coherent SMB3 Mario, Castlevania Simon, Kung Fu Thomas, and ROM-extracted Contra pose sets, and fails unless every 32x40 frame stays within one NES sprite palette. Raw references stay in the ignored `out/reference-sprites/` cache. `make capture-kung-fu-reference KUNG_FU_ROM=/path/to/Kung\ Fu.nes` drives an original local ROM and writes a 44-frame comparison sheet without bundling or downloading game code.

## Music pipeline

Run `make rebuild-music` to download and verify the locked [VGMusic Menu 1 MIDI](https://www.vgmusic.com/file/3196febfd6b9651f9360ccb17deeea3c.html), sequenced by Dr. Fruitcake, and regenerate `nessy/assets/music.toml`. The reducer follows the complete Menu 1 form and source events instead of extracting a few bars. Its 154.113 BPM schedule is derived from the original 1,594,676-sample, 32 kHz loop documented by [Smash Custom Music](https://www.smashcustommusic.net/song/3418): 49.8336 seconds, or 2,995 NTSC frames. Edit the voice-routing policy in `nessy/tools/music/rebuild_music.py`, not the generated TOML.

Nessy expands and validates the generated score into `src/generated/music_data.h`; malformed event ticks, voice lengths, notes, controls, tempo, and loops fail the build. `src/music.c` sequences 926 variable-duration events through pulse 1, pulse 2, triangle, and noise, keeps the theme continuous across scene changes, and layers short jump, hit, and KO cues over it.

## Agent and CI entrypoints

Native scenarios are declared in `nessy/tools/emulator/suite/suite.toml`. Add a Lua file and one manifest entry; Rust discovers, sorts, isolates, logs, and aggregates it. Each case receives:

- `NESSY_TEST_OUT`: required two-line PASS/FAIL report path
- `NESSY_ARTIFACT_DIR`: case-specific screenshot/artifact directory
- isolated MAME config and NVRAM directories

Browser tests start their own local server, reject external requests and console errors, wait for a running emulator rather than page load, inject real keyboard input, prove the canvas changes, verify Tab escapes the emulator, and reject blank or placeholder screenshots.

Useful evidence:

- `out/reports/emulator/summary.json`
- `out/reports/emulator/logs/*.log`
- `out/reports/emulator/artifacts/`
- `out/reports/web/summary.json`
- `out/reports/web/screens/*.png`
- `out/snaps/*.png`

## Web routes

- `/`: cartridge landing page
- `/play/`: offline EmulatorJS player
- `/docs/`: generated technical manual
- `/info/`: source and ROM build provenance
- `/rom/smash.nes`: downloadable ROM

Keyboard controls are `WASD` to move, `J` for A/jab, `K` for B/special, `Enter` for Start, and `'` for Select. Clicking the screen focuses input; Tab navigates out normally. Held keyboard controls release when focus leaves the player, the window loses focus, or the tab is hidden.

## Deployment

`make ci-pages` builds the ROM, syncs docs/metadata and any current captures, and copies only `web/public` into `site/`. A tracked title capture is used when MAME output is unavailable in the build environment. There is no Dioxus/WASM shell or `dx` dependency in the publish path. Cloudflare Pages uses `bash infra/cloudflare/build.sh` and publishes `site/`.

No deployment is performed by local test commands.

## License

MIT (workspace metadata in `Cargo.toml`).
