import { expect, test } from '@playwright/test';
import { mkdir, readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const screens = fileURLToPath(new URL('../../out/reports/web/screens/', import.meta.url));
const chapters = [
  '/book/',
  '/docs/', '/docs/build/', '/docs/assets/', '/docs/testing/', '/docs/website/',
  '/docs/architecture/', '/docs/physics/', '/docs/contact/', '/docs/rendering/', '/docs/audio/',
  '/blog/', '/blog/a-small-physics-language/', '/blog/making-the-fight-readable/',
  '/blog/one-rom-many-observers/',
];
function observe(page) {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  page.on('request', request => {
    const url = new URL(request.url());
    if (['http:', 'https:'].includes(url.protocol) && url.hostname !== '127.0.0.1') errors.push(`External request: ${url}`);
  });
  return errors;
}
test.beforeAll(async () => { await mkdir(screens, { recursive: true }); });

test('Book, Docs, and Blog indexes render canonical content and link to existing routes', async ({ page, request }) => {
  const errors = observe(page);
  const links = new Set();
  for (const route of chapters) {
    expect((await page.goto(route))?.ok(), route).toBe(true);
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    for (const href of await page.locator('a[href^="/"]').evaluateAll(elements => elements.map(a => a.getAttribute('href')))) {
      links.add(href.split('#')[0]);
    }
    await page.setViewportSize({ width: 390, height: 844 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), route).toBeLessThanOrEqual(1);
    if (route === '/book/') await page.screenshot({ path: path.join(screens, 'book-mobile.png') });
    await page.setViewportSize({ width: 1440, height: 1000 });
  }
  for (const route of links) {
    expect(route, `canonical internal link ${route}`).not.toMatch(/^\/(?:guide|technical|devlog|emulator)(?:\/|$)/);
    expect((await request.get(route)).ok(), `Internal link ${route}`).toBe(true);
  }
  await page.goto('/docs/architecture/');
  await expect(page.getByRole('heading', { level: 1, name: 'Runtime architecture', exact: true })).toBeVisible();
  await expect(page.getByRole('img', { name: /source-to-player build pipeline/i })).toBeVisible();
  expect(errors).toEqual([]);
});

test('legacy learning, technical, and journal routes redirect to the matching canonical path without losing query or hash', async ({ page }) => {
  const redirects = [
    ['/book/first-match/', '/book/#first-match'], ['/book/controls/', '/book/#controls'],
    ['/book/fighters/', '/book/#fighters'], ['/book/combat/', '/book/#combat'],
    ['/book/architecture/', '/docs/architecture/'], ['/book/physics/', '/docs/physics/'],
    ['/book/contact/', '/docs/contact/'], ['/book/rendering/', '/docs/rendering/'],
    ['/book/audio/', '/docs/audio/'], ['/guide/getting-started/', '/book/#first-match'],
    ['/guide/controls/', '/book/#controls'], ['/guide/fighters/', '/book/#fighters'],
    ['/guide/combat/', '/book/#combat'], ['/technical/architecture/', '/docs/architecture/'],
    ['/technical/physics/', '/docs/physics/'], ['/technical/combat/', '/docs/contact/'],
    ['/technical/rendering/', '/docs/rendering/'], ['/technical/audio/', '/docs/audio/'],
    ['/technical/testing/', '/docs/testing/'],
    ['/technical/site/', '/docs/website/'], ['/devlog/', '/blog/'],
    ['/devlog/a-small-physics-language/', '/blog/a-small-physics-language/'],
    ['/devlog/making-the-fight-readable/', '/blog/making-the-fight-readable/'],
    ['/devlog/one-rom-many-observers/', '/blog/one-rom-many-observers/'],
  ];
  for (const [legacy, canonical] of redirects) {
    const destination = new URL(canonical, 'http://127.0.0.1');
    destination.search = '?from=legacy';
    const escapedDestination = `${destination.pathname}${destination.search}${destination.hash}`.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    await page.goto(`${legacy}?from=legacy`);
    await expect(page).toHaveURL(new RegExp(`${escapedDestination}$`));

    await page.goto(`${legacy}?from=legacy#preserved-anchor`);
    const escapedPreserved = `${destination.pathname}${destination.search}#preserved-anchor`.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    await expect(page).toHaveURL(new RegExp(`${escapedPreserved}$`));
  }
});

test('documentation header keeps global navigation visible and keeps search and section navigation available on mobile', async ({ page }) => {
  await page.goto('/docs/build/');
  const header = page.locator('header.header');
  await expect(header).toBeVisible();
  await expect(header).toHaveCSS('position', 'fixed');
  await expect(header.getByRole('link', { name: 'Play', exact: true })).toBeVisible();
  await expect(header.getByRole('link', { name: 'Book', exact: true })).toBeVisible();
  await expect(header.getByRole('link', { name: 'Docs', exact: true })).toBeVisible();
  await expect(header.getByRole('link', { name: 'Blog', exact: true })).toBeVisible();
  await expect(header.getByRole('button', { name: /Search/ })).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(header.getByRole('link', { name: 'Play', exact: true })).toBeVisible();
  await expect(header.getByRole('link', { name: 'Book', exact: true })).toBeVisible();
  await expect(header.getByRole('link', { name: 'Docs', exact: true })).toBeVisible();
  await expect(header.getByRole('link', { name: 'Blog', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Menu', exact: true }).click();
  await expect(page.locator('.sidebar-pane')).toBeVisible();
  await expect(page.locator('.sidebar-pane').getByRole('link', { name: 'Assets', exact: true })).toBeVisible();
  await expect(page.locator('header.header').getByRole('combobox')).toHaveCount(0);
  await expect(page.locator('[data-site-footer]').getByRole('combobox')).toHaveCount(1);
});

test('Book uses its complete custom contents while Docs keeps its section sidebar and Blog has none', async ({ page }) => {
  await page.goto('/book/');
  await expect(page.getByRole('heading', { level: 1, name: 'The Book', exact: true })).toHaveCount(1);
  const header = page.locator('header.header');
  await expect(header).toHaveCSS('position', 'fixed');
  await expect(header.getByRole('button', { name: /Search/ })).toBeVisible();
  await expect(page.locator('[data-site-footer]').getByRole('combobox')).toHaveCount(1);
  await expect(page.locator('.sidebar-pane')).toHaveCount(0);
  const contents = page.locator('aside.book-nav details[data-book-contents]');
  await expect(contents).toHaveAttribute('open', '');
  await expect(contents.locator('summary')).toBeHidden();
  const chapterLinks = contents.locator('nav[aria-label="Book contents"] [data-chapter-link]');
  await expect(chapterLinks).toHaveCount(4);
  await expect(chapterLinks.first()).toHaveAttribute('href', '#first-match');
  await expect(chapterLinks.last()).toHaveAttribute('href', '#combat');
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(contents).not.toHaveAttribute('open', '');
  await expect(contents.locator('summary')).toHaveText(/Contents.*↕/);
  await contents.locator('summary').click();
  await expect(contents).toHaveAttribute('open', '');
  await chapterLinks.nth(1).click();
  await expect(page).toHaveURL(/\/book\/#controls$/);
  await expect(contents).not.toHaveAttribute('open', '');
  await page.setViewportSize({ width: 1440, height: 1000 });

  await page.goto('/docs/testing/');
  const docsSidebar = page.locator('.sidebar-pane');
  await expect(docsSidebar).toBeVisible();
  await expect(docsSidebar.locator('a[href^="/docs/"]')).not.toHaveCount(0);
  await expect(docsSidebar.locator('a[href^="/book/"]')).toHaveCount(0);
  await expect(docsSidebar.locator('ul ul')).toHaveCount(0);

  await page.goto('/blog/one-rom-many-observers/');
  await expect(page.locator('.sidebar')).toHaveCount(0);
});

test('Starlight search finds a technical chapter and opens its result', async ({ page }) => {
  const errors = observe(page);
  await page.goto('/book/');
  await page.getByRole('button', { name: /Search/ }).click();
  const search = page.getByRole('textbox', { name: /Search/ });
  await search.fill('gravity');
  const result = page.locator('.pagefind-ui__results a[href="/docs/physics/#gravity-is-deliberately-coarse"]');
  await expect(result).toBeVisible();
  await result.click();
  await expect(page).toHaveURL(/\/docs\/physics\/#gravity-is-deliberately-coarse$/);
  await expect(page.getByRole('heading', { level: 1, name: 'Integer physics and recovery', exact: true })).toBeVisible();
  await expect(page.locator('#gravity-is-deliberately-coarse')).toBeInViewport();
  expect(errors).toEqual([]);
});

test('physics trace advances, lands, resets, and responds to an experiment', async ({ page }) => {
  const errors = observe(page);
  await page.goto('/docs/physics/');
  const lab = page.locator('[data-trajectory-lab]');
  const status = lab.locator('[data-status]');
  await expect(status).toContainText('frame 0 · y 143 · vy -9');
  await lab.getByRole('button', { name: 'Advance one update' }).click();
  await expect(status).toContainText('frame 1 · y 135 · vy -8');
  await lab.getByRole('button', { name: 'Run 40 updates' }).click();
  await expect(status).toContainText('landed');
  await expect(status).toContainText('y 143 · vy 0');
  await lab.getByRole('button', { name: 'Reset trace' }).click();
  await lab.locator('[data-vy]').fill('-4');
  await lab.getByRole('button', { name: 'Advance one update' }).click();
  await expect(status).toContainText('frame 1 · y 140 · vy -3');
  await lab.getByRole('button', { name: 'Run 40 updates' }).click();
  await page.screenshot({ path: path.join(screens, 'physics-lab.png') });
  expect(errors).toEqual([]);
});

test('retro art studies support pointer and keyboard selection at desktop and mobile', async ({ page }) => {
  const errors = observe(page);
  await page.goto('/design/');
  await page.evaluate(() => document.fonts.load('16px Cartridge', 'SMASH'));
  expect(await page.evaluate(() => document.fonts.check('16px Cartridge', 'SMASH'))).toBe(true);
  for (const [id, name] of [['poster', 'Fight poster'], ['fighters', 'Fighter roster'], ['manual', 'Cartridge manual']]) {
    const tab = page.getByRole('tab', { name: new RegExp(name) });
    await tab.click();
    await expect(tab).toHaveAttribute('aria-selected', 'true');
    await expect(page.getByRole('tabpanel')).toHaveCount(1);
    const panel = page.getByRole('tabpanel');
    if (id === 'manual') await expect(panel.getByRole('img', { name: 'Current Smash.NES battle capture' })).toBeVisible();
    else await expect(panel.locator('.fighter-pose')).toHaveCount(4);
    if (id === 'fighters') await page.screenshot({ path: path.join(screens, `design-${id}-desktop.png`) });
    await page.setViewportSize({ width: 390, height: 844 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), name).toBeLessThanOrEqual(1);
    expect((await panel.locator('.artboard').boundingBox()).width, `${name} art uses the mobile column`).toBeGreaterThan(320);
    if (id === 'fighters') await page.screenshot({ path: path.join(screens, `design-${id}-mobile.png`) });
    await page.setViewportSize({ width: 1440, height: 1000 });
  }
  await page.getByRole('tab', { name: /Cartridge manual/ }).press('Home');
  await expect(page.getByRole('tab', { name: /Fight poster/ })).toBeFocused();
  await expect(page.getByRole('tab', { name: /Fight poster/ })).toHaveAttribute('aria-selected', 'true');
  await page.getByRole('tab', { name: /Fight poster/ }).press('ArrowRight');
  await expect(page.getByRole('tab', { name: /Fighter roster/ })).toHaveAttribute('aria-selected', 'true');
  expect(errors).toEqual([]);
});

test('manual defaults dark and preserves explicit paper and automatic themes', async ({ page }) => {
  const errors = observe(page);
  await page.emulateMedia({ colorScheme: 'light' });
  await page.goto('/book/');
  const theme = page.locator('[data-site-footer]').getByRole('combobox');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(theme).toHaveValue('dark');
  await theme.selectOption('light');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await expect(theme).toHaveValue('light');
  await page.getByRole('button', { name: /Search/ }).click();
  await page.getByRole('textbox', { name: /Search/ }).fill('gravity');
  await page.locator('.pagefind-ui__results a[href="/docs/physics/#gravity-is-deliberately-coarse"]').click();
  await expect(page).toHaveURL(/\/docs\/physics\/#gravity-is-deliberately-coarse$/);
  await expect(page.getByRole('heading', { level: 1, name: 'Integer physics and recovery', exact: true })).toBeVisible();
  await expect(page.locator('#gravity-is-deliberately-coarse')).toBeInViewport();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.goto('/docs/architecture/');
  await theme.selectOption('auto');
  await page.emulateMedia({ colorScheme: 'dark' });
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await page.reload();
  await expect(theme).toHaveValue('auto');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  expect(errors).toEqual([]);
});

test('workshop studies persist while the chosen cartridge site and screen effects stay consistent', async ({ page }) => {
  const errors = observe(page);
  const themes = [['a','Fight Poster'], ['b','Rental Cartridge'], ['c','Xerox Brawl'], ['d','After Hours']];
  const root = page.locator('html');
  await page.goto('/design/?keep=1#main');
  for (const [id, name] of themes) {
    const radio = page.getByRole('radio', { name: new RegExp(name) });
    await radio.click();
    await expect(radio).toHaveAttribute('aria-checked', 'true');
    await expect(root).toHaveAttribute('data-art-theme', id);
    const selectedURL = new URL(page.url());
    expect(selectedURL.searchParams.get('theme')).toBe(id);
    expect(selectedURL.searchParams.get('keep')).toBe('1');
    expect(selectedURL.hash).toBe('#main');
    for (const width of [320, 390, 900]) {
      await page.setViewportSize({ width, height: 844 });
      expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), `${name} art at ${width}px`).toBeLessThanOrEqual(1);
    }
    await page.setViewportSize({ width: 1440, height: 1000 });
  }
  await page.getByRole('radio', { name: /After Hours/ }).press('Home');
  await expect(page.getByRole('radio', { name: /Fight Poster/ })).toBeFocused();
  await page.getByRole('radio', { name: /Fight Poster/ }).press('ArrowRight');
  await expect(root).toHaveAttribute('data-art-theme', 'b');
  await page.reload();
  await expect(root).toHaveAttribute('data-art-theme', 'b');

  // Alternate art studies stay in the workshop; published routes use the chosen B treatment.
  await page.goto('/design/?theme=d');
  await page.getByRole('navigation', { name: 'Primary navigation' }).getByRole('link', { name: /^Play$/i }).click();
  await expect(root).toHaveAttribute('data-art-theme', 'b');
  await expect.poll(() => page.evaluate(() => Boolean(window.EJS_emulator?.started)), { timeout: 60_000 }).toBe(true);
  await expect(page.locator('[data-rental-roster]')).toHaveCount(0);
  const emulator = await page.evaluateHandle(() => window.EJS_emulator);
  const toggle = page.locator('[data-site-footer]').getByRole('button', {name:'Screen effects'});
  for (const enabled of [false, true, false]) {
    await toggle.click();
    await expect(root).toHaveAttribute('data-screen-fx', enabled ? 'on' : 'off');
    expect(await emulator.evaluate(previous => previous === window.EJS_emulator), 'Effects retain the running emulator').toBe(true);
  }
  await emulator.dispose();
  await page.goto('/docs/architecture/');
  await expect(root).toHaveAttribute('data-art-theme', 'b');
  await expect(root).toHaveAttribute('data-screen-fx','off');
  await expect(page.locator('[data-site-footer]').getByRole('button',{name:'Screen effects'})).toHaveAttribute('aria-pressed','false');
  for (const scheme of ['dark', 'light']) {
    await page.locator('[data-site-footer]').getByRole('combobox').selectOption(scheme);
    await expect(root).toHaveAttribute('data-theme', scheme);
    await page.screenshot({ path: path.join(screens, `theme-b-docs-${scheme}.png`) });
    for (const width of [320, 390]) {
      await page.setViewportSize({ width, height: 844 });
      expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth), `Cartridge docs at ${width}px`).toBeLessThanOrEqual(1);
    }
    await page.setViewportSize({ width:1440, height:1000 });
  }
  await page.goto('/design/?theme=unknown');
  await expect(root).toHaveAttribute('data-art-theme','d');
  expect(errors).toEqual([]);
});

test('animation scrubber shows authored poses and only marks melee windows', async ({ page }) => {
  const errors = observe(page);
  await page.goto('/docs/rendering/');
  const viewer = page.locator('[data-animation-viewer]');
  await viewer.getByLabel('Choose a fighter').selectOption('simon');
  await viewer.getByLabel('Choose an animation').selectOption('jab');
  await viewer.locator('[data-tick]').fill('5');
  await expect(viewer.locator('[data-caption]')).toContainText('Simon Belmont · jab · tick 5 · jab_active · collision active');
  await expect(viewer.locator('[data-pose]')).toHaveAttribute('data-active', 'true');
  await expect(viewer.locator('[data-phase]')).toHaveCount(3);
  await expect.poll(() => viewer.locator('.thumb').first().evaluate(element => element.getBoundingClientRect().width)).toBe(40);
  await expect(viewer.locator('[data-sheet]')).toHaveAttribute('src', '/assets/fighters/simon.png');
  await viewer.locator('[data-phase="2"]').click();
  await expect(viewer.locator('[data-caption]')).toContainText('jab_recover');
  await expect(viewer.locator('[data-pose]')).toHaveAttribute('data-active', 'false');
  await page.screenshot({ path: path.join(screens, 'animation-lab.png') });
  await viewer.getByLabel('Choose a fighter').selectOption('mario');
  await viewer.getByLabel('Choose an animation').selectOption('special');
  await viewer.locator('[data-tick]').fill('4');
  await expect(viewer.locator('[data-caption]')).toContainText('special_active');
  await expect(viewer.locator('[data-pose]')).toHaveAttribute('data-active', 'false');
  expect(errors).toEqual([]);
});

test('downloaded ROM matches the locally validated native artifact byte for byte', async ({ request }) => {
  const native = await readFile(new URL('../../out/smash.nes', import.meta.url));
  const served = await (await request.get('/rom/smash.nes')).body();
  const digest = bytes => createHash('sha256').update(bytes).digest('hex');
  expect(digest(served)).toBe(digest(native));
});
