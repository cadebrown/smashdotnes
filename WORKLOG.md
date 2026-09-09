# Current work

Objective: apply the user's gritty, authentic retro + Smash direction throughout
the local Astro/Starlight website, retaining the immediately playable game and
all teaching tools. Use the actual fighter sprites and ROM lettering. Native
gameplay, new match modes, and roster expansion remain outside this website pass.

## Cleanup checkpoint: `62890e0`

- Existing four-fighter art, animation, music, native automation, and static web
  delivery overhaul is committed and pushed; remote `main` matched `62890e0`.
- Browser focus loss now releases held, currently mapped keyboard controls.
  Regression coverage includes keyboard navigation, window blur, hidden tabs,
  remapping, speed controls, and focus within the emulator toolbar.
- Corrected the documented NES A/jab and B/special names.
- Added clean-runner CI for Rust, native emulators, browser play, and site staging.
- Cleanup gate: 21 Rust tests, 13 MAME scenarios, FCEUX boot, all eight browser
  tests, format/typo checks, workflow lint, and exact site staging pass locally.

## Gameplay quality pass

Implemented downward-crossing platform landings, simultaneous melee contact
detection before hit effects, and stage-aware CPU recovery. The three new
controller-driven regressions fail on the checkpoint ROM and pass on the updated
ROM. Defender action cancellation now has a verified live-action precondition.

The first Linux CI run passed Rust and MAME but exposed 66px mobile overflow when
Impact is unavailable. The headline now sizes to its column. A fallback-font
regression fails before that CSS fix and passes at mobile and desktop widths.

Final local quality gate: 21 Rust tests, 16 MAME scenarios, FCEUX boot, nine browser
tests, format/typo/workflow checks, and exact site staging pass. Battle cadence
remains 360 simulation updates per 360 video frames with no stalls. Native impact,
platform recovery, CPU recovery, and fallback-font screenshots were inspected.

Production verification exposed an authenticated checkout URL in public build
metadata. The generator now excludes Git remotes entirely, with a synthetic
credential regression and a browser schema assertion. Prior deployment URLs and
credential revocation must be treated separately from replacing current content.

The original ROM and regression reports remain under `out/reports/quality-before/`;
updated comparison evidence is under `out/reports/quality-after/`. Full-gate
results use the standard `out/reports/emulator/`, `fceux/`, and `web/` paths.

Local tests do not establish physical hardware behavior or public deployment.
Check the latest GitHub Validate run and Cloudflare Pages check for the current
commit before making a release claim; those external statuses are not stored here.

## 2026-09-08: Direct player UI

Completed a local UI pass: the home page now opens the player, and legacy
`/play/` and `/emulator/` URLs redirect there. Removed the promotional landing
page, console decoration, feature counters, repeated headings, and extra toolbar
commands. The game has compact keyboard help, a ROM download link, and secondary
manual/build/source links. Manual and build pages use the same simpler styling.

Touch controls have their own space below the video, with A/B, direction, Start,
and Select. The narrow toolbar stays on one line; fullscreen preserves the video
aspect ratio. Existing keyboard focus-release behavior remains covered.

Validation: `make test-web` passed all 10 browser tests and screenshot validation;
`cargo fmt --all -- --check` and `git diff --check` passed. Desktop, narrow desktop,
touch, manual, and build screenshots were visually reviewed. The local and web
ROM hashes still match the previously validated ROM:
`daee33b394e3c52b4fd8fa66058a650b9497e459fb5a663714be2eb15e4d6625`.

Preview: http://127.0.0.1:8080/ . Evidence: `out/reports/web/summary.json`,
`out/reports/web/ui-final-gate.log`, and `out/reports/web/screens/`.
This UI pass is local and uncommitted for design review.

## 2026-09-08: Astro/Starlight learning site and design studies

Latest requested scope: compare several website and in-game aesthetic directions;
build Astro + Starlight with custom extensions, a player guide, technical chapters,
development articles, diagrams, charts, screenshots, and interactive explanations.
Keep the game immediately playable. Present concepts before choosing a final art
direction. Current design work is local; no new commit or publication requested.

Recovered the unfinished Astro migration from the interrupted status task. Fixed
the inherited sidebar configuration for Starlight 0.42 and connected the canonical
web test and staging commands to Astro output. The previous static tree remains
as reference, excluded from the new asset stage. Native ROM source is unchanged.

Completed locally:

