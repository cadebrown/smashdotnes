import { expect, test } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const reportDir = fileURLToPath(new URL("../../out/reports/web/", import.meta.url));
const screenDir = path.join(reportDir, "screens");
const viewports = [320, 390, 900, 1440];
const fighters = ["mario", "simon", "tommy", "rizer"];
const expectedAssets = [
  "/assets/captures/03_battle.png",
  ...fighters.map((fighter) => `/assets/fighters/${fighter}.png`)
];

async function selectStyle(page, style) {
  await page.getByRole("tab", { name: style[0].toUpperCase() + style.slice(1), exact: true }).click();
  await expect(page.locator(`[data-style-panel=${style}]`)).toBeVisible();
}

async function settleForCapture(page) {
  const motion = page.locator("[data-style-motion-toggle]");
  if (await motion.getAttribute("aria-pressed") !== "false") await motion.click();
  await expect(motion).toHaveAttribute("aria-pressed", "false");
  await page.evaluate(() => document.fonts.ready);
}

test("styles directions preserve navigation, motion, assets, sprite crops, and responsive layouts", async ({ page }) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  await mkdir(screenDir, { recursive: true });

  const response = await page.goto("/styles/");
  expect(response?.ok()).toBe(true);
  const tabs = page.getByRole("tab");
  await expect(tabs).toHaveCount(3);
  await expect(page.getByRole("tab", { name: "Hardware", exact: true })).toHaveAttribute("aria-selected", "true");

  await page.getByRole("tab", { name: "Hardware", exact: true }).focus();
  await page.keyboard.press("ArrowRight");
  await expect(page.getByRole("tab", { name: "Midnight", exact: true })).toHaveAttribute("aria-selected", "true");
  await expect(page).toHaveURL(/\/styles\/\?style=midnight$/);
  await page.keyboard.press("End");
  await expect(page.getByRole("tab", { name: "World", exact: true })).toHaveAttribute("aria-selected", "true");
  await expect(page).toHaveURL(/\/styles\/\?style=world$/);

  await selectStyle(page, "midnight");
  await page.reload();
  await expect(page.getByRole("tab", { name: "Midnight", exact: true })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("[data-style-panel=midnight]")).toBeVisible();

  const motion = page.locator("[data-style-motion-toggle]");
  await expect(motion).toHaveAttribute("aria-pressed", "true");
  await motion.click();
  await expect(motion).toHaveAttribute("aria-pressed", "false");
  await page.reload();
  await expect(motion).toHaveAttribute("aria-pressed", "false");
  await motion.click();
  await expect(motion).toHaveAttribute("aria-pressed", "true");
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect.poll(() => page.locator(".midnight-orbit").evaluate((element) => getComputedStyle(element).animationName)).toBe("none");
  await page.emulateMedia({ reducedMotion: "no-preference" });

  for (const fighter of fighters) {
    await page.getByRole("button", { name: `Preview ${fighter[0].toUpperCase() + fighter.slice(1)}` }).click();
    const contender = page.locator(`[data-midnight-contender=${fighter}]`);
    await expect(contender).toBeVisible();
    const crop = await contender.locator(".midnight-sprite").evaluate((element) => {
      const style = getComputedStyle(element);
      const number = (name) => Number(style.getPropertyValue(name));
      const rect = element.getBoundingClientRect();
      return {
        width: rect.width, height: rect.height,
        expectedWidth: number("--crop-width") * number("--scale"),
        expectedHeight: number("--crop-height") * number("--scale")
      };
    });
    expect(crop.width, `${fighter} crop width`).toBeCloseTo(crop.expectedWidth, 5);
    expect(crop.height, `${fighter} crop height`).toBeCloseTo(crop.expectedHeight, 5);
  }

  const layout = [];
  for (const style of ["hardware", "midnight", "world"]) {
    await selectStyle(page, style);
    for (const width of viewports) {
      await page.setViewportSize({ width, height: 900 });
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
      expect(overflow, `${style} horizontal page overflow at ${width}px`).toBeLessThanOrEqual(1);
      layout.push({ style, width, overflow });
    }
  }

  for (const asset of expectedAssets) {
    const assetResponse = await page.request.get(asset);
    expect(assetResponse.ok(), `${asset} loaded`).toBe(true);
  }
  expect(await page.locator("img").evaluateAll((images) => images.every((image) => image.complete && image.naturalWidth > 0))).toBe(true);

  await page.setViewportSize({ width: 1440, height: 900 });
  for (const style of ["hardware", "midnight", "world"]) {
    await selectStyle(page, style);
    await settleForCapture(page);
    await page.screenshot({ path: path.join(screenDir, `styles-${style}-desktop.png`), fullPage: true });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.screenshot({ path: path.join(screenDir, `styles-${style}-mobile.png`), fullPage: true });
    await page.setViewportSize({ width: 1440, height: 900 });
  }

  for (const style of ["hardware", "midnight", "world"]) {
    await selectStyle(page, style);
    const links = page.locator(`[data-style-panel=${style}] a[href='/play/']`);
    expect(await links.count(), `${style} has a Play link`).toBeGreaterThan(0);
    await expect(links.first()).toHaveAttribute("href", "/play/");
  }

  await writeFile(path.join(reportDir, "styles-layout.json"), `${JSON.stringify({ viewports: layout, assets: expectedAssets, errors }, null, 2)}\n`);
  expect(errors).toEqual([]);
});
