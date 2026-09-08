# Automation

The repository tests behavior through the same ROM artifact a player receives. Text markers alone are not accepted as visual or gameplay evidence.

## Native test drive

```bash
make capture-audio
make capture-theme
make test-emu
```

`make capture-audio` drives these named 256x240 states:

1. title
2. lobby
3. battle
4. walk
5. jump
6. jab
7. special

It decodes every PNG, requires at least four colors, rejects a 99% dominant color, records 16-bit PCM, requires a meaningful signal, and rejects full-scale clipping.

`make capture-theme` records the complete Menu 1 arrangement to `out/reports/emulator/melee_theme.wav`. Its Lua drive reads the 16-bit music index, observes all 926 generated events, and proves the jump from event 925 to event 124 before reporting PASS.

For title-screen design work, run:

```bash
make title-iterations
```

The target compiles three separate mapper-0 ROMs, drives each through title, lobby, and battle in MAME, checks every capture and its 60 Hz title/battle cadence, runs the default world build again, and writes a comparison image to `out/reports/title-iterations/contact-sheet.png`. The alternate ROMs and their seven-state capture sets remain beside it for direct inspection.

The suite in `nessy/tools/emulator/suite/suite.toml` adds boot signature, frame progression, injected movement, four-voice APU activity, contact/damage/spark synchronization, all four fighters' authored animation poses, exact loaded-battle cadence, and longer-run responsiveness checks. `animation_states` selects each fighter, drives every required walk, jump, jab, and special pose, reads the rendered frame ID from the test ABI, and saves the distinct native frames. `battle_cadence` requires 360 simulation and music updates in 360 video frames while both fighters attack, jump, hit, and spawn projectiles. The manifest is the source of discovery; Rust does not contain a second hard-coded case list.

For the per-character visual audit:

```bash
make tas
```

`tas_mario`, `tas_simon`, and `tas_tommy` each produce 16 DMA-aligned screenshots: idle; three walk poses; rise, apex, and fall; jab startup, active, and recovery; special startup, active, and recovery; hurt; impact; and idle recovery. `tas_rizer` adds a seventeenth screenshot for curl D. The drive stages a real P2 hit and requires damage, hitstun, the hurt frame, and the impact effect before it can pass. `rizer_aerial_fire` then requires all four curl IDs while the special timer and a bullet are active. `make tas` assembles per-fighter and combined sheets under `out/reports/emulator/artifacts/tas-contact-sheets/`.

Each case runs in its own directories:

- `out/reports/emulator/runtime/<case>/cfg`
- `out/reports/emulator/runtime/<case>/nvram`
- `out/reports/emulator/artifacts/<case>`
- `out/reports/emulator/logs/<case>.log`

A missing Lua file, timeout, emulator error, or missing two-line report becomes a structured FAIL case. The runner still writes `summary.json` unless the manifest itself cannot be parsed.

## Adding a native scenario

Create a MAME Lua file beside the existing cases and add one `[[case]]` entry:

```toml
[[case]]
id = "projectile_regression"
script = "projectile_regression.lua"
timeout_seconds = 20
description = "Drives and verifies one projectile lifecycle."
```

The script must write:

```text
PASS
one-line evidence
```

to `NESSY_TEST_OUT`. Write screenshots or other case artifacts under `NESSY_ARTIFACT_DIR`. Read runtime state through the reserved `$06E0-$06FF` ABI rather than guessing from emulator timing.

Run one case with:

```bash
cargo run --package nessy -- test emulator --case projectile_regression
```

Repeat `--case` to run a focused set in one aggregate report.

Run the independent FCEUX boot and frame-progress check with:

```bash
make test-fceux
```

The FCEUX scenario uses its native Lua API and an offscreen Qt platform. It does not reuse the MAME-only scenario scripts.

## Rebuilding fighter art

```bash
make rebuild-fighters
cargo run --package nessy -- test emulator --case animation_states
```

The source lock stores URLs and SHA-256 digests for the coherent SMB3 Mario set, Castlevania Simon animations, Kung Fu Thomas references, and Contra ROM-extracted poses. The rebuild checks each reference before extracting poses and validates every final dimension and palette. Generated CHR then has its own 248-tile fighter budget gate, so new art cannot silently overwrite the eight effect tiles.