- Playable homepage, four guide chapters, seven technical chapters, three devlog
  articles, journal index, build page, legacy redirects, and a custom 404.
- Starlight extension for shared teaching styles/article navigation, local fonts,
  Pagefind search, MDX build diagram, integer trajectory lab, and a pose/tick
  scrubber backed by generated character metadata.
- Arcade Atlas, Cartridge Handbook, and Frame Lab website/game-scene mockups.
  These are editable web concepts; no final ROM art direction is selected.
- Canonical browser tests and Cloudflare artifact staging now build Astro output.
  The generated `.public` tree excludes the legacy HTML and is cleared per build.
- Source review corrected the audio-channel ownership and projectile/melee
  distinction. Visual QA corrected mobile mockup columns and dynamic thumbnail
  styles. Those behaviors have regression coverage.

Verification: Astro check has zero errors, warnings, or hints; all 16 browser
tests and screenshot validation passed; 22 Rust tests and doctest gate passed;
format and whitespace checks passed. The exact static tree stages into `site/`.
The ROM SHA-256 remains
`daee33b394e3c52b4fd8fa66058a650b9497e459fb5a663714be2eb15e4d6625`.
The native gameplay suite was not rerun in this web-only pass; the built/served
ROM is byte-for-byte identical to the earlier validated artifact.

Preview: http://127.0.0.1:8082/design/ (built site); game at the same origin `/`.
Restart with `uv run python -m http.server 8082 --bind 127.0.0.1 --directory web/dist`
after `make ci-pages`, or use `make web-dev` for Astro live editing on port 8080.
The older static preview may still occupy port 8080.

Evidence: `out/reports/web/summary.json`, `out/reports/web/playwright.json`,
`out/reports/web/screens/`, `out/reports/astro-final-web-gate.log`,
`out/reports/astro-rust-gate.log`, and `out/reports/astro-site-staging.log`.
Next decision: select/refine an aesthetic direction, then apply its approved
game art through the NES asset pipeline and native visual/cadence gates.
All current site changes are local and uncommitted; nothing was published.

## 2026-09-08: Gritty retro / Smash direction applied

The user selected an authentic, gritty retro-game + Smash treatment and asked to
implement it throughout the website. Applied soot-black surfaces, bone paper,
red/orange ink, heavy condensed fight-poster typography, and restrained print
grain to the player, art board, manual, engineering chapters, journal, and build
page. The home page still opens directly into the playable ROM.

The art board now contains a fight poster, the four-fighter roster, and an
instruction booklet. Actual authored fighter sheets are enlarged at integer
scales. A 1,112-byte local WOFF2 font derives its pixel outlines directly from the
ROM's 8×8 CHR lettering, with a reproducible generator and geometry/metrics checks.
Anton is bundled locally so the display typography also works without host fonts.
Starlight defaults dark and preserves explicit paper/light or automatic themes.

Final validation:

- `make test-web`: 17 browser tests pass; no skipped, flaky, or failed tests.
  Covers gameplay and touch controls, local resource loading, mobile layout,
  search in both themes, persistent theme choice, art-board tabs and keyboard
  navigation, physics experiments, animation scrubbing, and ROM integrity.
- Screenshot validator passes. Visually reviewed desktop/mobile play, all three
  art-board treatments, technical diagrams, sprites, and dark/paper manuals.
- `make ci-pages`: Astro check has zero errors/warnings/hints; production build
  and exact `web/dist` → `site` parity pass. Shell syntax and whitespace checks pass.
- Updated the browser-report screenshot references for the renamed art studies.
- The ROM remains byte-for-byte unchanged:
  `daee33b394e3c52b4fd8fa66058a650b9497e459fb5a663714be2eb15e4d6625`.
  This pass does not claim changed in-game art or new native/hardware validation.

Evidence: `out/reports/retro-web-final-gate.log`,
`out/reports/retro-site-staging.log`, `out/reports/web/summary.json`,
`out/reports/web/playwright.json`, and `out/reports/web/screens/`.
The local preview remains http://127.0.0.1:8082/design/ with play at `/`.
Implementation and local verification are complete. Changes remain uncommitted;
no publication was requested or performed.

## 2026-09-08: Four retro themes ready for comparison

The user requested four choices (A/B/C/D), preserving the current look while
exploring stronger retro styling and grit. Implemented a live theme workshop at
`/design/` with the same poster, fighter roster, and manual views under each:

