# Website and learning tools

The site uses Astro 7 + Starlight, with a vendored EmulatorJS player and a
project-local Starlight extension for article navigation and teaching styles.

From the repository root:

```bash
make web-dev
make test-web
make ci-pages
```

`make web-dev` rebuilds/syncs the ROM, installs locked dependencies and starts
Astro on port 8080. For an already prepared workspace, use
`npm --prefix web run dev -- --port 8082`.

`make test-web` runs the Astro type/content check, builds the production site,
and runs Playwright against that built output. `make ci-pages` copies the same
`web/dist` output into `site/`, verifies parity, and does not publish it.

## Source boundaries

- `src/pages/`: fighter cover, player, section indices, build information, design lab
- `src/content/docs/book/`: ordered player and implementation chapters
- `src/content/docs/docs/`: practical build, asset, testing, and website reference
- `src/content/docs/blog/`: dated development articles
- `src/components/learning/`: interactive explanations embedded in MDX
- `src/extensions/learning.mjs`: Starlight style/navigation extension
- `public/assets/`: vendored runtime, native captures, design assets
- `scripts/stage-assets.mjs`: prepares ignored `.public/` runtime inputs
- `dist/`: generated static site and Pagefind search index

Astro deliberately uses `.public/` instead of `public/`: the old static HTML
remains as reference, while only runtime assets enter the new build. The staging
script copies the built ROM, metadata, native captures, and original fighter
sheets. It clears its generated output so removed assets cannot linger.

## Routes

The home page is the fighter cover; it does not load the emulator. The four main
navigation links open `/play/`, `/book/`, `/docs/`, and `/blog/`. The player lives
on `/play/`. `/book/` renders four short, illustrated chapters in one document: first match,
controls, fighters, and combat. Architecture, physics, contact, rendering, and
audio live in Docs. Its contents links jump within the page. Reference pages have their
own flat sidebar; blog posts have neither a chapter sidebar nor pagination.

`/design/` retains the retro art workshop outside the main navigation. `/info/`
shows build provenance and `/rom/smash.nes` downloads the exact ROM used by the
player. `src/data/legacy-routes.ts` maps former guide, technical, devlog, and
emulator links to their current destinations while preserving query strings and
fragments. Former `/book/<chapter>/` URLs also redirect into the single book.
An existing fragment takes priority over the chapter's default anchor.

## Validation

The Book is a continuous instruction booklet, rendered by `pages/book.astro`
through Starlight so search and theme preferences remain available. The four player
chapters stay in `content/docs/book/`, sorted by their declared order. Desktop
contents stay beside the reading column; mobile contents collapse above it and
close after selection. Chapter headings are h2; lesson and diagram headings are
h3. `book.css` keeps text narrow and places screenshot annotations beside the
image, stacking them on phones. `FighterPortrait.astro` crops the original sheets
at integer scale.

`ControllerManual.astro` is a printed controller diagram: all four moves are
visible around Evan-Amos's original Nintendo NES controller photograph. Ink
arrows point at measured photo coordinates; keyboard overlays and callout keys
respond to mouse, touch, and local keyboard focus. `controller-controls.ts` owns
the mappings. `ControlMovePreview.astro` keeps two annotated native captures
visible together: recovery and a missed jab. No stepping or practice counter is
required to read the controls. The Play link is always available. The fight-zine header crops Mario and Rizer from the
original roster sheets; key presses briefly ring the matching controller button
unless reduced motion is requested.

The controls chapter uses a torn red masthead, a battered controller sheet, and
no right-hand contents rail. `src/assets/manual/battered-paper.png` supplies
generated material texture; Astro builds an optimized WebP. Its provenance is
documented alongside the source. `manual-ink-wear.svg`, `manual-paper-grime.svg`,
and `manual-type-wear.svg` add editable, static wear. Stamps, tape, and clippings
sample the same material at different scales. Keyboard labels and native
screenshots stay clear. No animated noise or full-page filter is used.
Desktop uses a spread; narrow screens put the diagrams in reading order. Astro
builds local 640px and 1280px WebP variants. The original photograph, public-domain
provenance, and checksum live in `src/assets/controllers/`. Native button activation,
Tab navigation, and held-key release remain available. `JumpSequence.astro` illustrates
press, release, and press again. `AnnotatedScreenshot.astro` overlays editable
SVG ink on unchanged native captures, with coordinates and teaching notes in
`src/data/manual-scenes.json` (typed by `manual-annotations.ts`). Its image and overlay share one coordinate
box at every size. These illustrations show menu choices, the three-step double jump, all four
specials, and missed versus landed jabs.

Manual handwriting uses locally bundled Caveat 600. The font license and
provenance are in `public/assets/fonts/`; `manual-ink.css` owns the shared ink
palette and font. Browser checks cover controller mapping, native activation,
focus escape, control overlap, annotation alignment, and narrow layouts.

