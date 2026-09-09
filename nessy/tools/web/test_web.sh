#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SCREEN_DIR="$ROOT/out/reports/web/screens"
rm -f "$ROOT/out/reports/web/summary.json"

echo "[web-test] Syncing the built ROM, native captures, docs, and metadata"
(cd "$ROOT" && make web-sync)

echo "[web-test] Installing the pinned browser-test package"
npm --prefix "$ROOT/web" ci --ignore-scripts --no-audit --no-fund
(cd "$ROOT/web" && npx --no-install playwright install chromium)

echo "[web-test] Checking and building the Astro/Starlight site"
npm --prefix "$ROOT/web" run check
npm --prefix "$ROOT/web" run build

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

# Keep report serialization out of a shell here-document: Bash can block while
# writing the expanded document to its internal pipe on this macOS host.
node "$ROOT/web/scripts/write-validation-summary.mjs"