- A / Fight Poster: current warm red and bone baseline.
- B / Rental Cartridge: original CHR tile type, worn black label, plastic ribs.
- C / Xerox Brawl: monochrome cutouts, pasted headline, grainy copied paper.
- D / After Hours: multicolor tile marquee, amber/teal, restrained CRT texture.

The A–D radio selector is shared across the player, docs, and journal. URL
`?theme=a|b|c|d` overrides and persists to local storage; switching preserves other
query parameters, the fragment, and the live emulator instance. The docs retain
independent light/dark/auto preferences and include matching variants of all four
palettes. A fresh browser defaults to A. No final theme has been selected.

Shared theme definitions, controls, and palette rules live in
`web/src/data/art-themes.ts`, `web/src/components/themes/`, and
`web/src/styles/art-themes.css`; art compositions remain in `design.css`.
Textures are editable local SVGs. Visual review reduced repeated background
scratches, improved Xerox controls and dark/light contrast, fixed the compact
selector's default docs tokens, and corrected mobile heading whitespace. The
main browser chrome color follows the chosen palette.

Validation completed:

- `make test-web`: 18 passed, zero failed/skipped/flaky; screenshot validator PASS.
- Four-theme checks cover 320/390/900px layouts, direct links, persistence,
  URL/hash preservation, keyboard navigation, and preservation of the player.
- Captured all four art themes on desktop/mobile, all four player treatments,
  and all four docs themes in light and dark. Visually inspected these plus the
  live in-app browser workshop. Original gameplay/input/learning tests pass.
- `make ci-pages`: Astro check and static build pass; exact dist/site parity.
  Verified the final site hash matches the browser report and all 21 named
  screenshot references exist. Shell syntax and whitespace checks pass.
- ROM SHA-256 is unchanged:
  `daee33b394e3c52b4fd8fa66058a650b9497e459fb5a663714be2eb15e4d6625`.

Evidence: `out/reports/four-themes-web-final.log`,
`out/reports/four-themes-site-staging.log`, `out/reports/web/summary.json`,
`out/reports/web/playwright.json`, and `out/reports/web/screens/theme-*.png`.
Preview: http://127.0.0.1:8082/design/?theme=b (switch A–D in place).
Next step is user comparison/refinement. Work is local and uncommitted; nothing
was published, and no in-game art or hardware validation is claimed.

## 2026-09-08: Rental Cartridge roster / CRT refinement

The user selected Rental Cartridge / Fighter Roster as the starting point,
requested more grit and screen-space effects, and explicitly asked for visual
inspection of centered, sensibly sized fighters.

Implemented a larger cartridge shell with edge wear, a battered label, recessed
CRT glass, scanlines/RGB mask, phosphor shadows, moving grain, a slow scan band,
and pointer-following reflection. Hover/focus illuminates a fighter without
shifting its position or scale. Each fighter links directly to its guide section.
Screen FX toggles all screen overlays and persists its choice; reduced motion
disables animated grain, scan-band movement, and pointer tracking. The emulator
canvas remains outside these effects.

The source frames contain uneven transparent padding and different foot positions.
The roster now centers the painted bounds instead of the 32×40 frame canvas and
aligns the painted bottoms. Original action poses and aspect ratios are retained.
Desktop integer scales are Mario 6, Simon 6, Tommy 5, Rizer 6, yielding visible
heights 162/174/190/192px. Smaller screens use 4/4/3/4, yielding 108/116/114/128px.
All fighters are within 19% in visible height. Independent browser measurements
found zero center or baseline error and no clipped pixels at widths
1440/900/600/390/320.

Visual QA inspected full desktop/mobile shell captures and the live in-app
browser. It corrected an overlapping theme picker, aligned the floor with the
actual feet, and fixed the reduced-motion rule's selector specificity.
Study tabs now preserve `study=poster|fighters|manual` in the URL.

Sources: `web/src/components/design/FighterRoster.astro`,
`web/src/data/roster.ts`, `web/src/styles/rental-roster.css`,
`web/src/scripts/rental-roster.ts`, and `web/public/assets/rental-wear.svg`.
Measurements: `out/reports/roster-proportions.json` and
`out/reports/web/roster-layout.json`. Visuals: `rental-screen-1440.png`,
`rental-screen-390.png`, and the full-page/reduced-motion captures in
`out/reports/web/screens/`.

