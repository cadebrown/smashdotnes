import { expect, test } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const screens = fileURLToPath(new URL('../../out/reports/web/screens/', import.meta.url));

test('controller manual maps browser keys to NES controls and keeps its keyboard focus local', async ({ page }) => {
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await mkdir(screens, { recursive: true });
  await page.goto('/book/controls/');
  const manual = page.locator('[data-controller-manual]');
  await expect(manual).toBeVisible();
  const photo = manual.getByRole('img', { name: /original Nintendo NES controller/ });
  await expect(photo).toBeVisible();
  await expect(photo).toHaveAttribute('src', /nes-controller-flat.*\.webp$/);

  await manual.focus();
  await page.keyboard.press('j');
  await expect(manual.locator('[data-control="a"]')).toHaveAttribute('aria-pressed', 'true');
  await expect(manual.locator('[data-readout]')).toContainText('NES A');
  await expect(manual.locator('[data-readout]')).toContainText('Jab');

  await page.keyboard.press('k');
  await expect(manual.locator('[data-control="b"]')).toHaveAttribute('aria-pressed', 'true');
  await expect(manual.locator('[data-readout]')).toContainText('NES B');
  await expect(manual.locator('[data-readout]')).toContainText('Special');

  await manual.getByRole('button', { name: /D-pad Left, A: Move left/ }).click();
  await expect(manual.locator('[data-control="left"]')).toHaveAttribute('aria-pressed', 'true');
  await expect(manual.locator('[data-readout]')).toContainText('Move left');
  await page.keyboard.press('j');
  await expect(manual.locator('[data-control="a"]')).toHaveAttribute('aria-pressed', 'true');

  await manual.getByRole('button', { name: 'NES A, J: Jab', exact: true }).focus();
  await page.keyboard.press('Enter');
  await expect(manual.locator('[data-control="a"]')).toHaveAttribute('aria-pressed', 'true');
  for (let step = 0; step < 12; step++) {
    await page.keyboard.press('Tab');
    if (await page.evaluate(() => !document.querySelector('[data-controller-manual]')?.contains(document.activeElement))) break;
  }
  expect(await page.evaluate(() => {
    const root = document.querySelector('[data-controller-manual]');
    return !root?.contains(document.activeElement);
  })).toBe(true);
  await page.keyboard.press('j');
  await expect(manual.locator('[data-control="a"]')).toHaveAttribute('aria-pressed', 'true');

  await manual.scrollIntoViewIfNeeded();
  for (const width of [320, 390, 799, 1440]) {
    await page.setViewportSize({ width, height: width < 600 ? 1200 : 1000 });
    await manual.scrollIntoViewIfNeeded();
    await photo.evaluate(image => image.decode());
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), `${width}px page overflow`).toBeLessThanOrEqual(1);
    const fits = await manual.locator('[data-control]').evaluateAll((controls) => {
      const image = document.querySelector('.controller-photo');
      const shell = image?.getBoundingClientRect();
      if (!(image instanceof HTMLImageElement) || !image.complete || !image.naturalWidth) return false;
      return !!shell && controls.every((control, index) => {
        const rect = control.getBoundingClientRect();
        const centered = Math.abs(rect.x + rect.width / 2 - (shell.x + shell.width * Number(control.dataset.photoX) / 3721)) < 1 &&
          Math.abs(rect.y + rect.height / 2 - (shell.y + shell.height * Number(control.dataset.photoY) / 1870)) < 1;
        const overlaps = controls.some((other, otherIndex) => {
          if (index === otherIndex) return false;
          const b = other.getBoundingClientRect();
          return Math.min(rect.right, b.right) - Math.max(rect.left, b.left) > 1 && Math.min(rect.bottom, b.bottom) - Math.max(rect.top, b.top) > 1;
        });
        return centered && !overlaps && rect.left >= shell.left - 1 && rect.right <= shell.right + 1 && rect.top >= shell.top - 1 && rect.bottom <= shell.bottom + 1;
      });
    });
    expect(fits, `${width}px key overlays stay aligned with the controller photo`).toBe(true);
    if (width === 1440) await page.screenshot({ path: path.join(screens, 'controller-manual-desktop.png') });
    if (width === 390) await page.screenshot({ path: path.join(screens, 'controller-manual-mobile.png') });
  }
  expect(errors).toEqual([]);
});

