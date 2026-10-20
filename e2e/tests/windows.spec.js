// The Win98 windows (spec 11.9): taskbar buttons, focus and order, minimize,
// maximize, close, dragging, and the Logbook window itself.
import { test, expect } from '@playwright/test';
import { watchErrors } from './helpers.js';

const win = (page, name) => page.getByRole('region', { name, exact: true });
const task = (page, id) => page.locator(`.taskbar [data-task="${id}"]`);
const logbook = page => page.locator('[data-win="logbook"]');
const icon = (page, name) => page.getByRole('button', { name, exact: true }).and(page.locator('.dicon'));

test('app windows: taskbar buttons, focus, order, minimize, maximize, close', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await icon(page, 'Documents').dblclick();
  await expect(win(page, 'Documents')).toBeVisible();
  await expect(task(page, 'docs')).toBeVisible();
  await expect(task(page, 'docs')).toHaveAttribute('aria-pressed', 'true');
  // The other windows get a gray title bar.
  await expect(logbook(page)).toHaveClass(/inactive/);
  await expect(win(page, 'Documents')).not.toHaveClass(/inactive/);

  await icon(page, 'Recycle Bin').dblclick();
  await expect(win(page, 'Recycle Bin')).toBeVisible();
  await expect(win(page, 'Documents')).toHaveClass(/inactive/);
  await expect(task(page, 'docs')).toHaveAttribute('aria-pressed', 'false');
  const z = name => win(page, name).evaluate(el => Number(getComputedStyle(el).zIndex));
  expect(await z('Recycle Bin')).toBeGreaterThan(await z('Documents'));

  // A click on a window brings it to the front.
  await win(page, 'Documents').locator('.titlebar .ttl').click();
  expect(await z('Documents')).toBeGreaterThan(await z('Recycle Bin'));
  await expect(task(page, 'docs')).toHaveAttribute('aria-pressed', 'true');

  // Minimize: the window goes, the taskbar button stays. The button brings it back.
  await win(page, 'Documents').getByRole('button', { name: 'Minimize' }).click();
  await expect(win(page, 'Documents')).toBeHidden();
  await expect(task(page, 'docs')).toBeVisible();
  await task(page, 'docs').click();
  await expect(win(page, 'Documents')).toBeVisible();
  // The button of the active window minimizes it.
  await task(page, 'docs').click();
  await expect(win(page, 'Documents')).toBeHidden();
  await task(page, 'docs').click();

  // Maximize fills the screen above the taskbar. A double-click on the title bar restores.
  await win(page, 'Documents').getByRole('button', { name: 'Maximize' }).click();
  const box = await win(page, 'Documents').boundingBox();
  const bar = await page.locator('.taskbar').boundingBox();
  const vw = await page.evaluate(() => document.documentElement.clientWidth);
  expect(box.x).toBe(0);
  expect(box.y).toBe(0);
  expect(box.width).toBe(vw);
  expect(Math.round(box.y + box.height)).toBe(Math.round(bar.y));
  await win(page, 'Documents').locator('.titlebar .ttl').dblclick();
  expect((await win(page, 'Documents').boundingBox()).width).toBeLessThan(vw);

  // Close: the window and its taskbar button go.
  await win(page, 'Documents').getByRole('button', { name: 'Close' }).click();
  await expect(win(page, 'Documents')).toBeHidden();
  await expect(task(page, 'docs')).toBeHidden();
  await expect(win(page, 'Recycle Bin')).not.toHaveClass(/inactive/);
  expect(errors, errors.join('\n')).toEqual([]);
});

test('a window moves by its title bar and stays on the screen', async ({ page }) => {
  await page.goto('/');
  await icon(page, 'Recycle Bin').dblclick();
  const bin = win(page, 'Recycle Bin');
  const before = await bin.boundingBox();
  const title = await bin.locator('.titlebar .ttl').boundingBox();
  await page.mouse.move(title.x + 20, title.y + 5);
  await page.mouse.down();
  await page.mouse.move(title.x + 20 - 150, title.y + 5 + 60, { steps: 6 });
  await page.mouse.up();
  const after = await bin.boundingBox();
  expect(Math.round(after.x)).toBe(Math.round(before.x - 150));
  expect(Math.round(after.y)).toBe(Math.round(before.y + 60));
  // A drag far up stops at the top edge.
  await page.mouse.move(after.x + 30, after.y + 10);
  await page.mouse.down();
  await page.mouse.move(after.x + 30, -200, { steps: 4 });
  await page.mouse.up();
  expect((await bin.boundingBox()).y).toBe(0);
});