During validation, Homebrew Bash 5.3.15 stalled in `heredoc_write > write` while
exporting the expanded report to an internal pipe. The exact stalled child was
stopped. Replaced the shell here-document with
`web/scripts/write-validation-summary.mjs`: it checks result counts, enumerates
the current run's screenshots, hashes artifacts, and atomically replaces JSON.
Evidence: `out/reports/roster-report-hang.txt`. The complete gate was rerun after
this fix and finished normally.

Final validation: `make test-web` PASS, 19 tests, zero failed/skipped/flaky,
45 screenshots validated. `make ci-pages` PASS, Astro diagnostics clear and
exact dist/site parity. Final site/ROM hashes and all screenshot references
verified; shell/Node syntax and whitespace checks pass. The ROM hash remains
`daee33b394e3c52b4fd8fa66058a650b9497e459fb5a663714be2eb15e4d6625`.
Logs: `out/reports/rental-roster-web-final.log` and
`out/reports/rental-roster-site-staging.log`.

Preview: http://127.0.0.1:8082/design/?theme=b&study=fighters#panel-fighters
Work is local, uncommitted, and not published. Ready for the next visual
iteration; no new in-game art or native/hardware validation is claimed.

## 2026-09-08 — Chosen rental-cartridge presentation across the website

The approved Rental Cartridge / Fighter Roster direction is now the canonical
website presentation. A/C/D remain reviewable in the design workshop; ordinary
routes use B without overwriting the saved workshop choice. Shared masthead,
player bezel, editorial panels, journal index, build data, and 404 use ribbed
plastic, scuffed paper labels, native cartridge lettering, and recessed green
screens. Starlight guide, technical chapters, and journal articles use matching
chapter labels and reading panels, with an independent light paper mode.

The home page now includes the full fighter roster. Its base grid and centered
sprite-stage geometry moved from design-page CSS to component-owned `roster.css`
after full-page visual inspection caught the missing dependency. Source PNGs,
painted-bound crop data, and integer fighter scales remain unchanged. Browser
measurements now exercise both workshop and home instances at five widths.

`ScreenEffectsToggle.astro` and `screen-effects.ts` synchronize header/roster
controls and persist `smash-screen-fx` between pages and tabs. Pointer reflection
remains roster-local; reduced motion stops its moving effects. The emulator
canvas and gameplay code remain outside the effect stack. Desktop, mobile, and
light-mode inspection caught and corrected docs navigation overlap and low TOC
contrast. Tablet navigation uses the existing popup menu and releases the
reserved sidebar column.

Final validation: `make test-web` PASS, 21 tests with zero failed/skipped/flaky,
56 screenshots validated. `make ci-pages` PASS, with clear Astro diagnostics
and exact dist/site parity. The final summary's entry hash matches the staged
site and all screenshot references exist. The ROM hash remains
`daee33b394e3c52b4fd8fa66058a650b9497e459fb5a663714be2eb15e4d6625`.
Final full-page desktop/mobile captures were visually inspected, as were the
live in-app player and tablet docs. Header controls stay inside their frame at
320/390/799/900/1100/1201/1280/1440px. Tablet menu evidence:
`out/reports/sitewide-rental-tablet-menu.png`.

Logs: `out/reports/sitewide-rental-web-final.log` and
`out/reports/sitewide-rental-site-staging.log`. Summary and roster measurements:
`out/reports/web/summary.json`, `out/reports/web/roster-layout.json`, and
`out/reports/web/home-roster-layout.json`. Preview: http://127.0.0.1:8082/.
Work remains local and uncommitted; nothing was published. No new native or
physical-hardware validation is claimed for this website styling pass.

## 2026-09-08 — Five-page structure and simpler cartridge layouts

Reorganized the site around Home, Play, Book, Docs, and Blog. The shared main
navigation contains only Play / Book / Docs / Blog; the logo links home. Source,
ROM download, build information, and screen effects live in the footer. The
four-theme design workshop remains at `/design/` outside the main navigation.

The homepage is now the centered fighter cover with a Play action and short
links into the reading sections. It does not load EmulatorJS or the ROM. The
player moved to `/play/` with its existing input, touch, fullscreen, and download
behavior. Removed repeated roster prompts, promotional panels, and the extra
article navigation. Simplified the shared styles to worn labels, the approved
green cartridge materials, and a single reading surface; no change to the game
canvas or source fighter art.

