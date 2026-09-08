import { expect, test } from "@playwright/test";
import { createHash } from "node:crypto";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const screenshotDir = fileURLToPath(
  new URL("../../out/reports/web/screens/", import.meta.url)
);

function observePage(page) {
  const errors = [];
  const externalRequests = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (["http:", "https:"].includes(url.protocol) && url.hostname !== "127.0.0.1") {
      externalRequests.push(request.url());
    }
  });
  return { errors, externalRequests };
}

function digest(buffer) {
  return createHash("sha256").update(buffer).digest("hex");
}

test.beforeAll(async () => {
  await mkdir(screenshotDir, { recursive: true });
});

test("landing page is complete on desktop and mobile", async ({ page }) => {
  const observed = observePage(page);
  const response = await page.goto("/");
  expect(response?.ok()).toBe(true);
  await expect(page).toHaveTitle(/Smash\.NES/);
  await expect(page.getByRole("heading", { level: 1 })).toContainText("Four legends");
  await expect(page.getByRole("link", { name: "Power on" })).toBeVisible();

  const titleImage = page.locator(".console-screen img");
  await expect(titleImage).toBeVisible();
  await expect.poll(() => titleImage.evaluate((image) => image.naturalWidth)).toBeGreaterThanOrEqual(256);
  await page.screenshot({ path: path.join(screenshotDir, "home-desktop.png"), fullPage: true });

  await page.setViewportSize({ width: 390, height: 844 });
  await page.reload();
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
  expect(overflow).toBeLessThanOrEqual(1);
  await page.screenshot({ path: path.join(screenshotDir, "home-mobile.png"), fullPage: true });

  expect(observed.externalRequests).toEqual([]);
  expect(observed.errors).toEqual([]);
});

test("landing headline fits with fallback fonts at narrow and desktop widths", async ({ page }) => {
  await page.goto("/");
  // Linux runners lack Impact; Arial reproduces the wider fallback on macOS too.
  await page.addStyleTag({ content: ".hero-copy h1 { font-family: Arial, sans-serif; }" });

  for (const width of [320, 390, 901, 1280, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    const overflow = await page.evaluate(() => {
      const heading = document.querySelector(".hero-copy h1");
      return {
        page: document.documentElement.scrollWidth - innerWidth,
        heading: heading.scrollWidth - heading.clientWidth
      };
    });
    expect(overflow.page, `page overflow at ${width}px`).toBeLessThanOrEqual(1);
    expect(overflow.heading, `headline overflow at ${width}px`).toBeLessThanOrEqual(1);
    if (width === 390 || width === 1440) {
      await page.screenshot({
        path: path.join(screenshotDir, `home-fallback-${width}.png`),
        fullPage: true
      });
    }
  }
});

test("manual and build provenance routes render real data", async ({ page }) => {
  const observed = observePage(page);
  let response = await page.goto("/docs/");
  expect(response?.ok()).toBe(true);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("Manual");
  await expect(page.getByRole("link", { name: "Architecture" })).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "manual.png"), fullPage: true });

  response = await page.goto("/info/");
  expect(response?.ok()).toBe(true);
  await expect(page.locator(".kv-row")).toHaveCount(8);
  await expect(page.locator("#snapshot")).toContainText("SmashDotNES");
  const metadata = await page.request.get("/info/build.json");
  expect(metadata.ok()).toBe(true);
  await expect(metadata.json()).resolves.toMatchObject({
    project: "SmashDotNES",
    rom_path: "/rom/smash.nes"
  });
  await page.screenshot({ path: path.join(screenshotDir, "build.png"), fullPage: true });

  const rom = await page.request.get("/rom/smash.nes");
  expect(rom.ok()).toBe(true);
  expect(Array.from((await rom.body()).subarray(0, 4))).toEqual([0x4e, 0x45, 0x53, 0x1a]);
  expect(observed.externalRequests).toEqual([]);
  expect(observed.errors).toEqual([]);
});

test("vendored emulator boots, advances, accepts input, and releases focus", async ({ page }) => {
  const observed = observePage(page);
  const response = await page.goto("/play/", { waitUntil: "domcontentloaded" });
  expect(response?.ok()).toBe(true);

  await expect.poll(
    () => page.evaluate(() => Boolean(window.EJS_emulator?.started)),
    { timeout: 60_000 }
  ).toBe(true);
  await expect(page.locator("#status")).toHaveText("Running at 60 Hz");

  const canvas = page.locator("#game canvas").first();
  await expect(canvas).toBeVisible();
  const dimensions = await canvas.evaluate((element) => ({
    width: element.width,
    height: element.height
  }));
  expect(dimensions.width).toBeGreaterThanOrEqual(256);
  expect(dimensions.height).toBeGreaterThanOrEqual(240);

  await page.waitForTimeout(750);
  const before = await canvas.screenshot();
  await canvas.click({ position: { x: 20, y: 20 } });
  await page.keyboard.down("Enter");
  await page.waitForTimeout(140);
  await page.keyboard.up("Enter");
  await page.waitForTimeout(900);
  const after = await canvas.screenshot();
  expect(digest(after)).not.toBe(digest(before));

  await page.evaluate(() => document.activeElement?.blur());
  await page.screenshot({ path: path.join(screenshotDir, "play-running.png"), fullPage: true });

  await page.locator("#game").focus();
  expect(await page.evaluate(() => document.activeElement?.id)).toBe("game");
  await page.keyboard.press("Tab");
  expect(await page.evaluate(() => document.activeElement?.id)).not.toBe("game");

  expect(observed.externalRequests).toEqual([]);
  expect(observed.errors).toEqual([]);
});