test('the Logbook window minimizes, maximizes, closes, and comes back', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  const lb = logbook(page);
  const home = page.locator('.taskbar .task[data-nav="home"]');

  // Minimize: Home in the taskbar brings it back.
  await lb.getByRole('button', { name: 'Minimize' }).click();
  await expect(lb).toBeHidden();
  await expect(home).toBeVisible();
  await home.click();
  await expect(lb).toBeVisible();

  // Maximize: the full width of the screen.
  await lb.getByRole('button', { name: 'Maximize' }).click();
  const vw = await page.evaluate(() => document.documentElement.clientWidth);
  expect((await lb.boundingBox()).width).toBe(vw);
  await expect(lb.getByRole('button', { name: 'Restore' })).toBeVisible();
  await lb.getByRole('button', { name: 'Restore' }).click();
  expect((await lb.boundingBox()).width).toBeLessThan(vw);

  // Close: the window and its taskbar buttons go. Logbook.exe opens it again.
  await lb.getByRole('button', { name: 'Close' }).click();
  await expect(lb).toBeHidden();
  await expect(home).toBeHidden();
  await icon(page, 'Logbook.exe').dblclick();
  await expect(lb).toBeVisible();
  await expect(home).toBeVisible();
  await expect(page.locator('#ns-rows a.ttl').first()).toBeVisible();
  expect(errors, errors.join('\n')).toEqual([]);
});

test('a mode change hides the apps; Windows shows them again', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await icon(page, 'Documents').dblclick();
  await logbook(page).getByRole('button', { name: 'Minimize' }).click();
  await page.evaluate(() => Mode.set('night'));
  await expect(page.locator('dos-shell')).toBeVisible();
  await expect(win(page, 'Documents')).toBeHidden();
  await expect(icon(page, 'Logbook.exe')).toBeHidden();
  await page.evaluate(() => Mode.set('general'));
  await expect(win(page, 'Documents')).toBeVisible();
  // The Logbook window is back: a mode change shows it.
  await expect(logbook(page)).toBeVisible();
});

test('a long post scrolls inside the Logbook window, which stops above the taskbar', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/posts/block-level-access-lists-and-parallel-execution');
  const view = logbook(page).locator('.viewport');
  const fit = () => page.evaluate(() => ({
    page: document.documentElement.scrollHeight - innerHeight,
    gap: document.querySelector('.taskbar').getBoundingClientRect().top - document.querySelector('[data-win="logbook"]').getBoundingClientRect().bottom,
  }));
  const f = await fit();
  expect(f.page).toBe(0);
  expect(f.gap).toBeGreaterThanOrEqual(0);
  expect(await view.evaluate(v => v.scrollHeight > v.clientHeight)).toBe(true);
  await view.hover();
  await page.mouse.wheel(0, 400);
  await expect.poll(() => view.evaluate(v => v.scrollTop)).toBeGreaterThan(0);
  // A new page starts at the top.
  await logbook(page).getByRole('button', { name: 'Home' }).click();
  await expect.poll(() => view.evaluate(v => v.scrollTop)).toBe(0);
  // Maximized: the window fills the screen above the taskbar.
  await logbook(page).getByRole('button', { name: 'Maximize' }).click();
  expect((await fit()).gap).toBeGreaterThanOrEqual(0);
  expect((await fit()).page).toBe(0);
  expect(errors, errors.join('\n')).toEqual([]);
});

test.describe('tall screen', () => {
  test.use({ viewport: { width: 1280, height: 2000 } });

  test('the Logbook window is only as tall as a short page; maximized, it fills the screen', async ({ page }) => {
    await page.goto('/posts/block-level-access-lists-and-parallel-execution');
    const view = logbook(page).locator('.viewport');
    const gap = () => page.evaluate(() => document.querySelector('.taskbar').getBoundingClientRect().top - document.querySelector('[data-win="logbook"]').getBoundingClientRect().bottom);
    expect(await view.evaluate(v => v.scrollHeight - v.clientHeight)).toBe(0);
    expect(await gap()).toBeGreaterThan(100);
    await logbook(page).getByRole('button', { name: 'Maximize' }).click();
    expect(await gap()).toBe(0);
  });
});

test.describe('narrow screen', () => {
  test.use({ viewport: { width: 390, height: 844 } });

  test('app windows open maximized, with icon-only taskbar buttons', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: 'Start' }).click();
    await page.getByRole('menuitem', { name: 'Documents', exact: true }).click();
    const box = await win(page, 'Documents').boundingBox();
    const bar = await page.locator('.taskbar').boundingBox();
    expect(box.x).toBe(0);
    expect(box.y).toBe(0);
    expect(box.width).toBe(390);
    expect(Math.round(box.y + box.height)).toBe(Math.round(bar.y));
    await expect(task(page, 'docs')).toBeVisible();
    await expect(task(page, 'docs').locator('span')).toBeHidden();
  });
});
