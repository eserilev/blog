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

// A 2x2 red PNG.
const PNG = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAFklEQVR4nGP8z8DAwMDAxMDAwMDAAAANHQEDasKb6QAAAABJRU5ErkJggg==', 'base64');

test('the editor uploads an image and the preview shows it', async ({ page }) => {
  const errors = watchErrors(page);
  await signIn(page);
  await page.getByRole('button', { name: 'New post' }).click();
  await expect(page.locator('md-editor')).toHaveAttribute('data-loaded', '');
  await page.locator('[data-image-file]').setInputFiles({ name: 'red.png', mimeType: 'image/png', buffer: PNG });
  await expect(page.locator('md-editor [data-status]')).toHaveText('Image added.');
  await expect(page.getByLabel('Post markdown')).toHaveValue(/!\[\]\(\/media\/[0-9a-f]{64}\.png\)/);
  const img = page.locator('[data-preview] img');
  await expect(img).toBeVisible();
  expect(await img.evaluate(el => el.naturalWidth)).toBe(2);
  expect(errors, errors.join('\n')).toEqual([]);
});

test('the surf window says when there is no data', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('surf-report [data-rating]')).toHaveText('No NOAA data yet.');
});
