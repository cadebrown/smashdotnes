# Architecture

SmashDotNES has four explicit boundaries: an NES runtime, a source-to-ROM compiler, observable emulator automation, and a static browser distribution of the same ROM.

## Products

- `out/smash.nes`: NROM-256 ROM, 32 KiB PRG plus 8 KiB CHR
- `nessy`: Rust asset compiler, linker driver, inspectors, and scenario runner
- `web/public`: static browser player, manual, provenance, and ROM endpoint
- `site`: clean Cloudflare Pages staging artifact

## Build flow

```text
fighter PNG + TOML ──> deduplicated CHR atlas ──> chr.s ──┐
                     └> frame maps + flip attrs ─> main.c ├─> cl65 ─> smash.nes
music.toml ───────────> music_data.h ─────> music.c┘
                                                        │
                       MAME Lua <── TEST_RAM ABI <──────┤
                       EmulatorJS <── web sync <────────┘
```

The normal sequence is:

1. `nessy generate all`
2. `nessy rom build`
3. `nessy rom inspect`
4. `nessy test emulator`
5. `nessy web sync`
6. Playwright browser drive plus Rust screenshot inspection

## Runtime data

The four tracked character sheets in `src/chars/` provide 14 poses for Mario, Simon, and Tommy and 15 for Rizer on a shared 32x40 canvas. The generator skips transparent 8x8 cells, deduplicates equal tiles, and canonicalizes horizontal and vertical variants so the renderer can recover them with PPU sprite-flip bits. It also compacts each frame into visible `(x, y, tile, attributes)` commands, so the 6502 never scans transparent canvas cells during battle. `src/chr.s` includes the resulting atlas directly. `src/main.c` reads the packed render commands, semantic animation indices, and timing from `roster_runtime.h`.

The current atlas uses 244 sprite tiles for fighters. Eight effect tiles follow it, leaving four slots free in the 256-tile sprite pattern table. Fireball, axe, and Rizer curl phases recover mirrored source frames with hardware flip bits, the whip repeats its identical middle segment, and the lobby marker shares the hit-spark tile. Background UI, clouds, hills, bricks, and title art use the separate background pattern table.

TOML controls visual frame mapping plus jab/special duration, phase boundaries, and active windows. Those generated timelines drive both the visible pose and collision checks. Movement rules and state transitions remain explicit C code; the exported state-graph bytes are validated build artifacts, not a general runtime state-machine interpreter.

The music importer verifies the locked Melee Menu 1 MIDI and maps its source lead handoffs, counterline, bass, and percussion into four NES voices. Original loop metadata fixes the target at 154.113 BPM and 2,995 NTSC frames. The compiler expands named event patterns, converts cumulative MIDI ticks into variable frame durations, and validates voice lengths, loop boundaries, notes, tempo, and pulse duty/volume before emitting `music_data.h`. `music_update()` advances once per game frame and keeps the same theme position across scene changes. Gameplay cues share those APU voices under a short, deterministic priority rule.

## Frame ordering

One battle tick uses a stable order:

1. wait for vblank, DMA the previously prepared OAM, and refresh changed HUD tiles
2. sample controllers, age hit effects, and tick current action timers
3. choose controller or CPU intent and apply movement, jump, and attack input
4. detect both melee contacts before applying either hit, then update projectiles
5. apply fighter physics, landing, hitstun, and KOs; prepare changed damage text
6. advance music and prepare the next OAM frame from the resulting state
7. publish the test state and advance the frame counter

Collision and visible attack frames therefore agree. Simultaneous active melee
contacts trade regardless of player number, while a hit still cancels an attack
that has not reached contact. Contact creates a short spark; hitboxes are not
rendered in release visuals.

The platform is one-way: landing requires a downward crossing of its top surface.
Entering its horizontal span from below cannot snap a fighter upward. CPU recovery
prioritizes steering toward the platform and using available jumps while falling.

## Observable runtime ABI

The linker reserves `$06E0-$06FF` outside C BSS. The runtime writes this 32-byte block every frame, and MAME scenarios treat it as a versioned API.

| Offset | Meaning |
| --- | --- |
| 0-1 | `SN` signature |
| 2 | scene: title, lobby, battle |
| 3 | game mode |
| 4 | frame counter |
| 5-6 | sampled P1/P2 input |
| 7 | active projectile count |
| 8-11 | P1 X and Y, signed little-endian 16-bit values |
| 12-13 | P1 damage and KO count |
| 14-17 | P2 X and Y, signed little-endian 16-bit values |
| 18-19 | P2 damage and KO count |
| 20 | music scene |
| 21-22 | 16-bit music event index |
| 23 | most recent sound cue |
| 24-29 | P1/P2 attack, special, and hitstun timers |
| 30 | packed rendered frame IDs: P1 low nibble, P2 high nibble |
| 31 | packed render load: OAM count low six bits, active hit effects high two bits |

Any offset change requires matching Lua and documentation changes.

## Emulator boundary

MAME drives the full automation suite because those Lua scripts use MAME's `manager.machine` device, memory, screen, and input APIs. Each manifest case gets isolated config, NVRAM, artifacts, logs, and a required report path. A separate FCEUX-native Lua smoke test independently verifies mapper loading, the runtime signature, and frame progression without pretending the MAME scripts are portable.

Four fighter TAS cases each select one roster slot, drive idle, every walk/jump/jab/special phase, validate a bounded jump arc, stage an opponent hit, and capture hurt, impact, and recovery. Rizer's case requires all four curl poses; `rizer_aerial_fire` separately proves those poses continue while a live bullet crosses the screen. Each screenshot waits one extra game tick after observing its rendered frame ID so the image matches the OAM data transferred by the next DMA.

The ROM header does not declare battery-backed RAM, so test sessions cannot inherit accidental save state.

## Web boundary

The publish tree is static. EmulatorJS and its NES core are vendored under `web/public/assets`, the version request is redirected locally, and the player does not fetch external scripts or localization at runtime.

Public build metadata includes the source revision and dirty flag, but excludes
Git remotes: build services can embed credentials in their checkout URLs.

Browser readiness comes from EmulatorJS's start event, not `window.load`. Playwright waits for the emulator's running state, sends a real mapped key, compares rendered canvas frames, and confirms focus can leave with Tab. A separate image decoder rejects blank and placeholder captures.

## Deployment boundary

`make ci-pages` wipes only the repository's `site/` staging directory and copies `web/public` into it. Cloudflare never builds a Dioxus debug bundle and does not need a `dx` executable. Tests do not deploy or mutate Cloudflare state.
