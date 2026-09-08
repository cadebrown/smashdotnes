# Current work

Objective: preserve and publish the validated prototype, then improve existing
gameplay feel, reliability, and polish. Keep new match modes and roster expansion
outside this pass.

## Cleanup checkpoint

- Existing four-fighter art, animation, music, native automation, and static web
  delivery overhaul is validated locally and ready for the cleanup checkpoint.
- Browser focus loss now releases held, currently mapped keyboard controls.
  Regression coverage includes keyboard navigation, window blur, hidden tabs,
  remapping, speed controls, and focus within the emulator toolbar.
- Corrected the documented NES A/jab and B/special names.
- Added clean-runner CI for Rust, native emulators, browser play, and site staging.
- Cleanup gate: 21 Rust tests, 13 MAME scenarios, FCEUX boot, all eight browser
  tests, format/typo checks, workflow lint, and exact site staging pass locally.

## Next quality pass

Reproduce and fix platform landing from below, simultaneous-melee player-order
bias, and CPU offstage recovery. Validate through controller-driven native
scenarios, inspect captures, and retain the strict 60 Hz combat check.

Evidence is generated under `out/reports/`. Local tests do not establish physical
hardware behavior or public deployment. Remote CI must be observed after push.
