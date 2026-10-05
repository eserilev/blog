import { test, expect } from '@playwright/test';
import { watchErrors } from './helpers.js';

const BAL = 'block-level-access-lists-and-parallel-execution';

/** Clicks the toolbar button and waits for the terminal. */
async function enterDos(page) {
  await page.getByRole('button', { name: 'Dark mode (MS-DOS)' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-mode', 'night');
  await expect(page.locator('dos-shell')).toBeVisible();
}

/** Types a command at the prompt and presses Enter. */
async function command(page, text) {
  const input = page.getByLabel('C:\\LOGBOOK>');
  await input.fill(text);
  await input.press('Enter');
}

test('first visit is Win98; Dark mode starts MS-DOS mode and it persists', async ({ page }) => {
  const errors = watchErrors(page);
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('data-mode', 'general');
  await expect(page.locator('dos-shell')).toBeHidden();
  await enterDos(page);
  await expect(page.locator('.desktop')).toBeHidden();
  await expect(page.locator('.taskbar')).toBeHidden();
  // The terminal starts empty: only the prompt and the hint.
  await expect(page.locator('dos-shell [data-out]')).toBeEmpty();
  await expect(page.locator('.dos-hint')).toHaveText('Type HELP for a list of commands. Type WIN to go back to Windows.');
  expect(await page.evaluate(() => localStorage.getItem('logbook.mode'))).toBe('night');

  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-mode', 'night');
  await expect(page.locator('dos-shell')).toBeVisible();
  await expect(page.locator('.desktop')).toBeHidden();

  await command(page, 'ver');
  await expect(page.locator('dos-shell [data-out]')).toContainText('Logbook DOS Version 6.22');
  await command(page, 'foo');
  await expect(page.locator('dos-shell [data-out]')).toContainText('Bad command or file name');
  await command(page, 'cls');
  await expect(page.locator('dos-shell [data-out]')).toBeEmpty();

  // WIN goes back to Windows and saves that choice.
  await command(page, 'win');
  await expect(page.locator('html')).toHaveAttribute('data-mode', 'general');
  await expect(page.locator('.desktop')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Dark mode (MS-DOS)' })).toBeFocused();
  expect(await page.evaluate(() => localStorage.getItem('logbook.mode'))).toBe('general');
  await page.reload();
  await expect(page.locator('dos-shell')).toBeHidden();
  expect(errors, errors.join('\n')).toEqual([]);
});

test('DIR lists the posts as files; TYPE prints one; Back works', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await enterDos(page);
  await command(page, 'dir');
  const out = page.locator('dos-shell [data-out]');
  const row = out.locator('.dos-row', { hasText: BAL });
  await expect(row.locator('.c-name')).toHaveText('BLOCKLEV');
  await expect(row.locator('.c-ext')).toHaveText('TXT');
  await expect(row.locator('.c-date')).toHaveText('09-28-26');
  await expect(row.locator('a.c-long')).toHaveAttribute('href', `/posts/${BAL}`);
  await expect(out).toContainText('6 file(s)');
  await expect(out.locator('.dos-row .c-name', { hasText: /^ABOUT$/ })).toHaveCount(0);
  await expect(out).toContainText('Example: TYPE BLOCKLEV.TXT');

  await command(page, 'type blocklev.txt');
  await expect(page).toHaveURL(`/posts/${BAL}`);
  await expect(page).toHaveTitle(/^Block-level access lists and parallel execution · /);
  const title = out.locator('h1.dos-title');
  await expect(title).toHaveText('Block-level access lists and parallel execution');
  await expect(title).toBeFocused();
  await expect(out.locator('.dos-prose h2').first()).toHaveText('What clients gain');
  await expect(out.locator('.dos-prose pre[data-lang="rust"]')).toBeVisible();
  await expect(page.locator('dos-shell [data-status]')).toHaveText('Showing BLOCKLEV.TXT');

  // Back shows the DIR output again.
  await page.goBack();
  await expect(page).toHaveURL('/');
  await expect(out.locator('.dos-row', { hasText: BAL })).toBeVisible();

  // The long name is a link; TYPE also takes the slug and the row number.
  await out.getByRole('link', { name: 'zero-copy-ssz-decoding-in-rust' }).click();
  await expect(page).toHaveURL('/posts/zero-copy-ssz-decoding-in-rust');
  await expect(out.locator('h1.dos-title')).toHaveText('Zero-copy SSZ decoding in Rust');
  await command(page, `type ${BAL}`);
  await expect(page).toHaveURL(`/posts/${BAL}`);
  // Row 1 of DIR is the newest post.
  const [newest] = await (await page.request.get('/api/posts')).json();
  await command(page, 'type 1');
  await expect(page).toHaveURL(`/posts/${newest.slug}`);
  await expect(out.locator('h1.dos-title')).toHaveText(newest.title);
  // There is no About file. The old address goes to the empty prompt.
  await command(page, 'type about.txt');
  await expect(out).toContainText('File not found - ABOUT.TXT');
  await page.goto('/about');
  await expect(page).toHaveURL('/');
  await expect(page.locator('dos-shell [data-out]')).toBeEmpty();

  await command(page, 'type nope.txt');
  await expect(out).toContainText('File not found - NOPE.TXT');
  await command(page, 'type');
  await expect(out).toContainText('Required parameter missing');
  expect(errors, errors.join('\n')).toEqual([]);
});

test('a deep link in MS-DOS mode prints the post; topics and missing posts work', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('logbook.mode', 'night'));
  await page.goto(`/posts/${BAL}`);
  const out = page.locator('dos-shell [data-out]');
  await expect(out.locator('.dos-line.cmd').first()).toHaveText('C:\\LOGBOOK>TYPE BLOCKLEV.TXT');
  await expect(out.locator('h1.dos-title')).toHaveText('Block-level access lists and parallel execution');
  await expect(page).toHaveTitle(/^Block-level access lists/);

  await page.goto('/topics/rust');
  await expect(out).toContainText('Directory of C:\\LOGBOOK\\RUST');
  await expect(out.locator('.dos-row')).toHaveCount(1);

  await page.goto('/posts/epbs-from-a-clients-perspective');
  await expect(out).toContainText('File not found - EPBS-FROM-A-CLIENTS-PERSPECTIVE');
  expect(errors.filter(e => !e.includes('404')), errors.join('\n')).toEqual([]);
});

