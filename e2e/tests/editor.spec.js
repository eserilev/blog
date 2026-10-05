import { test, expect } from '@playwright/test';
import { signIn, watchErrors } from './helpers.js';

test('write, preview, save, publish, read', async ({ page }) => {
  const errors = watchErrors(page);
  await signIn(page);

  // A new post opens in the editor.
  await page.getByRole('button', { name: 'New post' }).click();
  await expect(page).toHaveURL(/\/write\/\d+$/);
  const editor = page.locator('md-editor');
  await expect(editor).toHaveAttribute('data-loaded', '');

  await page.getByLabel('Title').fill('Testing the editor');
  await page.getByLabel('Summary').fill('A post written by the browser test.');
  await page.getByLabel('Topic').selectOption('rust');
  await page.getByLabel('Tags').fill('Rust, Testing');
  await page.getByLabel('Post markdown').fill('# Hello\n\nSome **bold** text.\n\n```rust\nfn main() {}\n```\n\n<b>raw</b>');

  // The WASM preview renders on input.
  const preview = page.locator('[data-preview]');
  await expect(preview.locator('h1')).toHaveText('Hello');
  await expect(preview.locator('strong')).toHaveText('bold');
  await expect(preview.locator('pre[data-lang="rust"] span[class^="hl-"]').first()).toBeVisible();
  await expect(preview).toContainText('<b>raw</b>');
  await expect(page.locator('[data-dirty]')).toBeVisible();

  // Save keeps it a draft; guests cannot see it.
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.locator('[data-dirty]')).toBeHidden();
  await expect(page.locator('[data-file]')).toHaveText('testing-the-editor.md');
  expect((await page.request.get('/api/posts/testing-the-editor')).status()).toBe(404);

  // Publish.
  await page.getByLabel('Public: anyone can read it').check();
  await page.getByRole('button', { name: 'Publish' }).click();
  await expect(page.locator('md-editor [data-toast]')).toHaveText('Post published. Anyone can read it now.');

  // Preview = server (spec 7.2): the published HTML equals the preview HTML.
  const previewHtml = await preview.innerHTML();
  const server = await (await page.request.get('/api/posts/testing-the-editor')).json();
  expect(server.body_html).toBe(previewHtml);
  expect(server.tags).toEqual(['rust', 'testing']);

  // The post is live.
  await page.goto('/posts/testing-the-editor');
  await expect(page.locator('post-view h1.t')).toHaveText('Testing the editor');
  await expect(page.locator('post-view .prose h1')).toHaveText('Hello');

  expect(errors, errors.join('\n')).toEqual([]);
});

test('a save from an old tab is a conflict', async ({ page, context }) => {
  await signIn(page);
  await page.getByRole('button', { name: 'New post' }).click();
  await expect(page.locator('md-editor')).toHaveAttribute('data-loaded', '');
  const url = page.url();

  // A second tab opens the same post, then saves first.
  const tab2 = await context.newPage();
  await tab2.goto(url);
  await expect(tab2.locator('md-editor')).toHaveAttribute('data-loaded', '');
  await tab2.getByLabel('Title').fill('Saved in tab two');
  await tab2.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(tab2.locator('[data-dirty]')).toBeHidden();

  // The first tab still has the old version.
  await page.getByLabel('Title').fill('Saved in tab one');
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.locator('md-editor [data-status]')).toHaveText('Conflict: reload to see the newer version.');
  await expect(page.locator('[data-dirty]')).toBeVisible();

  // The server kept tab two's save.
  await page.reload();
  await expect(page.getByLabel('Title')).toHaveValue('Saved in tab two');
});

test('delete needs two clicks', async ({ page }) => {
  await signIn(page);
  await page.getByRole('button', { name: 'New post' }).click();
  await expect(page.locator('md-editor')).toHaveAttribute('data-loaded', '');
  const id = page.url().split('/').pop();
  await page.getByRole('button', { name: 'Delete' }).click();
  await expect(page.getByRole('button', { name: 'Click again to delete' })).toBeVisible();
  await page.getByRole('button', { name: 'Click again to delete' }).click();
  await expect(page).toHaveURL('/write');
  expect((await page.request.get(`/api/owner/posts/${id}`)).status()).toBe(404);
});
