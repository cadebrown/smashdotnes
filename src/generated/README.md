# Generated Runtime Includes

This directory contains generator outputs consumed by the NES runtime.

Ownership:
- produced by `nessy` pipeline (`make generate`)
- read by `src/main.c` and `src/chr.s`
- not intended for manual edits
- static include payloads are synced from `nessy/assets/generated_static/`

Typical files:
- `roster_chr.inc`: flip-aware, deduplicated fighter tile atlas
- `roster_runtime.h`: semantic frame maps, sprite attributes, and action timelines
- `roster_table.inc`
- `roster_meta.json`
- `title_logo.h`
- `title_tiles.inc`
- `music_data.h`: expanded Menu 1 events, NTSC timing, and APU pulse controls
- fixed projectile and effect tile includes used by `src/chr.s`

Regeneration:

```bash
make generate
```

or directly:

```bash
cargo run --package nessy -- generate all
```