The Book has nine ordered chapters spanning play and implementation. Docs has
four practical references: Build, Assets, Testing, and Website. Blog has three
dated articles with no chapter sidebar or pagination. Book and Docs sidebars and
previous/next links stay within their own section. Legacy guide, technical,
devlog, docs, and emulator URLs map to canonical routes while preserving query
strings and fragments. The new route map and content boundaries are documented
in both READMEs; asset examples were checked against the authored fighter data.

Visual review covered the live homepage, contents, articles, player, phone
layouts, and light reading mode. Fighter measurements still check actual painted
pixels, integer scales, centering, baseline alignment, clipping, and screen
effects at five widths on both the homepage and workshop. Touch capture uses
the viewport because full-page screenshots reset coarse-pointer emulation in
the installed Chromium build; the test verifies touch instructions before and
after capture.

Final validation: `make test-web` PASS, 24 tests with zero failed/skipped/flaky
and 22 validated screenshots. `make ci-pages` PASS with zero Astro errors,
warnings, or hints and exact `web/dist` / `site` parity. The ROM SHA-256 remains
`daee33b394e3c52b4fd8fa66058a650b9497e459fb5a663714be2eb15e4d6625`.
Logs: `out/reports/simple-site-web-final.log` and
`out/reports/simple-site-staging.log`; reports: `out/reports/web/summary.json`,
`out/reports/web/playwright.json`, and `out/reports/web/screens/`.
Local preview: http://127.0.0.1:8082/. Changes remain uncommitted and unpublished.

## 2026-09-08 — Illustrated Book and hands-on lessons

Replaced the Book's plain contents opening with a worn instruction-book spread,
native-capture preview tabs, two parts, and direct links into the experiments.
The preview tabs support pointer and keyboard navigation; captions distinguish
the jab capture's actual missed kick from successful contact.

All nine chapters now include concrete practice prompts or short answer reveals.
Native captures illustrate the player chapters and rendering; cropped original
fighter portraits sit beside their respective profiles. Chapter numbers and part
labels keep the reading sequence visible. Existing routes, fighter anchors, and
the simplified site navigation remain intact.

Added a knockback experiment using the actual post-hit damage formula and
move-before-decay order from `apply_hit()` and `apply_fighter_physics()`. Damage
presets, a keyboard-accessible slider, step, and reset show the resulting push
and accumulated horizontal distance. The diagram uses the shipped Mario and
Rizer frames and explicitly excludes gravity, player control, and KO behavior.
Boundary checks cover the 16-damage step and 14-pixel speed cap; Book checks
exercise tabs, all experiment anchors, chapter navigation, answer reveals,
and narrow layouts. Visual review covered the booklet, native illustrations,
and experiment in the browser and desktop/mobile captures.

Final validation: `make test-web` PASS with 26 tests, zero failed/skipped/flaky,
and 26 validated screenshots. `make ci-pages` PASS; Astro reports zero errors,
warnings, or hints, and staged output matches `web/dist`. Additional light-mode
inspection is saved in `out/reports/book-knockback-light.png` and
`out/reports/book-fighter-light-mobile.png`; the fighter chapter has no horizontal
overflow at 320/390/900px. Logs: `out/reports/book-web-final.log` and
`out/reports/book-staging.log`. The ROM is unchanged. Work remains local and
uncommitted; the refreshed Book is at http://127.0.0.1:8082/book/.

## 2026-09-08 — Annotated instruction manual

Added an interactive NES controller to the Controls chapter, with worn paper,
handwritten arrows, depressed button states, keyboard labels, and Mario's original
action poses. The guide uses W for Up/jump, J for NES A/jab, and K for NES B/special.
Keyboard shortcuts stay inside the lesson; native Enter/Space activation and Tab
navigation remain available. The controller scales proportionally, including its
D-pad center, and the small-screen readout keeps prose below the pose.

Six native captures now have editable SVG arrows and circled details, with notes
stored in `web/src/data/manual-annotations.ts`. The overlay shares the capture's
coordinate box. Callouts point to the actual airborne Mario, missed kick, fireball,
selection frames, and damage counters. The Book cover previews the annotations;
First Match, Controls, and Combat include the full plates. A three-step diagram
shows press, release, and press again for the second jump. Caveat handwriting is
bundled locally with its OFL license. Original capture pixels and the ROM are unchanged.