test('HELP, NOW, SURF, and the F-key bar', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await enterDos(page);
  const out = page.locator('dos-shell [data-out]');
  const keys = page.getByRole('navigation', { name: 'Function keys' });
  await keys.getByRole('button', { name: 'F1 Help' }).click();
  await expect(out.locator('.dos-help')).toContainText('get the swell report for Redondo Beach');
  await keys.getByRole('button', { name: 'F3 Now' }).click();
  await expect(out).toContainText('C:\\LOGBOOK>NOW');
  await keys.getByRole('button', { name: 'F4 Surf' }).click();
  await expect(out).toContainText('No NOAA data yet.');
  await keys.getByRole('button', { name: 'F9 Cls' }).click();
  await expect(out).toBeEmpty();
  await keys.getByRole('button', { name: 'F10 Win' }).click();
  await expect(page.locator('.desktop')).toBeVisible();
  expect(errors, errors.join('\n')).toEqual([]);
});

test('LINKS prints the GitHub and X links; HELP lists LINKS', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await enterDos(page);
  const out = page.locator('dos-shell [data-out]');
  await command(page, 'help');
  await expect(out.locator('.dos-help')).toContainText('LINKS');
  await expect(out.locator('.dos-help')).toContainText('show my GitHub and X links');
  await command(page, 'links');
  const lines = out.locator('.dos-entry').last().locator('.dos-line:not(.cmd)');
  await expect(lines).toHaveText(['GITHUB  https://github.com/eserilev', 'X       https://x.com/0xUncleBill']);
  await expect(lines.nth(0).locator('a')).toHaveAttribute('href', 'https://github.com/eserilev');
  await expect(lines.nth(1).locator('a')).toHaveAttribute('href', 'https://x.com/0xUncleBill');
  await expect(lines.locator('a')).toHaveText(['https://github.com/eserilev', 'https://x.com/0xUncleBill']);
  expect(errors, errors.join('\n')).toEqual([]);
});

