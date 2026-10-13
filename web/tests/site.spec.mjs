import { devices, expect, test } from "@playwright/test";
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

test("home introduces the fighters and sends Play to the live player", async ({ page }) => {
  const observed = observePage(page);
  const response = await page.goto("/");
  expect(response?.ok()).toBe(true);
  await expect(page).toHaveTitle(/Smash\.NES/);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await expect(page.locator("#game")).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => {
    return !window.EJS_emulator && ![...document.scripts].some(script => /emulatorjs|emulator\.min/i.test(script.src));
  })).toBe(true);
  const roster = page.locator("[data-home-arena]");
  await expect(roster.locator("[data-roster-fighter]")).toHaveCount(4);
  await roster.evaluate(async element => {
    await Promise.all(element.getAnimations({ subtree: true }).map(animation => animation.finished));
  });
  await page.screenshot({ path: path.join(screenshotDir, "home-desktop.png") });

  for (const width of [320, 390, 901, 1280, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth),
      `page overflow at ${width}px`).toBeLessThanOrEqual(1);
    await expect(page.getByRole("link", { name: "Play Smash.NES", exact: true })).toBeInViewport();
  }
  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: path.join(screenshotDir, "home-mobile.png") });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.getByRole("link", { name: "Play", exact: true }).first().click();
  await expect(page).toHaveURL(/\/play\/$/);
  await expect.poll(() => page.evaluate(() => Boolean(window.EJS_emulator?.started)), { timeout: 60_000 }).toBe(true);
  expect(observed.externalRequests).toEqual([]);
  expect(observed.errors).toEqual([]);
});

test("the player stays canonical at /play and the emulator alias redirects there", async ({ page }) => {
  await page.goto("/play/");
  await expect(page).toHaveURL(/\/play\/$/);
  await expect(page.locator("#game")).toBeVisible();
  await page.goto("/emulator/?from=legacy#controls");
  await expect(page).toHaveURL(/\/play\/\?from=legacy#controls$/);
  await expect(page.locator("#game")).toBeVisible();
});

test("reference pages render real build data", async ({ page }) => {
  const observed = observePage(page);
  let response = await page.goto("/docs/");
  expect(response?.ok()).toBe(true);
  await expect(page).toHaveURL(/\/docs\/$/);
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
  await page.screenshot({ path: path.join(screenshotDir, "docs-index.png") });

  await page.goto("/book/architecture/");
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();

  response = await page.goto("/info/");
  expect(response?.ok()).toBe(true);
  await expect(page.locator(".build-row")).toHaveCount(5);
  await page.getByText("View raw build data", { exact: true }).click();
  await expect(page.locator(".raw-json")).toContainText("SmashDotNES");
  const metadata = await page.request.get("/info/build.json");
  expect(metadata.ok()).toBe(true);
  await expect(metadata.json()).resolves.toMatchObject({
    project: "SmashDotNES",
    rom_path: "/rom/smash.nes"
  });
  expect((await metadata.json()).git).not.toHaveProperty("remote");
  await page.screenshot({ path: path.join(screenshotDir, "build.png") });

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
  await expect(page.locator("#status")).toHaveText("Game ready");

  const canvas = page.locator("#game canvas").first();
  await expect(canvas).toBeVisible();
  const dimensions = await canvas.evaluate((element) => ({
    width: element.width,
    height: element.height
  }));
  expect(dimensions.width).toBeGreaterThanOrEqual(256);
  expect(dimensions.height).toBeGreaterThanOrEqual(240);

  for (const width of [1440, 390, 320]) {
    await page.setViewportSize({ width, height: 844 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth),
      `player overflow at ${width}px`).toBeLessThanOrEqual(1);
    await page.locator("#game").hover();
    // EmulatorJS redraws its toolbar after its 250ms resize debounce.
    await page.waitForTimeout(350);
    for (const name of ["Restart", "Pause", "Control Settings", "Mute", "Enter Fullscreen"]) {
      await expect.poll(() => page.getByRole("button", { name, exact: true }).evaluate(button => {
        const bounds = button.getBoundingClientRect();
        const game = document.getElementById("game").getBoundingClientRect();
        return bounds.left >= game.left && bounds.right <= game.right;
      }), { message: `${name} stays inside the player at ${width}px` }).toBe(true);
    }
  }
  await page.setViewportSize({ width: 1440, height: 900 });

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

  await page.locator("#game").hover();
  await page.getByRole("button", { name: "Enter Fullscreen", exact: true }).click();
  await expect.poll(() => page.evaluate(() => document.fullscreenElement?.id)).toBe("game");
  const fullscreenVideo = await canvas.boundingBox();
  expect(fullscreenVideo.height / fullscreenVideo.width).toBeCloseTo(3 / 4, 2);
  await page.locator("#game").hover();
  await page.getByRole("button", { name: "Exit Fullscreen", exact: true }).click();
  await expect.poll(() => page.evaluate(() => Boolean(document.fullscreenElement))).toBe(false);

  await page.locator("#game").focus();
  expect(await page.evaluate(() => document.activeElement?.id)).toBe("game");
  await page.keyboard.press("Tab");
  expect(await page.evaluate(() => document.activeElement?.id)).not.toBe("game");

  expect(observed.externalRequests).toEqual([]);
  expect(observed.errors).toEqual([]);
});

test.describe("touch player", () => {
  test.use({ isMobile: true, hasTouch: true, userAgent: devices["Pixel 7"].userAgent, viewport: { width: 390, height: 844 } });

  test("touch controls stay below the video and start the game", async ({ page }) => {
    const observed = observePage(page);
    await page.goto("/play/");
    await expect.poll(() => page.evaluate(() => Boolean(window.EJS_emulator?.started)), { timeout: 60_000 }).toBe(true);
    await page.waitForTimeout(1000);
    const canvas = page.locator("#game canvas").first();
    const controller = page.locator(".ejs_virtualGamepad_parent");
    const start = controller.getByText("Start", { exact: true });
    await expect(start).toBeVisible();
    await expect(page.locator(".touch-hint")).toBeVisible();
    await expect(page.locator(".keyboard-controls")).toBeHidden();

    for (const width of [320, 390]) {
      await page.setViewportSize({ width, height: 844 });
      const video = await canvas.boundingBox();
      expect(video.width).toBeGreaterThan(250);
      expect(video.height / video.width).toBeCloseTo(3 / 4, 2);
      for (const selector of [".ejs_virtualGamepad_left", ".ejs_virtualGamepad_right"]) {
        const pad = await page.locator(selector).boundingBox();
        expect(pad.y).toBeGreaterThanOrEqual(video.y + video.height - 1);
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
    }

    const before = await canvas.screenshot();
    const box = await start.boundingBox();
    const touch = await page.context().newCDPSession(page);
    await touch.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: box.x + box.width / 2, y: box.y + box.height / 2 }] });
    await page.waitForTimeout(140);
    await touch.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
    await page.waitForTimeout(600);
    expect(digest(await canvas.screenshot())).not.toBe(digest(before));
    await expect(page.locator(".touch-hint")).toBeVisible();
    // Full-page capture resets this Chromium build's coarse-pointer emulation.
    await page.screenshot({ path: path.join(screenshotDir, "play-touch.png") });
    await expect(page.locator(".touch-hint")).toBeVisible();
    await touch.detach();
    expect(observed.externalRequests).toEqual([]);
    expect(observed.errors).toEqual([]);
  });
});