Visual review covered the live cover/controller and desktop, phone, light, and
dark illustrations. The 320px review caught and corrected a D-pad center overlap
and a cramped readout. `make test-web` PASS: 28 tests, zero failed/skipped/flaky,
35 validated screenshots. `make ci-pages` PASS with zero Astro diagnostics and
exact `web/dist` / `site` parity. Logs: `out/reports/manual-web-final.log` and
`out/reports/manual-staging.log`; additional light-mode evidence is under
`out/reports/manual-*-light.png`. The ROM retains SHA-256
`daee33b394e3c52b4fd8fa66058a650b9497e459fb5a663714be2eb15e4d6625`.
Work remains local, uncommitted, and unpublished.

## 2026-09-08 — Real Nintendo controller and key overlays

Replaced the CSS controller approximation with Evan-Amos's public-domain,
front-on photograph of an original Nintendo NES controller. The unmodified
source and provenance live in `web/src/assets/controllers/`; Astro generates
local 640px/1280px WebP files (about 17/87 KB). Keyboard overlays sit at measured
photo coordinates, with interactive selection rings, accessible button names,
and the existing move guide. Removed the drawn shell, fake buttons, and loose
controller arrows. The photograph's Nintendo mark and printed A/B/Start/Select
labels remain visible.

Visual inspection covered desktop, 390px mobile, and 320px light mode. All eight
control centers remain clickable, and tests verify overlay alignment at four
widths, J/K mapping, native button activation, and focus escape. `make test-web`
PASS: 28 tests and 35 validated screenshots. `make ci-pages` PASS with exact
staging parity; ROM hash unchanged. Logs: `out/reports/controller-photo-web-final.log`
and `out/reports/controller-photo-staging.log`. Extra narrow light-mode evidence:
`out/reports/controller-photo-320-light.png`. No commit or publication.

## 2026-09-08 — Follow-along controller lessons

Turned the controller reference into four guided practice steps: Move, Jump,
Jab, and Special. Each has a short instruction, relevant controller highlights,
large practice keys, an annotated native game example, and a session-local
record of moves tried. Jump requires two fresh presses; repeated keydown while
W is held cannot complete the exercise. These are mapping/rhythm exercises,
not a running match. All photo overlays and their source coordinates remain intact.

`web/src/data/controller-lessons.ts` owns typed controls and lesson copy.
`ControlMovePreview.astro` shows closeups of unchanged native captures with
source-aligned ink. Next/Back focus the next instruction and scroll below the
fixed header without double-counting Starlight's scroll padding. Held-key
feedback clears on key release, focus escape, window blur, and hidden tabs.

Reorganized the Controls chapter around practice and a short first-match
checklist. Full mappings, menu controls, timing, touch/gamepad setup, and
troubleshooting remain available as expandable notes. The second-jump field
notes retain the illustrated key sequence and full annotated capture.

Live browser interaction covered movement and the double-press exercise.
Desktop/mobile/light-mode review included 320px layout and keyboard/overlay
alignment. `make test-web` PASS: 29 tests, zero failed/skipped/flaky, and 37
validated screenshots. `make ci-pages` PASS with exact staging parity and
zero Astro diagnostics. Logs: `out/reports/controller-lessons-web-final.log`
and `out/reports/controller-lessons-staging.log`. Extra evidence:
`out/reports/controller-lessons-320-light.png` and
`out/reports/controller-lessons-desktop-light.png`. ROM unchanged; no commit
or publication.

## 2026-09-08 — Direct controller sheet

Replaced the guided Controls wizard with four immediately available move buttons.
Removed the step numbers, tried counters, Next/Back navigation, repeated prompts,
and extra instruction headings. The real Nintendo controller, measured keyboard
overlays, and annotated native game captures remain. Move buttons and photo
controls share keyboard/press feedback; W still needs a fresh second press.
The Play link is always visible. Ink draws in briefly when an example changes,
with reduced-motion support. Chapter copy and reference headings are shorter.

Verified native button activation, focus escape, double-jump rhythm, touch target
sizes, photo/annotation alignment, and zero overflow at 320/390/799/1440px.
Live in-app interaction and desktop/mobile/light-mode screenshots were visually
reviewed, including a narrow-label fix at 320px. `make test-web` PASS: 29 tests,
37 validated screenshots; `make ci-pages` PASS with exact local staging parity.
Astro reports zero errors, warnings, or hints. ROM hash unchanged. Evidence:
`out/reports/controller-direct-web-final.log`, `controller-direct-staging.log`,
and `controller-direct-{320,390,1440}-light.png`. No commit or publication.

