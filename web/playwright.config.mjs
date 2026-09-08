import { defineConfig } from "@playwright/test";

const port = Number.parseInt(process.env.NESSY_WEB_TEST_PORT ?? "4173", 10);
if (!Number.isInteger(port) || port < 1 || port > 65_535) {
  throw new Error(`invalid NESSY_WEB_TEST_PORT: ${process.env.NESSY_WEB_TEST_PORT}`);
}
const baseURL = `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: "./tests",
  fullyParallel: false,
  workers: 1,
  timeout: 90_000,
  expect: { timeout: 15_000 },
  outputDir: "../out/reports/web/playwright",
  reporter: [
    ["line"],
    ["json", { outputFile: "../out/reports/web/playwright.json" }]
  ],
  use: {
    baseURL,
    browserName: "chromium",
    viewport: { width: 1440, height: 900 },
    permissions: ["screen-wake-lock"],
    trace: "retain-on-failure",
    screenshot: "only-on-failure"
  },
  webServer: {
    command: `uv run python -m http.server ${port} --bind 127.0.0.1 --directory public`,
    url: `${baseURL}/`,
    reuseExistingServer: false,
    stdout: "pipe",
    stderr: "pipe"
  }
});