For a source-to-runtime comparison of Tommy, point the optional reference TAS at a locally owned original ROM:

```bash
KUNG_FU_ROM=/path/to/Kung\ Fu.nes make capture-kung-fu-reference
```

It captures 44 native title, idle, walk, jump, punch, and kick frames plus a contact sheet under `out/reference-sprites/kung-fu-original/`. The repository neither downloads nor ships the ROM.

## Browser test drive

```bash
make test-web
```

The command rebuilds and syncs the ROM, installs the pinned Playwright package, serves `web/public`, and drives:

- desktop landing page and real title-screen asset
- 390x844 responsive layout and overflow check
- generated manual and build-provenance data
- downloadable iNES magic bytes
- vendored EmulatorJS boot to its actual running state
- keyboard Start input with a changed canvas frame
- release of held mapped controls when focus leaves the emulator, the window loses focus, or the tab becomes hidden
- zero external requests and zero console errors

After Playwright captures the pages, `nessy test screenshots` decodes every PNG and rejects missing, 1x1, undersized, uniform, or nearly blank output. There is no placeholder fallback.

Failures retain a Playwright trace under `out/reports/web/playwright/`.

## Full gate

On a fresh checkout, fetch and verify the locked music source once:

```bash
uv run nessy/tools/music/rebuild_music.py --check
```

This downloads the MIDI named by `nessy/assets/music_sources.lock` when its ignored cache is absent, checks both locked digests, and compares the reduction with the tracked score. It does not rewrite `music.toml`. The subsequent music check stays offline.

```bash
make test
```

This runs the locked music check, Rust tests and doctests, the complete manifest-driven MAME suite, and the complete browser drive. Run `make test-fceux` for the independent emulator check. For a deployment-shaped artifact without publishing it:

```bash
make ci-pages
```

Inspect:

- `out/reports/emulator/summary.json`
- `out/reports/web/summary.json`
- `out/snaps/`
- `out/reports/web/screens/`
- `site/`

## Continuous integration

`.github/workflows/validate.yml` runs the full gate, FCEUX, and `make ci-pages` for pushes to `main`, pull requests, and manual dispatches. It compares the native ROM with both staged copies and rejects changes to tracked files during generation. The workflow has read-only repository permissions and does not publish or deploy.

The runner is Ubuntu 24.04. Actions are pinned to commit digests; Rust, Node, uv, and nextest have explicit versions. Cargo uses `Cargo.lock`, npm uses `web/package-lock.json`, and the MIDI is checked against its source lock. The runner installs cc65, MAME, FCEUX, Python, and browser system libraries from the Ubuntu image and repositories; those packages can receive updates. `out/reports/ci/toolchain.txt` records the actual tool versions. Ubuntu's MAME 0.264 includes the `emu.wait_next_frame` API used by the suite; FCEUX's package includes Lua 5.1 support. The actual emulator and browser scenarios must pass on the runner to establish compatibility.

Every completed run retains `validation-evidence` for 14 days, including available emulator logs, JSON summaries, screenshots, browser failure traces, and the ROM. Successful runs also retain `verified-site`. Setup failures may have no test artifacts; their step logs remain in Actions.

Cloudflare's connected-repository build remains independent of this validation job. Adding this workflow does not make Cloudflare wait for its result or configure a required branch-protection check. Those repository and hosting settings are separate from the local validation gate.

Primary references: [Playwright CI setup](https://playwright.dev/docs/ci-intro), [nextest installation in CI](https://nexte.st/docs/installation/pre-built-binaries/), [MAME 0.264 Lua API](https://github.com/mamedev/mame/blob/mame0264/docs/source/luascript/ref-common.rst), and [Ubuntu FCEUX package](https://packages.ubuntu.com/noble/fceux).

## Failure triage

1. Open the aggregate JSON and identify whether the failure is build, emulator infrastructure, runtime behavior, audio, browser runtime, or image evidence.
2. Run the named scenario or Playwright test alone.
3. Read its emulator log or Playwright trace.
4. Fix the lowest failing contract. Do not weaken the evidence check or substitute a placeholder artifact.
