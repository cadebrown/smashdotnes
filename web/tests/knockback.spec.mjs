import { expect, test } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const screens = fileURLToPath(new URL('../../out/reports/web/screens/', import.meta.url));

test.beforeAll(async () => { await mkdir(screens, { recursive: true }); });

test('knockback lab preserves the ROM impulse steps, cap, and accumulated horizontal trace', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await page.goto('/docs/physics/#knockback-title');
  const lab = page.locator('[data-knockback-lab]');
  const slider = lab.getByRole('slider', { name: /damage after hit/i });
  const push = lab.locator('[data-push]');
  const distance = lab.locator('[data-distance]');
  const step = lab.getByRole('button', { name: 'Step one update' });

  await expect(slider).toHaveAttribute('min', '0');
  await expect(slider).toHaveAttribute('max', '255');
  await expect(lab.getByText(/total damage after the hit/i)).toBeVisible();
  await expect(push).toHaveText('4 px/update');
  await slider.focus();
  await page.keyboard.press('ArrowRight');
  await expect(lab.locator('[data-damage-output]')).toHaveText('1%');
  await expect(push).toHaveText('4 px/update');
  await slider.fill('16');
  await expect(push).toHaveText('5 px/update');
  await page.keyboard.press('End');
  await expect(lab.locator('[data-damage-output]')).toHaveText('255%');
  await expect(push).toHaveText('14 px/update');

  await lab.getByRole('button', { name: '64%' }).click();
  await expect(push).toHaveText('8 px/update');
  await step.click();
  await expect(distance).toHaveText('8 px');
  await step.click();
  await expect(distance).toHaveText('15 px');
  await lab.getByRole('button', { name: 'Reset trace' }).click();
  await lab.getByRole('button', { name: '160% cap' }).click();
  await expect(push).toHaveText('14 px/update');
  for (let count = 0; count < 14; count += 1) await step.click();
  await expect(distance).toHaveText('105 px');
  await expect(lab.getByRole('button', { name: 'Push spent' })).toBeDisabled();

  await lab.scrollIntoViewIfNeeded();
  await page.screenshot({ path: path.join(screens, 'knockback-lab-desktop.png') });

  await page.setViewportSize({ width: 320, height: 1200 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).toBeLessThanOrEqual(1);
  await lab.scrollIntoViewIfNeeded();
  await page.screenshot({ path: path.join(screens, 'knockback-lab.png') });
  expect(errors).toEqual([]);
});
