# Automation

Automation is not an add-on in this project. It is the primary way we trust a build.

## 1. Automation Philosophy

Every critical action should run headlessly:

- generation
- ROM build
- ROM validation
- emulator behavior checks
- web bundle checks

If a task cannot run in CI, it is considered incomplete.

## 2. Emulator Suite

`nessy test emulator` runs scripted scenarios from `nessy/tools/emulator/suite/`.
The suite validates boot health, frame progression, input response, and long-run stability.

Outputs:
- text reports in `out/reports/emulator/`
- aggregate summary in `out/reports/emulator/summary.json`

Use this suite when gameplay behavior changes or data contracts move.

## 3. Web Suite

`make test-web` validates the publish surface:

1. regenerate and sync web assets
2. build the Dioxus bundle
3. serve locally
4. verify routes and required page markers
5. validate ROM header bytes
6. capture screenshots (or deterministic placeholders when headless browser capture is unavailable)

Outputs:
- `out/reports/web/summary.json`
- `out/reports/web/screens/*.png`
- `out/reports/web/http_server.log`

For local Pages parity, run `make ci-pages` to build and stage a deployable static site in `site/`.

## 4. Fast Failure Debugging

When automation fails:

1. run the failing command in isolation (`make test-emu` or `make test-web`)
2. open the JSON report first, then case-level logs
3. verify whether failure is data, runtime logic, or packaging
4. fix at the lowest layer that actually caused the break

This keeps turnaround fast and avoids papering over contract failures.
