# Implementation Notes

These notes describe the current runtime shape and why it is intentionally conservative.

## 1. Core Game Loop

The runtime flows through a compact state machine:

- title
- lobby
- battle

The locked Melee Menu 1 port continues through title, lobby, and battle transitions. Its variable-duration event schedule and gameplay simulation both advance once per main-loop frame.

## 2. Fighter Model

The boundary is explicit:

- fighter CHR, semantic frame indices, walk timing, action phases, and active windows come from generated payloads
- idle and hurt have one pose; walk and jump expose four table entries; jab and special have three timed phases
- source-authentic single-pose actions may hold one drawing across several phases
- normal jumps select rise, apex, or fall from vertical velocity; Rizer cycles four curl poses while airborne
- AI, damage, movement rules, and state transitions remain C mechanics
- movement/combat interactions remain deterministic
- projectile pools and HUD state are bounded

This keeps runtime complexity manageable on 6502 constraints.

## 3. NES Constraints

The project treats hardware limits as design inputs:

- explicit linker map via `src/nessmash.cfg`
- tile/palette restrictions respected in generation
- `$06E0-$06FF` reserved outside BSS for deterministic test visibility
- no accidental battery-backed RAM flag or cross-run save state
- contact effects drawn before fighters to bound OAM priority pressure
- visible sprite cells compile into a packed OAM command stream to preserve 60 Hz under combat load
- 244 deduplicated fighter tiles plus eight effect tiles leave four sprite CHR slots free

The result is less flashy code but much stronger reliability.

## 4. Web Player Packaging

The web player is a publish target for the same ROM, not a forked game path.

- ROM + docs + metadata are synced into the static `web/public` tree
- EmulatorJS runs from local vendored assets
- Playwright waits for a live core, drives mapped input, and validates rendered screenshots

This lets one build pipeline serve both emulator testing and end-user play.

Reference links used by implementation and tooling:

- NESdev PPU programming: https://www.nesdev.org/wiki/PPU_programming
- NESdev iNES format: https://www.nesdev.org/wiki/INES
- cc65 NES target docs: https://cc65.github.io/doc/nes.html
- `cl65` docs: https://cc65.github.io/doc/cl65.html
- MAME command-line docs: https://docs.mamedev.org/commandline/commandline-all.html
- MAME Lua API docs: https://docs.mamedev.org/luascript/
- FCEUX command-line docs (interactive alternate): https://fceux.com/web/help/CommandLineOptions.html
- EmulatorJS project: https://github.com/EmulatorJS/EmulatorJS
- SMB3 Mario poses: https://www.spriters-resource.com/nes/supermariobros3/asset/4230/
- SMB3 Mario kick: https://www.mariowiki.com/File:SMB3_Super_Mario_kick.png
- Castlevania Simon animations: https://www.nesmaps.com/maps/Castlevania/sprites/CastlevaniaSprites.html
- Castlevania Simon labeled sheet: https://www.spriters-resource.com/nes/cv/asset/8477/
- Kung Fu character sheet: https://www.spriters-resource.com/nes/kungfu/asset/32871/
- Contra disassembly sprite library: https://github.com/vermiceli/nes-contra-us/tree/main/docs/sprite_library

The battle nametable uses a sky field, three-part clouds, green hill slopes, and alternating brick masonry derived from the same four-color background constraints as Super Mario Bros. The platform remains a simple collision plane; the richer tile treatment is presentational, not a second collision map.
