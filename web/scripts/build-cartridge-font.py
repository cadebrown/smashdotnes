#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "fonttools==4.59.1",
#   "brotli==1.1.0",
# ]
# ///
"""Build the cartridge bitmap font directly from the NES CHR assembly."""

from __future__ import annotations

import re
from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "src/chr.s"
OUTPUT = ROOT / "web/public/assets/fonts/cartridge.woff2"
TILE_RE = re.compile(r"^\s*TILE([12])\s+(.+)$", re.MULTILINE)
BYTE_RE = re.compile(r"%([01]{8})")

EXPECTED_TILES = 58
TILE_TO_CHAR = {0: " ", 2: "-", 3: ":", 15: "+", 24: "%", 25: "."}
TILE_TO_CHAR.update({index: str(index - 4) for index in range(4, 14)})
TILE_TO_CHAR.update({index: chr(ord("A") + index - 32) for index in range(32, 58)})


def read_tiles() -> dict[int, list[int]]:
    source = SOURCE.read_text()
    block = source.split("BgStart:", 1)[1].split("; 58+:", 1)[0]
    lines = TILE_RE.findall(block)
    if len(lines) != EXPECTED_TILES:
        raise ValueError(f"expected {EXPECTED_TILES} tile macros, found {len(lines)}")
    tiles_by_number = {}
    for tile_number, (tile_kind, line) in enumerate(lines):
        fields = [field.strip() for field in line.split(",")]
        if len(fields) != (8 if tile_kind == "1" else 16) or any(not BYTE_RE.fullmatch(field) for field in fields):
            raise ValueError(f"tile {tile_number} is not eight 8-bit rows")
        rows = [int(BYTE_RE.fullmatch(field).group(1), 2) for field in fields]
        tiles_by_number[tile_number] = rows[:8]
    if set(TILE_TO_CHAR) - set(tiles_by_number):
        raise ValueError("required glyph tile is missing")
    return tiles_by_number


def pixel_glyph(rows: list[int], advance: int = 800):
    pen = TTGlyphPen(None)
    for row, bits in enumerate(rows):
        for column in range(8):
            if bits & (0x80 >> column):
                x0, y0 = column * 100, (7 - row) * 100
                pen.moveTo((x0, y0))
                pen.lineTo((x0 + 100, y0))
                pen.lineTo((x0 + 100, y0 + 100))
                pen.lineTo((x0, y0 + 100))
                pen.closePath()
    return pen.glyph(), advance


def boxed_notdef():
    pen = TTGlyphPen(None)
    for x0, y0, x1, y1 in ((0, 0, 100, 800), (700, 0, 800, 800), (0, 700, 800, 800), (0, 0, 800, 100)):
        pen.moveTo((x0, y0)); pen.lineTo((x1, y0)); pen.lineTo((x1, y1)); pen.lineTo((x0, y1)); pen.closePath()
    return pen.glyph()


