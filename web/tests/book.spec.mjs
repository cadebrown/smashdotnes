import { expect, test } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const screens = fileURLToPath(new URL('../../out/reports/web/screens/', import.meta.url));
const chapters = ['first-match', 'controls', 'fighters', 'combat'];

test('the Book keeps four player chapters, usable contents, and valid local anchors', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await mkdir(screens, { recursive: true });
  await page.goto('/book/');
  await expect(page.getByRole('heading', { level: 1, name: 'The Book' })).toBeVisible();
  const sections = page.locator('[data-book-chapter]');
  await expect(sections).toHaveCount(4);
  expect(await sections.evaluateAll(nodes => nodes.map(node => node.id))).toEqual(chapters);
  await expect(page.locator('[data-trajectory-lab], [data-knockback-lab], [data-animation-viewer]')).toHaveCount(0);
  const duplicateIds = await page.locator('[id]').evaluateAll(nodes => nodes.map(node => node.id).filter((id, index, all) => all.indexOf(id) !== index));
  expect(duplicateIds).toEqual([]);

  const contents = page.getByRole('navigation', { name: 'Book contents' });
  expect(await contents.locator('[data-chapter-link]').evaluateAll(links => links.map(link => link.getAttribute('href')))).toEqual(chapters.map(chapter => `#${chapter}`));
  await page.screenshot({ path: path.join(screens, 'book-cover-desktop.png') });
  await contents.getByRole('link', { name: 'Controls', exact: true }).click();
  await expect(page).toHaveURL(/\/book\/#controls$/);
  await expect(page.locator('[data-controller-manual]')).toBeVisible();
  await expect(contents.locator('[data-chapter-link=controls]')).toHaveAttribute('aria-current', 'location');
  await page.locator('[data-practice=a]').click();
  await expect(page.locator('[data-readout]')).toContainText(/J.*NES A.*Jab/);

  const invalidBookAnchors = await page.locator('a[href^="#"], a[href^="/book/#"]').evaluateAll(links => links
    .filter(link => !document.getElementById(decodeURIComponent(new URL(link.href).hash.slice(1))))
    .map(link => link.getAttribute('href')));
  expect(invalidBookAnchors).toEqual([]);
  const undecodedImages = await page.locator('img').evaluateAll(async images => {
    images.forEach(image => { image.loading = 'eager'; });
    await Promise.all(images.map(image => image.decode().catch(() => undefined)));
    return images.filter(image => !image.complete || image.naturalWidth === 0 || image.naturalHeight === 0)
      .map(image => image.currentSrc || image.src);
  });
  expect(undecodedImages).toEqual([]);

  for (const width of [320, 390, 799, 1440]) {
    await page.setViewportSize({ width, height: 1000 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), `Book at ${width}`).toBeLessThanOrEqual(1);
    if (width === 390) {
      await page.goto('/book/');
      await page.screenshot({ path: path.join(screens, 'book-cover-mobile.png') });
      const menu = page.locator('[data-book-contents]');
      await expect(menu).not.toHaveAttribute('open', '');
      await menu.locator('summary').click();
      await expect(menu).toHaveAttribute('open', '');
      await contents.getByRole('link', { name: 'Fighters', exact: true }).click();
      await expect(page).toHaveURL(/\/book\/#fighters$/);
      await expect(menu).not.toHaveAttribute('open', '');
      await expect(page.locator('#fighters')).toBeInViewport();
    }
  }

  await page.goto('/book/#controller-manual-title');
  await expect(page.locator('#controller-manual-title')).toBeInViewport();
  await page.goto('/book/#_top');
  await expect(page.locator('[data-book-contents]')).toBeVisible();
  await page.goto('/book/?from=bookmark#physics');
  await expect(page).toHaveURL(/\/docs\/physics\/\?from=bookmark$/);
  await page.goto('/book/?from=bookmark#gravity-is-deliberately-coarse');
  await expect(page).toHaveURL(/\/docs\/physics\/\?from=bookmark#gravity-is-deliberately-coarse$/);
  await page.goto('/book/controls/?from=bookmark#controller-manual-title');
  await expect(page).toHaveURL(/\/book\/\?from=bookmark#controller-manual-title$/);
  await expect(page.locator('#controller-manual-title')).toBeInViewport();
  const reveal = page.locator('#controls details').filter({ has: page.getByText('Why did held J stop after one jab?', { exact: true }) });
  await reveal.locator('summary').click();
  await expect(reveal).toHaveAttribute('open', '');
  expect(errors).toEqual([]);
});
