# Cartridge font

`cartridge.woff2` is generated from the first-plane 8×8 `TILE1` glyphs in
[`src/chr.s`](../../../../src/chr.s). Each set bit becomes one 100-unit square in
an 800-unit em; lowercase letters map to the matching uppercase glyph.

Regenerate and validate with:

```sh
uv run web/scripts/build-cartridge-font.py
```

The script checks the source tile count and row shape, reopens the WOFF2, and
checks glyph bounds, advances, and source pixel counts for the mapped letters
and digits.

## Handwritten manual notes

Caveat 600 Latin is bundled from the pinned `@fontsource/caveat` 5.3.0 package.
The font is loaded locally through `manual-ink.css`; no Google Fonts request is
made by the site. Its SIL Open Font License is included as `caveat-OFL.txt`.
Upstream: https://github.com/googlefonts/caveat.
