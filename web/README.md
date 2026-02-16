# web

This crate hosts the publish-facing web surface for SmashDotNES.

It includes:
- landing page and docs links (Dioxus app)
- ROM distribution endpoint (`/rom/smash.nes`)
- playable browser emulator via EmulatorJS (`/play`)
- synced technical docs with generated HTML (`/docs`)
- build metadata surface (`/info`)
- screenshot/static asset surface (`/snaps/*`)

## Development

From repository root:

```bash
make web-sync
make web-dev
```

Compatibility note:
- Use `dx` `0.7.2` to match `dioxus = 0.7.2` in `web/Cargo.toml`.

## Build

```bash
dx build --platform web --package smash-web
```

For Cloudflare Pages artifact staging from repo root:

```bash
make ci-pages
```

## Automation

Use the root target:

```bash
make test-web
```

This performs sync, build, endpoint checks, ROM header check, and screenshots.
If headless screenshot capture is unavailable in the environment, the suite writes deterministic placeholder PNGs and still validates routes/ROM data.

## Routes

- `/` landing page
- `/play` web emulator
- `/docs` docs index
- `/info` build metadata and links

## Browser Controls

- `WASD`: move
- `K`: A
- `J`: B
- `Enter`: Start
- `'`: Select
