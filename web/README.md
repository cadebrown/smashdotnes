# Web player

`web/public` is both the source and the deployment tree for the Smash.NES site. It is plain HTML/CSS/JavaScript around a vendored EmulatorJS runtime; there is no framework build or debug overlay.

From the repository root:

```bash
make web-dev
make test-web
make ci-pages
```

`make web-dev` rebuilds and syncs the ROM, docs, metadata, and available native captures, then serves the static tree on port 8080. `make ci-pages` copies that exact tree into `site/`.

The Playwright gate in `web/tests/site.spec.mjs` verifies:

- desktop and 390px layouts without horizontal overflow
- loaded, non-placeholder imagery
- generated manual and build provenance routes
- a valid downloadable iNES ROM
- no external browser requests or console errors
- EmulatorJS reaches its running state
- real Start input changes the rendered canvas
- Tab escapes the emulator focus surface

Screenshots and traces land under `out/reports/web/`. The Rust screenshot inspector then rejects missing, undersized, uniform, and 1x1 fallback images.

## Routes

- `/`
- `/play/`
- `/docs/`
- `/info/`
- `/rom/smash.nes`

## Controls

- `WASD`: move
- `J`: A / jab
- `K`: B / special
- `Enter`: Start
- `'`: Select
