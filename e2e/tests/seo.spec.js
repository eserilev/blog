// Search engine tests (spec 6.16): the author name in every mode, and post links
// that a crawler finds on a phone.
import { test, expect, devices } from '@playwright/test';
import { watchErrors } from './helpers.js';

const BAL = 'block-level-access-lists-and-parallel-execution';
const NAME = 'Eitan Seri-Levi';
// The project runs Chromium. Keep the iPhone screen, touch, and coarse pointer.
const { defaultBrowserType: _, ...iPhone } = devices['iPhone 13'];

test('Win98: the footer and the post byline show the author', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  const year = new Date().getUTCFullYear();
  await expect(page.locator('section[data-view="home"] .foot-note').first()).toContainText(`© ${year} ${NAME}`);
  await expect(page.locator('#site-author')).toBeVisible();
  await page.goto(`/posts/${BAL}`);
  const byline = page.locator('post-view .byline');
  await expect(byline).toBeVisible();
  await expect(byline).toContainText(`min read · by ${NAME}`);
  expect(errors).toEqual([]);
});

test('Win98: the server writes the post table before the script runs', async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  const page = await context.newPage();
  await page.goto('/');
  await expect(page.locator(`#ns-rows a[href="/posts/${BAL}"]`)).toHaveText('Block-level access lists and parallel execution');
  await context.close();
});

test('MS-DOS: TYPE prints the byline with the author', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Dark mode (MS-DOS)' }).click();
  await expect(page.locator('dos-shell')).toBeVisible();
  const input = page.getByLabel('C:\\LOGBOOK>');
  await input.fill('type blocklev.txt');
  await input.press('Enter');
  const out = page.locator('dos-shell [data-out]');
  await expect(out.locator('h1.dos-title')).toHaveText('Block-level access lists and parallel execution');
  await expect(out).toContainText(`· by ${NAME}`);
});

test.describe('phone', () => {
  test.use(iPhone);

  test('a post shows its title, the author, and the body', async ({ page }) => {
    const errors = watchErrors(page);
    await page.goto(`/posts/${BAL}`);
    await expect(page.locator('html')).toHaveAttribute('data-mode', 'phone');
    const read = page.locator('phone-shell [data-read]');
    await expect(read.locator('.ph-title')).toHaveText('Block-level access lists and parallel execution');
    await expect(read.locator('.ph-meta', { hasText: `by ${NAME}` })).toBeVisible();
    await expect(read.locator('.ph-body h2').first()).toHaveText('What clients gain');
    expect(errors).toEqual([]);
  });

  test('standby has links to the posts in the page, visually hidden', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('html')).toHaveAttribute('data-mode', 'phone');
    const posts = await (await page.request.get('/api/posts')).json();
    const nav = page.locator('phone-shell nav[aria-label="Posts"]');
    // In the accessibility tree, not display: none.
    const display = await nav.evaluate(el => getComputedStyle(el).display);
    expect(display).not.toBe('none');
    await expect(page.getByRole('navigation', { name: 'Posts' })).toBeAttached();
    for (const p of posts) {
      await expect(nav.locator(`a[href="/posts/${p.slug}"]`)).toHaveText(p.title);
    }
    // The standby screen looks the same: the links take no space on screen.
    const box = await nav.boundingBox();
    expect(box.width).toBeLessThanOrEqual(1);
    expect(box.height).toBeLessThanOrEqual(1);
    // A keyboard user who moves the focus into the links sees them.
    await nav.locator('a').first().focus();
    const shown = await nav.boundingBox();
    expect(shown.height).toBeGreaterThan(20);
  });
});
