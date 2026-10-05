import { test, expect, devices } from '@playwright/test';
import { watchErrors } from './helpers.js';

const BAL = 'block-level-access-lists-and-parallel-execution';
// The project runs Chromium. Keep the iPhone screen, touch, and coarse pointer.
const { defaultBrowserType: _, ...iPhone } = devices['iPhone 13'];

const shell = page => page.locator('phone-shell');
const soft = (page, side) => page.locator(`phone-shell [data-soft="${side}"]`);

test.describe('phone', () => {
  test.use(iPhone);

  test('phone mode is the default; Read opens the newest post; Back goes to the list', async ({ page }) => {
    const errors = watchErrors(page);
    await page.goto('/');
    await expect(page.locator('html')).toHaveAttribute('data-mode', 'phone');
    await expect(page.locator('html')).toHaveAttribute('data-device', 'phone');
    await expect(shell(page)).toBeVisible();
    await expect(page.locator('.desktop')).toBeHidden();
    await expect(shell(page).getByRole('heading', { name: "Eitan's Logbook" })).toBeVisible();
    await expect(shell(page).getByRole('img', { name: 'Logbook logo' })).toBeVisible();
    await expect(shell(page).locator('[data-clock]')).toHaveText(/^\d\d:\d\d$/);
    await expect(shell(page).getByRole('button', { name: '1 new post' })).toBeVisible();
    await expect(soft(page, 'left')).toHaveText('Read');
    await expect(soft(page, 'right')).toHaveText('Menu');

    // Other test files publish posts too: ask the API for the newest one.
    const [newest] = await (await page.request.get('/api/posts')).json();
    await soft(page, 'left').tap();
    await expect(page).toHaveURL(`/posts/${newest.slug}`);
    const read = shell(page).locator('[data-read]');
    await expect(read.locator('.ph-title')).toHaveText(newest.title);
    await expect(shell(page).locator('[data-pages]')).toHaveText(/^1\/\d+$/);
    await expect(soft(page, 'left')).toHaveText('Options');
    const noScroll = await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth);
    expect(noScroll).toBe(true);

    // The Back softkey goes to the post list; browser Back and Forward agree.
    await soft(page, 'right').tap();
    await expect(page).toHaveURL('/');
    await expect(shell(page).locator('[data-list-title]')).toHaveText('Posts');
    await expect(shell(page).locator('[data-list] a').first()).toHaveAttribute('href', `/posts/${newest.slug}`);
    await page.goForward();
    await expect(page).toHaveURL(`/posts/${newest.slug}`);
    await page.goBack();
    await expect(shell(page).locator('[data-list-title]')).toHaveText('Posts');
    await page.goBack();
    await expect(shell(page).locator('[data-menu-label]')).toHaveText('Posts');
    await page.goBack();
    await expect(shell(page).locator('[data-screen="standby"]')).toBeVisible();
    // The newest post is read now.
    await expect(shell(page).getByRole('button', { name: '1 new post' })).toBeHidden();
    await expect(soft(page, 'left')).toHaveText('Menu');
    await expect(soft(page, 'right')).toHaveText('Posts');
    expect(errors, errors.join('\n')).toEqual([]);
  });

  test('menu: keys 1 to 6, arrows, Esc, and the topic list', async ({ page }) => {
    const errors = watchErrors(page);
    await page.goto('/');
    await soft(page, 'right').tap();
    const label = shell(page).locator('[data-menu-label]');
    await expect(label).toHaveText('Posts');
    await expect(shell(page).locator('[data-menu-index]')).toHaveText('1');
    await expect(shell(page).getByRole('button', { name: 'Open Posts, item 1 of 6' })).toBeVisible();
    await page.keyboard.press('ArrowRight');
    await expect(label).toHaveText('Topics');
    await page.keyboard.press('ArrowLeft');
    await page.keyboard.press('ArrowLeft');
    await expect(label).toHaveText('Games');
    await shell(page).getByRole('button', { name: 'Next item' }).tap();
    await expect(label).toHaveText('Posts');

    const opens = [
      ['1', () => expect(shell(page).locator('[data-list-title]')).toHaveText('Posts')],
      ['2', () => expect(shell(page).locator('[data-list-title]')).toHaveText('Topics')],
      ['3', () => expect(shell(page).locator('[data-text-title]')).toHaveText('Now')],
      ['4', () => expect(shell(page).locator('[data-read]')).toContainText('No NOAA data yet.')],
      ['5', () => expect(shell(page).getByRole('button', { name: /^Phone/ })).toHaveAttribute('aria-pressed', 'true')],
      ['6', () => expect(shell(page).locator('[data-score]')).toBeVisible()],
    ];
    for (const [key, check] of opens) {
      await page.keyboard.press(key);
      await check();
      await page.keyboard.press('Escape');
      await expect(label).toBeVisible();
    }
    await expect(page).toHaveURL('/');

    // Topics filter the post list. Enter is the left softkey.
    await page.keyboard.press('2');
    await shell(page).getByRole('link', { name: 'Rust' }).tap();
    await expect(page).toHaveURL('/topics/rust');
    await expect(shell(page).locator('[data-list-title]')).toHaveText('Rust');
    const rust = await (await page.request.get('/api/topics/rust')).json();
    await expect(shell(page).locator('[data-list] a')).toHaveCount(rust.length);
    await page.keyboard.press('Backspace');
    await expect(shell(page).locator('[data-list-title]')).toHaveText('Topics');

    // The About page is gone. The old address goes to standby.
    await page.goto('/about');
    await expect(page).toHaveURL('/');
    await expect(shell(page).locator('[data-screen="standby"]')).toBeVisible();
    expect(errors, errors.join('\n')).toEqual([]);
  });

  test('a deep link to a post; Options: Jump to section, text size, font', async ({ page }) => {
    const errors = watchErrors(page);
    await page.goto(`/posts/${BAL}`);
    const read = shell(page).locator('[data-read]');
    await expect(read.locator('.ph-title')).toHaveText('Block-level access lists and parallel execution');
    expect(await page.evaluate(() => window.scrollY)).toBe(0);

    await soft(page, 'left').tap();
    const menu = shell(page).getByRole('menu', { name: 'Options' });
    await expect(menu.getByRole('menuitem')).toHaveText(['Top', 'Jump to section', 'Text size: Normal', 'Font: Pixel', 'Copy link']);
    await menu.getByRole('menuitem', { name: 'Jump to section' }).tap();
    await expect(menu.getByRole('menuitem').first()).toHaveText('§ What clients gain');
    await menu.getByRole('menuitem', { name: '§ Why it matters' }).tap();
    await expect(menu).toBeHidden();
    await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(100);
    await expect(shell(page).locator('[data-pages]')).not.toHaveText(/^1\//);

    // Enter opens Options; arrows move; Enter picks.
    await read.focus();
    await page.keyboard.press('Enter');
    await expect(menu).toBeVisible();
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Enter');
    await expect(read).toHaveAttribute('data-size', 'l');
    await expect(menu.getByRole('menuitem', { name: 'Text size: Large' })).toBeVisible();
    await menu.getByRole('menuitem', { name: 'Font: Pixel' }).tap();
    await expect(read).toHaveAttribute('data-clean', '');
    await page.keyboard.press('Escape');
    await expect(menu).toBeHidden();

    // A shared link has no earlier entry: Back goes to the post list.
    await soft(page, 'right').tap();
    await expect(page).toHaveURL('/');
    await expect(shell(page).locator('[data-list-title]')).toHaveText('Posts');
    expect(errors, errors.join('\n')).toEqual([]);
  });

  test('one scroll container: the page scrolls, also from a swipe at the edge', async ({ page }) => {
    const errors = watchErrors(page);
    await page.goto(`/posts/${BAL}`);
    const read = shell(page).locator('[data-read]');
    await expect(read.locator('.ph-title')).toBeVisible();
    const pages = shell(page).locator('[data-pages]');
    await expect(pages).toHaveText(/^1\/[2-9]\d*$/);
    const total = (await pages.textContent()).split('/')[1];

    // No element scrolls up and down. Only the page does.
    const scrollers = await page.evaluate(() => [...document.querySelectorAll('phone-shell *')]
      .filter(el => /auto|scroll/.test(getComputedStyle(el).overflowY) && el.scrollHeight > el.clientHeight + 1)
      .map(el => el.className || el.tagName));
    expect(scrollers).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollHeight > window.innerHeight)).toBe(true);

    // A touch swipe on the right edge, over the thin scroll bar, scrolls the page.
    const cdp = await page.context().newCDPSession(page);
    const swipeUp = async (x, y) => {
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x, y }] });
      for (let i = 1; i <= 10; i++) await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x, y: y - i * 25 }] });
      await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
    };
    await swipeUp(386, 500);
    await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(100);
    // A swipe in the middle scrolls the page too.
    const y = await page.evaluate(() => window.scrollY);
    await swipeUp(200, 300);
    await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(y);

    // The wheel and window.scrollTo work; the end of the post is the last page.
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.mouse.move(200, 400);
    await page.mouse.wheel(0, 400);
    await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(0);
    await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
    await expect(pages).toHaveText(`${total}/${total}`);

    // The softkeys and the status row stay on screen.
    const vh = await page.evaluate(() => window.innerHeight);
    const keys = await soft(page, 'left').boundingBox();
    expect(Math.abs(keys.y + keys.height - vh)).toBeLessThanOrEqual(1);
    expect((await shell(page).locator('.ph-status').boundingBox()).y).toBe(0);
    await expect(soft(page, 'left')).toHaveText('Options');

    // The arrow keys page through the post.
    await page.evaluate(() => window.scrollTo(0, 0));
    await read.focus();
    await page.keyboard.press('PageDown');
    await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(200);
    expect(errors, errors.join('\n')).toEqual([]);
  });

  test('Snake starts, pauses with 5 and on a hidden tab, and says the score', async ({ page }) => {
    const errors = watchErrors(page);
    await page.goto('/');
    await soft(page, 'right').tap();
    await page.keyboard.press('6');
    const status = shell(page).locator('[data-snake-status]');
    await expect(status).toHaveText('Score 0');
    // The snake starts right, towards the food in the same row.
    await expect(status).toHaveText('Score 9', { timeout: 5000 });
    await expect(shell(page).locator('[data-score]')).toHaveText('0009');
    await page.keyboard.press('5');
    await expect(shell(page).locator('[data-banner]')).toHaveText('Paused · press 5');
    const path = await shell(page).locator('[data-snake]').getAttribute('d');
    await page.waitForTimeout(600);
    expect(await shell(page).locator('[data-snake]').getAttribute('d')).toBe(path);
    await shell(page).getByRole('button', { name: 'Resume (5)' }).tap();
    await expect(shell(page).locator('[data-banner]')).toBeHidden();
    await page.evaluate(() => {
      Object.defineProperty(document, 'hidden', { configurable: true, get: () => true });
      document.dispatchEvent(new Event('visibilitychange'));
    });
    await expect(shell(page).locator('[data-banner]')).toHaveText('Paused · press 5');
    // Steer up into the wall: game over.
    await page.keyboard.press('5');
    await page.keyboard.press('2');
    await expect(status).toHaveText(/^Game over\. Score \d+$/, { timeout: 5000 });
    await expect(soft(page, 'left')).toHaveText('Again');
    await soft(page, 'left').tap();
    await expect(status).toHaveText('Score 0');
    expect(errors, errors.join('\n')).toEqual([]);
  });

  test('Profiles switch to General and Night, and back to Phone', async ({ page }) => {
    const errors = watchErrors(page);
    await page.goto('/');
    await soft(page, 'right').tap();
    await page.keyboard.press('5');
    await shell(page).getByRole('button', { name: /^General/ }).tap();
    await expect(page.locator('html')).toHaveAttribute('data-mode', 'general');
    await expect(page.locator('.desktop')).toBeVisible();
    await expect(shell(page)).toBeHidden();
    expect(await page.evaluate(() => localStorage.getItem('logbook.mode'))).toBe('general');
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-mode', 'general');

    // Win98 footer: View: Mobile | Desktop.
    const view = page.locator('.view-switch');
    await expect(view).toHaveText('View: Mobile | Desktop');
    await view.getByRole('button', { name: 'Mobile' }).tap();
    await expect(page.locator('html')).toHaveAttribute('data-mode', 'phone');
    await expect(shell(page)).toBeVisible();

    await soft(page, 'right').tap();
    await page.keyboard.press('5');
    await shell(page).getByRole('button', { name: /^Night/ }).tap();
    await expect(page.locator('html')).toHaveAttribute('data-mode', 'night');
    await expect(page.locator('dos-shell')).toBeVisible();
    const keys = page.getByRole('navigation', { name: 'Function keys' });
    await expect(keys.getByRole('button', { name: 'F8 Mobile' })).toBeVisible();
    const input = page.getByLabel('C:\\LOGBOOK>');
    await input.fill('help');
    await input.press('Enter');
    await expect(page.locator('.dos-help')).toContainText('switch to the phone view');
    await input.fill('mobile');
    await input.press('Enter');
    await expect(page.locator('html')).toHaveAttribute('data-mode', 'phone');
    await expect(shell(page)).toBeVisible();
    await page.reload();
    await expect(page.locator('html')).toHaveAttribute('data-mode', 'phone');

    // F8 in MS-DOS mode goes to phone mode too.
    await page.evaluate(() => localStorage.setItem('logbook.mode', 'night'));
    await page.reload();
    await keys.getByRole('button', { name: 'F8 Mobile' }).tap();
    await expect(page.locator('html')).toHaveAttribute('data-mode', 'phone');
    expect(errors, errors.join('\n')).toEqual([]);
  });

  test('status row: GitHub and X links left of the battery on every screen', async ({ page }) => {
    const errors = watchErrors(page);
    const status = shell(page).locator('.ph-status');
    const check = async label => {
      const gh = status.getByRole('link', { name: 'GitHub (eserilev)' });
      const x = status.getByRole('link', { name: 'X (@0xUncleBill)' });
      await expect(gh).toHaveAttribute('href', 'https://github.com/eserilev');
      await expect(x).toHaveAttribute('href', 'https://x.com/0xUncleBill');
      const batt = await status.locator('.ph-batt').boundingBox();
      const vw = page.viewportSize().width;
      expect(batt.x + batt.width, `${label}: battery on screen`).toBeLessThanOrEqual(vw);
      for (const link of [gh, x]) {
        const box = await link.boundingBox();
        expect(box.width, label).toBeGreaterThanOrEqual(44);
        expect(box.height, label).toBeGreaterThanOrEqual(44);
        expect(box.x + box.width, `${label}: link left of the battery`).toBeLessThanOrEqual(batt.x + 0.5);
      }
      // The time stays in the center of the row.
      const time = await status.locator('[data-time]').boundingBox();
      if (time.width) expect(Math.abs(time.x + time.width / 2 - vw / 2), `${label}: time centered`).toBeLessThanOrEqual(2);
    };
    await page.goto('/');
    await expect(shell(page).locator('[data-clock]')).toHaveText(/\d\d:\d\d/);
    await check('standby');
    await soft(page, 'right').tap();
    await check('menu');
    await page.keyboard.press('6');
    await check('snake');
    await page.goto(`/posts/${BAL}`);
    await expect(shell(page).locator('.ph-body')).toBeVisible();
    await check('reading');
    // The status row sticks to the top while the post scrolls.
    await page.evaluate(() => window.scrollTo(0, 600));
    await expect.poll(() => status.evaluate(el => el.getBoundingClientRect().top)).toBe(0);
    await check('reading, scrolled');
    expect(errors, errors.join('\n')).toEqual([]);
  });

  test('390 px: no horizontal scroll on any screen; 64 px softkeys', async ({ page }) => {
    const errors = watchErrors(page);
    const wide = () => page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    await page.goto('/');
    for (const b of [soft(page, 'left'), soft(page, 'right')]) {
      const box = await b.boundingBox();
      expect(box.height).toBeGreaterThanOrEqual(64);
      expect(box.width).toBeGreaterThanOrEqual(150);
    }
    expect(await wide()).toBeLessThanOrEqual(0);
    await soft(page, 'right').tap();
    for (const key of ['1', '2', '3', '5', '6']) {
      await page.keyboard.press(key);
      expect(await wide(), `menu item ${key}`).toBeLessThanOrEqual(0);
      await page.keyboard.press('Escape');
    }
    await page.goto('/posts/zero-copy-ssz-decoding-in-rust');
    await expect(shell(page).locator('.ph-body pre')).toBeVisible();
    expect(await wide()).toBeLessThanOrEqual(0);
    expect(errors, errors.join('\n')).toEqual([]);
  });
});

