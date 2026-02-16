# Generated Runtime Includes

This directory contains generator outputs consumed by the NES runtime.

Ownership:
- produced by `nessy` pipeline (`make generate`)
- read by `src/main.c` and `src/chr.s`
- not intended for manual edits

Typical files:
- `roster_chr.inc`
- `roster_table.inc`
- `roster_meta.json`
- `title_logo.h`
- `title_tiles.inc`
- additional generated tile includes used by `src/chr.s`

Regeneration:

```bash
make generate
```

or directly:

```bash
cargo run --package nessy -- generate all
```
