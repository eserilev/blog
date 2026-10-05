import { test, expect } from '@playwright/test';
import { signIn, watchErrors } from './helpers.js';

test('the owner edits the Now box; guests see it', async ({ page, browser }) => {
  const errors = watchErrors(page);
  await signIn(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Edit' }).click();
  await page.getByLabel('Edit the Now section (markdown)').fill('- Working on **ePBS**\n- Surfing');
  await page.getByRole('button', { name: 'Save' }).click();
  await expect(page.locator('#now-view strong')).toHaveText('ePBS');
  await expect(page.locator('#now-date')).toContainText('Updated');

  // A guest in a fresh browser sees it, with no Edit button.
  const guest = await browser.newPage();
  await guest.goto('/');
  await expect(guest.locator('#now-view strong')).toHaveText('ePBS');
  await expect(guest.getByRole('button', { name: 'Edit' })).toBeHidden();
  expect(errors, errors.join('\n')).toEqual([]);
});

test('the visitor counter shows seven digits that count page loads', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#lcd b')).toHaveCount(7);
  const read = async () => Number((await page.locator('#lcd').innerText()).replace(/\D/g, ''));
  await expect.poll(read).toBeGreaterThan(0);
  const before = await read();
  await page.reload();
  await expect.poll(read).toBeGreaterThan(before);
});

test('the RSS link opens the feed, not a page view', async ({ page }) => {
  await page.goto('/');
  const [res] = await Promise.all([
    page.waitForResponse('**/feed.xml'),
    page.getByRole('link', { name: 'RSS feed' }).click(),
  ]);
  expect(res.headers()['content-type']).toContain('application/rss+xml');
});