test('a desktop never shows phone mode or a switch to it', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('logbook.mode', 'phone'));
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-mode', 'general');
  await expect(page.locator('html')).not.toHaveAttribute('data-device', 'phone');
  await expect(shell(page)).toBeHidden();
  await expect(page.locator('.view-switch')).toBeHidden();
  await expect(page.getByText('View:')).toBeHidden();

  await page.getByRole('button', { name: 'Dark mode (MS-DOS)' }).click();
  const keys = page.getByRole('navigation', { name: 'Function keys' });
  await expect(keys.getByRole('button')).toHaveCount(6);
  await expect(keys.getByRole('button', { name: 'F8 Mobile' })).toBeHidden();
  const input = page.getByLabel('C:\\LOGBOOK>');
  await input.fill('help');
  await input.press('Enter');
  await expect(page.locator('.dos-help')).not.toContainText('MOBILE');
  await input.fill('mobile');
  await input.press('Enter');
  await expect(page.locator('dos-shell [data-out]')).toContainText('Bad command or file name');
  // F8 does nothing on a desktop.
  await input.press('F8');
  await expect(page.locator('dos-shell [data-out] .dos-line.cmd')).toHaveCount(2);
  await expect(page.locator('html')).toHaveAttribute('data-mode', 'night');
  expect(errors, errors.join('\n')).toEqual([]);
});
