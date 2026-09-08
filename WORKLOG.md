# Current work

Objective: preserve and publish the validated prototype, then improve existing
gameplay feel, reliability, and polish. Keep new match modes and roster expansion
outside this pass.

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

The original ROM and regression reports remain under `out/reports/quality-before/`;
updated comparison evidence is under `out/reports/quality-after/`. Full-gate
results use the standard `out/reports/emulator/`, `fceux/`, and `web/` paths.

Local tests do not establish physical hardware behavior or public deployment.
Check the latest GitHub Validate run and Cloudflare Pages check for the current
commit before making a release claim; those external statuses are not stored here.
