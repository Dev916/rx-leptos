import { test, expect, type Page } from '@playwright/test';

// The same five panels exist in the CSR and SSR apps, so this suite runs
// against either: `BASE_URL` selects the deployment.

const section = (page: Page, title: string) => page.locator('section', { hasText: title });
const number = (text: string | null) => Number((text ?? '').match(/-?\d+(\.\d+)?/)?.[0] ?? NaN);

test.beforeEach(async ({ page }) => {
  await page.goto('./');
  await expect(page.getByRole('heading', { level: 2, name: /Typeahead/ })).toBeVisible();
});

test('typeahead debounces, filters and cancels a stale search', async ({ page }) => {
  const panel = section(page, 'Typeahead');
  const input = panel.getByRole('searchbox');
  const status = panel.locator('p.muted');

  // The initial empty query settles after the debounce.
  await expect(status).toContainText('1 searches started, 1 delivered', { timeout: 5_000 });

  await input.fill('rx');
  await expect(status).toHaveText('Searching…');
  await expect(panel.locator('li')).toHaveText(['rxrust', 'rx-leptos']);
  await expect(status).toContainText('2 searches started, 2 delivered, 0 cancelled');

  // A new query while the previous search is in flight cancels it.
  await input.fill('r');
  await page.waitForTimeout(450);
  await input.fill('ru');
  await expect(panel.locator('li')).toHaveText(['rust']);
  await expect(status).toContainText('1 cancelled');
});

test('stopwatch runs, pauses, resumes and resets', async ({ page }) => {
  const panel = section(page, 'Stopwatch');
  const display = panel.locator('p').first();
  await expect(display).toHaveText('0.0 s');

  await panel.getByRole('button', { name: 'Start' }).click();
  await page.waitForTimeout(1_100);
  const running = number(await display.textContent());
  expect(running).toBeGreaterThanOrEqual(0.8);

  await panel.getByRole('button', { name: 'Pause' }).click();
  const paused = await display.textContent();
  await page.waitForTimeout(500);
  await expect(display).toHaveText(paused!);

  await panel.getByRole('button', { name: 'Start' }).click();
  await page.waitForTimeout(400);
  expect(number(await display.textContent())).toBeGreaterThan(number(paused));

  await panel.getByRole('button', { name: 'Reset' }).click();
  await page.waitForTimeout(150);
  expect(number(await display.textContent())).toBeLessThan(0.5);
});

test('mouse tracker follows the pointer', async ({ page }) => {
  const panel = section(page, 'Mouse');
  await expect(panel.locator('p')).toContainText('x = 0, y = 0');
  const box = (await panel.boundingBox())!;
  await page.mouse.move(box.x + 40, box.y + 40);
  await page.mouse.move(box.x + 60, box.y + 50);
  await expect(panel.locator('p')).not.toContainText('x = 0, y = 0');
});

test('frames advance while running and stop when paused', async ({ page }) => {
  const panel = section(page, 'Frames');
  const readout = panel.locator('p').first();
  await expect(readout).not.toContainText('0 frames', { timeout: 5_000 });
  await expect(readout).toContainText('fps');

  await panel.getByRole('button', { name: 'Pause' }).click();
  await page.waitForTimeout(100);
  const paused = await readout.textContent();
  await page.waitForTimeout(400);
  await expect(readout).toHaveText(paused!);
  await expect(panel.getByRole('button', { name: 'Resume' })).toBeVisible();
});

test('fetch loads data.json on demand', async ({ page }) => {
  const panel = section(page, 'Fetch');
  await expect(panel.locator('p.muted')).toHaveText('idle');
  await expect(panel.locator('li')).toHaveCount(0);

  await panel.getByRole('button', { name: 'Load data.json' }).click();
  await expect(panel.locator('p.muted')).toHaveText('ok');
  await expect(panel.locator('li').first()).toHaveText('rxrust');
  await expect(panel.locator('li')).toHaveCount(6);
});
