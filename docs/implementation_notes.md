# Implementation Notes

These notes describe the current runtime shape and why it is intentionally conservative.

## 1. Core Game Loop

The runtime flows through a compact state machine:

- title
- lobby
- battle

The goal is predictable progression and stable frame behavior, not feature breadth at this stage.

## 2. Fighter Model

Combat logic is data-driven where possible:

- animation/state behavior comes from generated payloads
- movement/combat interactions remain deterministic
- projectile pools and HUD state are bounded

This keeps runtime complexity manageable on 6502 constraints.

## 3. NES Constraints

The project treats hardware limits as design inputs:

- explicit linker map via `src/nessmash.cfg`
- tile/palette restrictions respected in generation
- deterministic RAM contract for test visibility

The result is less flashy code but much stronger reliability.

## 4. Web Player Packaging

The web player is a publish target for the same ROM, not a forked game path.

- ROM + docs + metadata are synced into `web/public`
- EmulatorJS runs from local vendored assets
- routes `/play`, `/docs`, and `/info` are tested in automation

This lets one build pipeline serve both emulator testing and end-user play.

Reference links used by implementation and tooling:

- NESdev PPU programming: https://www.nesdev.org/wiki/PPU_programming
- NESdev iNES format: https://www.nesdev.org/wiki/INES
- cc65 NES target docs: https://cc65.github.io/doc/nes.html
- `cl65` docs: https://cc65.github.io/doc/cl65.html
- MAME command-line docs: https://docs.mamedev.org/commandline/commandline-all.html
- MAME Lua API docs: https://docs.mamedev.org/luascript/
- FCEUX command-line docs: https://fceux.com/web/help/CommandLineOptions.html
- FCEUX Lua docs: https://fceux.com/web/help/LuaFunctionsList.html
- EmulatorJS project: https://github.com/EmulatorJS/EmulatorJS
- SMB1 sprite references: https://nesmaps.com/maps/SuperMarioBrothers/sprites/SuperMarioSprites.html
- Castlevania sprite references: https://nesmaps.com/maps/Castlevania/sprites/CastlevaniaSprites.html
- Contra sprite references: https://nesmaps.com/maps/Contra/sprites/ContraSprites.html
