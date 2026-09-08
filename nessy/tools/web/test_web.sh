#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SCREEN_DIR="$ROOT/out/reports/web/screens"

echo "[web-test] Syncing the built ROM, native captures, docs, and metadata"
(cd "$ROOT" && make web-sync)

echo "[web-test] Installing the pinned browser-test package"
npm --prefix "$ROOT/web" ci --ignore-scripts --no-audit --no-fund
(cd "$ROOT/web" && npx --no-install playwright install chromium)

if [[ -z "${NESSY_WEB_TEST_PORT:-}" ]]; then
  NESSY_WEB_TEST_PORT="$(uv run python -c 'import socket; s = socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
  export NESSY_WEB_TEST_PORT
fi

rm -rf "$ROOT/out/reports/web/playwright" "$SCREEN_DIR"
mkdir -p "$SCREEN_DIR"

echo "[web-test] Driving desktop, mobile, provenance, and live emulator flows"
npm --prefix "$ROOT/web" test

echo "[web-test] Rejecting placeholder, blank, and undersized screenshots"
(cd "$ROOT" && cargo run --quiet --package nessy -- test screenshots --dir out/reports/web/screens)

ROM_SHA256="$(shasum -a 256 "$ROOT/out/smash.nes" | awk '{print $1}')"
cat > "$ROOT/out/reports/web/summary.json" <<JSON
{
  "status": "PASS",
  "rom_sha256": "$ROM_SHA256",
  "playwright_report": "out/reports/web/playwright.json",
  "screenshots": [
    "out/reports/web/screens/home-desktop.png",
    "out/reports/web/screens/home-mobile.png",
    "out/reports/web/screens/manual.png",
    "out/reports/web/screens/build.png",
    "out/reports/web/screens/play-running.png"
  ]
}
JSON

echo "[web-test] PASS: out/reports/web/summary.json"
