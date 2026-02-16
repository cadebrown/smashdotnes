# Spec Reference

Character data is authored as TOML so content can scale without rewriting Rust generators.

## 1. Character Package

Each fighter lives in one folder:

- `src/chars/<id>/character.toml`
- `src/chars/<id>/sheet.png`
- `src/chars/<id>/generated/` (generated output)

That folder is the single source of truth for art, animation, and state definitions.

## 2. TOML Surface

The spec is organized around five concepts:

- `[character]`: identity and display fields
- `[spritesheet]` + `[[spritesheet.frames]]`: frame slicing from PNG
- `[[animations]]`: frame sequences with timing
- `[state_graph]` + `[[state_graph.states]]`: runtime state definitions
- `[[state_graph.states.transitions]]`: event-driven state edges

Validation guarantees that frame/animation/state references are consistent before code generation runs.

## 3. Generated Outputs

`nessy generate all` emits:

- per-character includes and metadata under `src/chars/<id>/generated/`
- roster-wide includes and metadata under `src/generated/`

These files are machine outputs. They should be regenerated, not hand-edited.

## 4. Add a New Fighter

Recommended flow:

1. copy `src/chars/_template.toml` (or an existing fighter spec) as a starting point
2. add aligned sprites in `sheet.png`
3. run `make generate`
4. run `make build` and `make test-emu`
5. run `make web-sync` to publish updated web assets

If this flow requires code edits in `nessy`, it usually means the spec model should be improved.