## 2026-09-08 — Printed controls spread

Rebuilt Controls as a single worn-paper spread. The real NES controller now has
four permanently visible callouts and source-aligned ink leaders. Removed the
move selector grid and dynamic lesson state; clicking or typing still highlights
the corresponding physical button. Recovery and missed-jab captures remain
visible together, with short handwritten captions. The keyboard mapping data is
now `web/src/data/controller-controls.ts`.

Controls-only layout overrides remove the nested article frame and title plaque;
`tableOfContents: false` removes the extra contents rail while retaining site and
chapter navigation. Wide screens put the controller beside the two examples;
narrow screens stack them. Reference notes use plain rules instead of boxes.

Live in-app interaction checked W/J/K mapping. Visual review covered 320px,
390px, desktop, and light mode. Screenshot capture explicitly awaits responsive
photo decoding. `make test-web` PASS: 29 tests, 37 validated screenshots;
`make ci-pages` PASS with exact local staging parity. Astro has zero diagnostics;
ROM SHA-256 is unchanged. Logs: `out/reports/manual-spread-web-final.log` and
`manual-spread-staging.log`. Visuals: `manual-spread-{320,390,1440}.png` and
`manual-spread-1440-light.png`. No commit or publication.

## 2026-09-08 — Fight-zine controls

Pushed the printed Controls spread into a louder photocopy collage: oversized
yellow/red lettering, an original Mario sprite cutout from the roster sheet,
P1 burst, stamped key labels, marker highlights, and heavier controller ink.
The two native captures now have crooked taped paper edges and UP!/WHIFF!
stamps; all four control descriptions remain visible together. Pressing a key
adds a brief ring around its physical button, with no animation under reduced
motion. Photo/overlay coordinates remain unchanged.

Visually reviewed 320px, 390px, desktop, and light/reduced-motion views. Confirmed
zero reduced-motion key animations. `make test-web` PASS: 29 tests and 37 validated
screenshots. `make ci-pages` PASS with exact local staging parity. Logs:
`out/reports/manual-zine-web-final.log`, `manual-zine-staging.log`; extra visuals:
`manual-zine-{320,390,1440}.png` and `manual-zine-light-reduced.png`. No commit or
publication.

## 2026-09-08 — Harder Xerox manual

Changed the Controls surface from warm paper to scratched charcoal with a torn
pale controller sheet, warning-yellow strip, and red print. Added editable
`web/public/assets/manual-ink-wear.svg` for the page wear. Original Mario and
Rizer roster cutouts face each other in the wide masthead; narrow layouts keep
Mario. Screenshot clippings use rough black backings, yellow tape, dirty-paper
captions, and red stamps. The real controller photograph, native captures,
source-coordinate overlays, and four visible instructions remain intact.

Live in-app W/J/K interaction, 320px/390px/desktop screenshots, and light/reduced
motion views were reviewed. Reduced motion produced zero key-ring animations.
`make test-web` PASS: 29 tests and 37 validated screenshots. `make ci-pages`
PASS with exact local staging parity. Logs: `out/reports/manual-hard-web-final.log`
and `manual-hard-staging.log`; visuals: `manual-hard-{320,390,1440}.png` and
`manual-hard-light-reduced.png`. No commit or publication.

## 2026-09-08 — Grimy, handled manual

Added static, editable paper and ink textures to the Controls spread: greasy
edge smudges, water stains, rubbed folds, chipped lettering, and old adhesive.
`manual-paper-grime.svg` and `manual-type-wear.svg` join the revised
`manual-ink-wear.svg`; wear stays in the material backgrounds. Screenshot
clippings now have dull yellowed tape and stained caption paper. No copy,
controller coordinates, source photographs, or native screenshots changed.
The headline retains visible text in forced-colors mode.

Reviewed the live in-app manual and clicked/pressed W, J, and K. Reviewed
320px/390px/1440px captures and light/reduced-motion rendering: zero horizontal
overflow, zero button animations under reduced motion. Controller photograph
SHA256 remains `0ac5f45fe5180e2e914dbfe2ec38e374d38c998f5c9e1b2337bec65daf40d21e`.
`make test-web` PASS: 29 tests and 37 validated screenshots; `make ci-pages`
PASS with exact local staging parity. Logs: `out/reports/manual-grime-web-final.log`
and `manual-grime-staging.log`; visuals: `manual-grime-{320,390,1440}.png` and
`manual-grime-light-reduced.png`. No commit or publication.