def main() -> None:
    tiles = read_tiles()
    glyph_order = [".notdef", "space", "hyphen", "colon", "plus", "percent", "period"]
    glyph_order += [f"zero" if n == 0 else ["one", "two", "three", "four", "five", "six", "seven", "eight", "nine"][n - 1] for n in range(10)]
    glyph_order += list("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
    glyph_order = list(dict.fromkeys(glyph_order))
    glyphs = {".notdef": boxed_notdef()}
    metrics = {".notdef": (800, 0)}
    for tile, char in TILE_TO_CHAR.items():
        name = "space" if char == " " else {"-": "hyphen", "+": "plus", ":": "colon", "%": "percent", ".": "period"}.get(char, {str(n): ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"][n] for n in range(10)}.get(char, char))
        glyphs[name], advance = pixel_glyph(tiles[tile])
        set_columns = [column for row in tiles[tile] for column in range(8) if row & (0x80 >> column)]
        metrics[name] = (advance, min(set_columns, default=0) * 100)
    cmap = {ord(char): name for tile, char in TILE_TO_CHAR.items() for name in ["space" if char == " " else {"-": "hyphen", "+": "plus", ":": "colon", "%": "percent", ".": "period"}.get(char, {str(n): ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"][n] for n in range(10)}.get(char, char))]}
    cmap.update({ord(char.lower()): char for char in "ABCDEFGHIJKLMNOPQRSTUVWXYZ"})
    builder = FontBuilder(800, isTTF=True)
    builder.setupGlyphOrder(glyph_order)
    builder.setupGlyf(glyphs)
    builder.setupHorizontalMetrics(metrics)
    builder.setupHorizontalHeader(ascent=800, descent=0)
    builder.setupCharacterMap(cmap)
    builder.setupNameTable({"familyName": "Cartridge", "styleName": "Regular", "fullName": "Cartridge Regular", "psName": "Cartridge-Regular"})
    builder.setupOS2(sTypoAscender=800, sTypoDescender=0, usWinAscent=800, usWinDescent=0)
    builder.setupPost(); builder.setupHead(created=0, modified=0)
    builder.font.flavor = "woff2"
    builder.font.recalcTimestamp = False
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    builder.save(str(OUTPUT))
    if OUTPUT.read_bytes()[:4] != b"wOF2":
        raise ValueError("output is not WOFF2")
    font = TTFont(OUTPUT, recalcTimestamp=False)
    if font.getBestCmap().get(ord("A")) != "A" or font["hmtx"]["A"][0] != 800:
        raise ValueError("font reopen validation failed")
    for char in "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-:+%.":
        name = font.getBestCmap()[ord(char)]
        glyph = font["glyf"][name]
        tile = (ord(char) - ord("A") + 32) if char.isalpha() else (ord(char) - ord("0") + 4 if char.isdigit() else {"-": 2, ":": 3, "+": 15, "%": 24, ".": 25}[char])
        expected = sum(row.bit_count() for row in tiles[tile])
        if glyph.numberOfContours != expected:
            raise ValueError(f"{char} pixel count mismatch: {glyph.numberOfContours} != {expected}")
        points = [(column * 100, (7 - row) * 100)
                  for row, bits in enumerate(tiles[tile])
                  for column in range(8) if bits & (0x80 >> column)]
        expected_bounds = (min(x for x, _ in points), min(y for _, y in points),
                          max(x for x, _ in points) + 100, max(y for _, y in points) + 100)
        if (glyph.xMin, glyph.yMin, glyph.xMax, glyph.yMax) != expected_bounds:
            raise ValueError(f"{char} geometry mismatch: {glyph.xMin, glyph.yMin, glyph.xMax, glyph.yMax} != {expected_bounds}")
        expected_lsb = min((column for row in tiles[tile] for column in range(8) if row & (0x80 >> column)), default=0) * 100
        if font["hmtx"][name] != (800, expected_lsb):
            raise ValueError(f"{char} metrics mismatch")
        actual_points = set(map(tuple, glyph.getCoordinates(font["glyf"])[0]))
        expected_points = {(x, y) for row, bits in enumerate(tiles[tile]) for column in range(8) if bits & (0x80 >> column) for x, y in ((column * 100, (7 - row) * 100), (column * 100 + 100, (7 - row) * 100), (column * 100 + 100, (8 - row) * 100), (column * 100, (8 - row) * 100))}
        if actual_points != expected_points:
            raise ValueError(f"{char} contour points mismatch")
        if glyph.xMin < 0 or glyph.xMax > 800 or glyph.yMin < 0 or glyph.yMax > 800:
            raise ValueError(f"{char} geometry outside 800-unit em")
    for char in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
        if font.getBestCmap().get(ord(char.lower())) != font.getBestCmap().get(ord(char)):
            raise ValueError(f"lowercase alias missing for {char}")
    if font["glyf"][".notdef"].numberOfContours != 4:
        raise ValueError("boxed .notdef validation failed")
    print(f"validated {len(tiles)} source tiles; wrote {OUTPUT} ({OUTPUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