`KnockbackLab.astro` traces the runtime's horizontal impulse at a chosen damage
total after a hit: `min(14, 4 + (damage >> 4))`, then move before one-unit decay.
It omits gravity, control input, stage contact, and KO resets. Jump trajectory
and authored-animation experiments live in Docs.

The browser gate covers responsive layout, ROM integrity, local-only resource
loading, real emulator boot and Start input, touch controls, fullscreen aspect
ratio, keyboard focus release, guide navigation, search, interactive learning
controls, the three art-board views, four workshop themes, sitewide screen effects, and manual
light/dark preferences. Screenshots/traces/reports are in
`out/reports/web/`; the Rust image inspector rejects placeholder or blank captures.

Website game-art studies are editable concepts, not new hardware-validated ROM
assets. Native screenshots show the existing ROM. Teaching simulations describe
their reduced scope alongside the relevant implementation links.

## Refresh the illustrations

Use `make manual-refresh`, inspect `out/reports/manual-refresh/index.html`, then
`make web-sync && make manual-accept && make test-web`. Annotation-only edits use
`make manual-review`. Every build runs the capture freshness check. The complete
[refresh recipe](../docs/manual-refresh.md) explains the input-only drive,
editable coordinates, review artifacts, and what to regenerate after UI changes.

## Retro presentation

`src/styles/retro.css` shares the display, bitmap, utility, and print-grain assets
between the playable site and Starlight. Anton is bundled locally under its OFL
license. The small `Cartridge` web font is generated from the actual 8×8 glyphs in
`src/chr.s`; regenerate it from the repo root with
`uv run web/scripts/build-cartridge-font.py`. The generator checks pixel geometry,
metrics, and the WOFF2 container. The checked-in font keeps normal npm builds
independent of Python. Fighter art uses the original sheets at integer scales.

The manual defaults dark and preserves explicit light or system-theme choices.
The art board contains a fight poster, fighter roster, and instruction booklet;
those studies do not alter the current ROM's stages or gameplay.

## Four-theme workshop

`/design/` compares four visual directions on the same poster, roster, and manual:

- **A — Fight Poster:** the existing red-and-bone baseline.
- **B — Rental Cartridge:** ROM-derived tile type, scuffed label, and plastic ribs.
- **C — Xerox Brawl:** monochrome fighter cutouts, pasted type, and copied paper.
- **D — After Hours:** amber/teal tile lettering and a worn arcade marquee.

Direct links use `?theme=a`, `?theme=b`, `?theme=c`, or `?theme=d`. A valid URL
choice takes precedence in the design workshop and persists in `smash-art-theme`
local storage. Without a query, the workshop uses the saved choice, defaulting to
B. The main website uses the chosen Rental Cartridge presentation throughout:
player, book, reference, blog, build information, and 404. A–D
remain available in the workshop. Changing studies preserves the current URL's
other parameters and fragment. Manual light/dark preferences remain independent.

`rental-site.css` owns the shared plastic shell and worn labels; `rental-docs.css`
adapts the reading panels to Starlight, including its light paper mode. The home
page includes the full centered fighter roster. The shared footer contains source,
ROM, build information, and Screen FX controls. Reading pages also expose their
light/dark preference there. The workshop roster has its own synchronized FX
control. `scripts/screen-effects.ts` persists `smash-screen-fx`; toggling effects
leaves the emulator running.

Edit names in `src/data/art-themes.ts`, the shared palette/material rules in
`src/styles/art-themes.css`, and composition studies in `src/styles/design.css`.
`src/components/themes/` owns the shared controls and selection behavior.
Wear textures are local editable SVGs; the game canvas is untouched.

### Rental roster refinement

`/design/?theme=b&study=fighters` opens the worn-cartridge fighter screen.
Study tabs preserve `study=poster|fighters|manual` in the URL. The roster uses
painted-pixel bounds in `src/data/roster.ts` to center the original poses, with
6/6/5/6 integer magnification on large screens and 4/4/3/4 on smaller screens.
CSS crops only transparent frame padding; source sheets and the ROM are unchanged.

`FighterRoster.astro`, `rental-roster.css`, and `rental-roster.ts` own the shell,
scanlines, phosphor shadows, grain, slow scan band, and pointer reflection.
Screen effects can be toggled off; this preference persists in `smash-screen-fx`.
Reduced motion stops the animated grain/scan band and pointer tracking. None of
these effects wrap the emulator. The roster tests independently measure source
alpha bounds and rendered alignment at five widths, writing
`out/reports/web/roster-layout.json` alongside visual captures.

The web gate writes its final JSON through
`scripts/write-validation-summary.mjs`. It verifies browser result counts, lists
the screenshots produced by that run, hashes the served artifacts, and replaces
the summary atomically. This avoids a blocking Bash here-document export.