## 2026-09-08 — Ripped rental-box manual

Pushed the user's request for a much crazier, dirtier Controls spread into a
torn red masthead, distressed cream type, fighter impact cutout, and heavily
battered paper. Added generated `web/src/assets/manual/battered-paper.png`,
with provenance in the neighboring README; Astro builds a 1200px WebP (340KB).
The texture supplies heavy toner marks, fingerprints, creases, and edge damage.
It is decorative generated material, not a historical scan. The real Nintendo
controller photograph and native game captures remain unchanged.

Controller instructions use pale scraps for readability. Screenshot clippings
have separate torn backings, crooked stamps, ripped tape, and readable caption
scraps sampling the center of the same texture. No interaction or mapping
changes. Reviewed 320px, 390px, 1440px, and light/reduced-motion renders and
exercised W/J/K in the in-app browser. Zero horizontal overflow; reduced motion
produced zero button animations. `make test-web` PASS (29 tests, 37 validated
screenshots); `make ci-pages` PASS with exact local staging parity.
Logs: `out/reports/manual-ripped-web-final.log`, `manual-ripped-staging.log`.
Visuals: `manual-ripped-{320,390,1440}.png`, `manual-ripped-light-reduced.png`.
No commit or publication.

## 2026-09-08 — One ordered Book

Consolidated all nine Book chapters at `/book/`: first match, controls, fighters,
combat, architecture, physics, contact, rendering, and audio. The page renders
existing MDX sources in sidebar order with one H1, numbered H2 chapter headings,
and H3 lessons. Desktop contents track the current chapter; the phone contents
menu closes after a jump. Search results jump to the matching lesson, including
content inside expandable details. Previous chapter, guide, and technical URLs
redirect to the Book while preserving incoming queries and anchors.

Removed duplicate setup instructions and shortened player-facing prose. Kept
the real Nintendo controller photograph, keyboard overlays, original sprites,
native captures, and battered paper treatment. Made reading columns quieter,
placed annotations beside captures on wide screens, and stacked them on phones.
The build pipeline now stacks vertically at narrow widths, and the trajectory
slider label no longer squeezes into a thin column. Fighter portraits retain
integer sprite scaling. Fixed doubled anchor offsets and chapter highlighting.

Visually reviewed every chapter on desktop and representative sections at 320px
and 390px, then inspected the live in-app Book. No page overflow; chapter jumps
land below the fixed header and contents. Exercised the controller, search,
contents, redirects, trajectory lab, and animation viewer through browser tests.
`make test-web` PASS: 29 tests and 37 validated screenshots. `make ci-pages`
PASS with exact local staging parity. Logs: `out/reports/book-single-web-final.log`
and `book-single-staging.log`; visuals: `book-single-{320,390,1440}-*.png`.
No commit or publication.

## 2026-09-08 — Essential Book and reproducible screenshots

Shortened `/book/` to four illustrated chapters: first match, controls,
fighters, and combat. Moved the five engine chapters and interactive labs to
Docs, preserving old chapter URLs, queries, and heading links. Kept the
battered manual treatment and real Nintendo controller photograph. Fighter
portraits no longer squeeze their annotated screenshots into narrow columns.

Added an input-only MAME capture driver with game-state assertions and 14
native frames. The Book shows the actual jump press/release/press sequence,
a jab connecting for 6%, and all four specials. Editable SVG annotations use
shared native image coordinates; the original capture pixels stay intact.

`make manual-refresh`, `manual-review`, `manual-accept`, and `manual-check`
provide a capture, visual review, and promotion recipe documented in
`docs/manual-refresh.md`. Builds reject stale ROM, capture-driver, annotation,
or image hashes. Five fixture tests exercise freshness and tampering failures.
Pinned Node 24.20.0 for Cloudflare Pages to match the GitHub workflow.

Visually inspected native captures and annotated layouts at 320px, 390px,
and 1440px, plus the live Book. No horizontal overflow. Final `make test`
PASS: 22 Rust tests, doctests, 16 MAME cases, and 29 browser tests with 39
validated screenshots. `make test-fceux`, `make ci-pages`, Rust formatting,
and `git diff --check` PASS. Logs: `out/reports/manual-release-full-final.log`,
`manual-release-fceux.log`, and `manual-release-staging.log`.
The user authorized committing, pushing, and deploying this accumulated
website release; production is deployed by Cloudflare Pages from `main`.
