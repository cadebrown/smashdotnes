import { expect, test } from "@playwright/test";

async function bootInputProbe(page) {
  await page.goto("/play/");
  await expect.poll(
    () => page.evaluate(() => Boolean(window.EJS_emulator?.started)),
    { timeout: 60_000 }
  ).toBe(true);
  await page.evaluate(() => {
    const manager = window.EJS_emulator.gameManager;
    const simulateInput = manager.simulateInput.bind(manager);
    window.inputProbe = {};
    manager.simulateInput = (player, button, value) => {
      window.inputProbe[`${player}:${button}`] = value;
      return simulateInput(player, button, value);
    };
  });
  await page.locator("#game").focus();
}

for (const reason of ["keyboard navigation", "window blur", "hidden tab"]) {
  test(`releases held controls on ${reason}`, async ({ page }) => {
    await bootInputProbe(page);
    await page.keyboard.down("d");
    await page.keyboard.down("j");
    await expect.poll(() => page.evaluate(() => window.inputProbe)).toEqual({
      "0:7": 1, "0:8": 1
    });

    if (reason === "keyboard navigation") {
      // A plain Tab lands on the emulator toolbar, which still receives keyup.
      // Shift+Tab leaves the entire input parent and reproduces the lost release.
      await page.keyboard.press("Shift+Tab");
      expect(await page.evaluate(() =>
        window.EJS_emulator.elements.parent.contains(document.activeElement)
      )).toBe(false);
    } else if (reason === "window blur") {
      await page.evaluate(() => window.dispatchEvent(new Event("blur")));
    } else {
      // Exercise the browser's lifecycle callback without depending on whether
      // this headless browser throttles or hides background pages.
      await page.evaluate(() => {
        Object.defineProperty(document, "hidden", { configurable: true, value: true });
        document.dispatchEvent(new Event("visibilitychange"));
      });
    }

    await expect.poll(() => page.evaluate(() => window.inputProbe)).toEqual({
      "0:7": 0, "0:8": 0
    });
    await page.keyboard.up("d");
    await page.keyboard.up("j");
    await page.locator("#game").focus();
    await page.keyboard.down("d");
    await expect.poll(() => page.evaluate(() => window.inputProbe["0:7"])).toBe(1);
    await page.keyboard.up("d");
    await expect.poll(() => page.evaluate(() => window.inputProbe["0:7"])).toBe(0);
  });
}

test("focus release follows remapped controls and preserves toolbar input", async ({ page }) => {
  await bootInputProbe(page);
  await page.evaluate(() => { window.EJS_emulator.controls[0][7].value = 72; });
  await page.keyboard.down("h");
  await expect.poll(() => page.evaluate(() => window.inputProbe["0:7"])).toBe(1);
  await page.keyboard.press("Tab");
  expect(await page.evaluate(() =>
    window.EJS_emulator.elements.parent.contains(document.activeElement)
  )).toBe(true);
  expect(await page.evaluate(() => window.inputProbe["0:7"])).toBe(1);
  await page.getByRole("link", { name: "Build", exact: true }).focus();
  await expect.poll(() => page.evaluate(() => window.inputProbe["0:7"])).toBe(0);
  await page.keyboard.up("h");
});

test("releases held speed controls without replaying save commands", async ({ page }) => {
  await bootInputProbe(page);
  await page.evaluate(() => {
    window.EJS_emulator.controls[0][27].value = 72;
    window.EJS_emulator.controls[0][24].value = 72;
  });
  await page.keyboard.down("h");
  await expect.poll(() => page.evaluate(() => window.inputProbe["0:27"])).toBe(1);
  await page.keyboard.press("Shift+Tab");
  await expect.poll(() => page.evaluate(() => window.inputProbe["0:27"])).toBe(0);
  expect(await page.evaluate(() => window.inputProbe["0:24"])).toBe(1);
  await page.keyboard.up("h");
});
