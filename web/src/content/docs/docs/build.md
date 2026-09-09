---
title: Build
description: Build the mapper-0 ROM, inspect it, and prepare the local browser player.
sidebar:
  label: Build
  order: 1
---

Run these commands from the repository root:

```bash
make doctor
make generate
make build
make web-sync
```

`make doctor` checks required and optional tools. `make generate` compiles character PNG/TOML and music TOML into ROM inputs. `make build` performs generation, writes `out/smash.nes`, and inspects its iNES header. `make web-sync` first builds the ROM, then copies the ROM, documentation, and native captures into the browser-facing public tree.

For an interactive native run, use `make run EMULATOR=mame` or `make run EMULATOR=fceux`. The Makefile supports only those two `EMULATOR` values. Use `make web-dev` to start the synchronized Astro player on port 8080.

Generated fighter includes belong under `src/generated/` and `src/chars/<id>/generated/`; edit `character.toml`, `sheet.png`, or the generator source, then regenerate. See [testing](/docs/testing/) for validation targets.