for (const width of [390, 1280]) {
  test(`top bar at ${width} px: GitHub and X links on one line`, async ({ browser }) => {
    const phone = width < 600;
    const context = await browser.newContext({ viewport: { width, height: phone ? 844 : 800 }, hasTouch: phone, isMobile: phone });
    const page = await context.newPage();
    const errors = watchErrors(page);
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('logbook.mode', 'night'));
    await page.reload();
    const bar = page.locator('.dos-bar');
    await expect(bar).toBeVisible();
    await expect(bar.getByRole('link', { name: 'GitHub (eserilev)' })).toHaveAttribute('href', 'https://github.com/eserilev');
    await expect(bar.getByRole('link', { name: 'X (@0xUncleBill)' })).toHaveAttribute('href', 'https://x.com/0xUncleBill');
    await expect(bar.getByRole('link', { name: 'GitHub (eserilev)' })).toHaveText('GITHUB');
    await expect(page.locator('#dos-clock')).toBeVisible();
    const r = await bar.evaluate(el => {
      const box = el.getBoundingClientRect();
      const kids = [...el.children].filter(k => k.getClientRects().length).map(k => k.getBoundingClientRect());
      return {
        overflow: el.scrollWidth - el.clientWidth,
        right: Math.max(...kids.map(k => k.right)) - box.right,
        oneLine: kids.every(k => Math.abs(k.top - kids[0].top) < 1 && k.height < 30),
      };
    });
    expect(r.overflow).toBeLessThanOrEqual(0);
    expect(r.right).toBeLessThanOrEqual(0);
    expect(r.oneLine).toBe(true);
    expect(errors, errors.join('\n')).toEqual([]);
    await context.close();
  });
}

test('keyboard only: F-keys, history, and Tab to the links', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Dark mode (MS-DOS)' }).focus();
  await page.keyboard.press('Enter');
  const input = page.getByLabel('C:\\LOGBOOK>');
  await expect(input).toBeFocused();
  const out = page.locator('dos-shell [data-out]');
  await page.keyboard.press('F2');
  await expect(out.locator('.dos-row')).toHaveCount(6);
  await page.keyboard.type('ver');
  await page.keyboard.press('Enter');
  await page.keyboard.press('ArrowUp');
  await expect(input).toHaveValue('VER');
  await page.keyboard.press('ArrowUp');
  await expect(input).toHaveValue('DIR');
  await page.keyboard.press('ArrowDown');
  await expect(input).toHaveValue('VER');
  await input.fill('');
  // Shift+Tab reaches the last DIR link; Enter opens it.
  await page.keyboard.press('Shift+Tab');
  await expect(page.locator(':focus')).toHaveClass('c-long');
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/\/posts\//);
  await expect(out.locator('h1.dos-title')).toBeFocused();
  await page.keyboard.press('F10');
  await expect(page.locator('.desktop')).toBeVisible();
  expect(errors, errors.join('\n')).toEqual([]);
});

test('a 390 px phone: F-key bar on screen, no horizontal scroll', async ({ browser }) => {
  const context = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });
  const page = await context.newPage();
  const errors = watchErrors(page);
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('logbook.mode', 'night'));
  await page.reload();
  const keys = page.getByRole('navigation', { name: 'Function keys' });
  await expect(keys).toBeInViewport();
  for (const b of await keys.getByRole('button').all()) expect((await b.boundingBox()).height).toBeGreaterThanOrEqual(44);
  // No autofocus on touch: the keyboard stays closed.
  await expect(page.getByLabel('C:\\LOGBOOK>')).not.toBeFocused();
  await keys.getByRole('button', { name: 'F2 Dir' }).tap();
  await expect(page.locator('.dos-row').first()).toBeVisible();
  await expect(page.locator('.dos-row .c-date').first()).toBeHidden();
  await page.locator('.dos-row a', { hasText: BAL }).tap();
  await expect(page.locator('h1.dos-title')).toBeVisible();
  const scroll = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(scroll).toBeLessThanOrEqual(0);
  await expect(keys).toBeInViewport();
  expect(errors, errors.join('\n')).toEqual([]);
  await context.close();
});
