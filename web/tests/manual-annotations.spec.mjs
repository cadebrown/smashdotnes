import { expect, test } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const screens = fileURLToPath(new URL('../../out/reports/web/screens/', import.meta.url));
const chapters = [['first-match', 3], ['controls', 3], ['fighters', 4], ['combat', 2]];
const screenshotWidths = [390, 1440];

test('manual annotations stay aligned with decoded native captures on desktop and mobile', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await mkdir(screens, { recursive: true });
  const screenshotFiles = [];
  for (const [route, count] of chapters) {
    await page.goto(`/book/#${route}`);
    const chapter = page.locator(`#${route}`);
    const plates = chapter.locator('.manual-plate');
    await expect(plates).toHaveCount(count);
    const undecoded = await page.locator('img').evaluateAll(async images => {
      images.forEach(image => { image.loading = 'eager'; });
    await Promise.all(images.map(image => image.decode().catch(() => undefined)));
      return images.filter(image => !image.complete || image.naturalWidth === 0 || image.naturalHeight === 0).map(image => image.currentSrc);
    });
    expect(undecoded, `${route} native captures decode`).toEqual([]);
    await page.evaluate(() => document.fonts.load('600 24px Caveat'));
    expect(await page.evaluate(() => document.fonts.check('600 24px Caveat'))).toBe(true);
    for (const width of [320, 390, 799, 1440]) {
      await page.setViewportSize({ width, height: 1000 });
      expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), `${route} at ${width}`).toBeLessThanOrEqual(1);
      const aligned = await plates.evaluateAll(elements => elements.every(element => {
        const image = element.querySelector('img');
        const overlay = element.querySelector('.manual-plate__ink');
        if (!image?.complete || image.naturalWidth !== 256 || image.naturalHeight !== 240 || !overlay) return false;
        const a = image.getBoundingClientRect(), b = overlay.getBoundingClientRect();
        return ['x', 'y', 'width', 'height'].every(key => Math.abs(a[key] - b[key]) < 1);
      }));
      expect(aligned, `${route} arrows match image coordinates at ${width}`).toBe(true);
      if (screenshotWidths.includes(width)) {
        const filename = `manual-${route}-${width}.png`;
        await plates.first().evaluate(element => window.scrollBy(0, element.getBoundingClientRect().top - 180));
        await page.screenshot({ path: path.join(screens, filename) });
        screenshotFiles.push(filename);
      }
    }
    if (route === 'controls') {
      const sequence = chapter.locator('.jump-sequence');
      const steps = sequence.locator(':scope > ol > li');
      await expect(steps).toHaveCount(3);
      const images = steps.locator('img');
      await expect(images).toHaveCount(3);
      expect(await images.evaluateAll(nodes => new Set(nodes.map(image => image.getAttribute('src'))).size)).toBe(3);
      await sequence.evaluate(element => window.scrollBy(0, element.getBoundingClientRect().top - 180));
      await page.screenshot({ path: path.join(screens, 'manual-jump-sequence.png') });
      screenshotFiles.push('manual-jump-sequence.png');
    }
    if (route === 'combat') {
      await expect(chapter.locator('[data-manual-scene="hit"]')).toContainText(/6%/);
    }
  }
  expect(screenshotFiles).toHaveLength(chapters.length * screenshotWidths.length + 1);
  expect(errors).toEqual([]);
});