test('the printed manual exposes every move and both illustrations without stepping through UI', async ({ page }) => {
  await mkdir(screens, { recursive: true });
  await page.goto('/book/controls/#controller-manual-title');
  const manual = page.locator('[data-controller-manual]');
  await expect(manual.getByRole('link', { name: 'Play a match ↗', exact: true })).toHaveAttribute('href', '/play/');
  const notes = manual.locator('[data-note]');
  await expect(notes).toHaveCount(4);
  for (const note of await notes.all()) await expect(note).toBeVisible();
  await expect(manual.locator('[data-note="up"]')).toContainText('Tap. Release. Tap again.');
  const examples = manual.locator('[data-preview-scene]');
  await expect(examples).toHaveCount(2);
  for (const example of await examples.all()) await expect(example).toBeVisible();
  await expect(manual.locator('[data-preview-scene="jab"]')).toContainText('This kick misses.');

  await manual.locator('[data-practice="b"]').click();
  await expect(manual.locator('[data-readout]')).toContainText('NES B');
  await manual.locator('[data-practice="a"]').focus();
  await page.keyboard.press('Enter');
  await expect(manual.locator('[data-control="a"]')).toHaveAttribute('aria-pressed', 'true');
  await manual.focus();
  await page.keyboard.down('w');
  await expect(manual.locator('[data-control="up"]')).toHaveAttribute('data-held', 'true');
  await expect(manual.locator('[data-practice="up"]')).toHaveAttribute('data-held', 'true');
  await page.keyboard.up('w');
  await expect(manual.locator('[data-control="up"]')).toHaveAttribute('data-held', 'false');
  await expect(manual.locator('[data-practice="up"]')).toHaveAttribute('data-held', 'false');
  for (const note of await notes.all()) await expect(note).toBeVisible();
  for (const example of await examples.all()) await expect(example).toBeVisible();
  await page.screenshot({ path: path.join(screens, 'controller-spread.png') });

  for (const width of [320, 390, 799, 1440]) {
    await page.setViewportSize({ width, height: 1100 });
    await manual.evaluate(element => window.scrollBy(0, element.getBoundingClientRect().top - 145));
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), `manual spread at ${width}`).toBeLessThanOrEqual(1);
    expect(await manual.locator('[data-practice], [data-play]').evaluateAll(buttons => buttons.every(button => {
      const rect = button.getBoundingClientRect();
      return rect.height >= 44 && rect.width >= 44;
    })), `touch targets at ${width}`).toBe(true);
    const aligned = await examples.evaluateAll(elements => elements.every(element => {
      const image = element.querySelector('img').getBoundingClientRect();
      const ink = element.querySelector('svg').getBoundingClientRect();
      return ['x','y','width','height'].every(key => Math.abs(image[key]-ink[key]) < 1);
    }));
    expect(aligned, `source-aligned game annotations at ${width}`).toBe(true);
    if (width === 390) await page.screenshot({ path: path.join(screens, 'controller-spread-mobile.png') });
  }
  await manual.focus();
  await page.keyboard.down('a');
  await page.keyboard.down('d');
  await page.keyboard.up('a');
  await expect(manual.locator('[data-practice="right"]')).toHaveAttribute('data-held', 'true');
  await page.keyboard.up('d');
  await expect(manual.locator('[data-practice="right"]')).toHaveAttribute('data-held', 'false');
  await page.keyboard.down('d');
  await page.evaluate(() => window.dispatchEvent(new Event('blur')));
  await expect(manual.locator('[data-control="right"]')).toHaveAttribute('data-held', 'false');
  await expect(manual.locator('[data-practice="right"]')).toHaveAttribute('data-held', 'false');
  await page.keyboard.up('d');
});
