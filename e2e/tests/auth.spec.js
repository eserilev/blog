import { test, expect } from '@playwright/test';
import { setupLink, virtualAuthenticator, watchErrors } from './helpers.js';

test('guests see no owner controls, and /write sends them home', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await expect(page.getByRole('link', { name: 'Block-level access lists and parallel execution' })).toBeVisible();
  await expect(page.locator('.tb', { hasText: 'Compose' })).toBeHidden();
  await expect(page.locator('.task', { hasText: 'Notepad' })).toBeHidden();
  await expect(page.locator('#now-open')).toBeHidden();
  await page.goto('/write');
  await expect(page).toHaveURL('/');
  expect(errors, errors.join('\n')).toEqual([]);
});

test('setup link → passkey → sign out → sign in with the passkey', async ({ page }) => {
  const errors = watchErrors(page);
  await virtualAuthenticator(page);

  // Register a passkey with the one-time link.
  const link = setupLink();
  await page.goto(link);
  await page.getByLabel('Name for this passkey:').fill('Test key');
  await page.getByRole('button', { name: 'Create passkey' }).click();
  await expect(page.locator('#setup-status')).toHaveText('Passkey created. You are signed in.');
  await expect(page).toHaveURL('/write');
  await expect(page.locator('.tb', { hasText: 'Compose' })).toBeVisible();
  await expect(page.locator('#passkey-list li')).toHaveCount(1);
  await expect(page.locator('#passkey-list')).toContainText('Test key');

  // The link works once: the same token is now refused.
  const reused = await page.request.post('/auth/register/start', {
    data: { setup_token: new URL(link).hash.slice(1) },
    headers: { Origin: 'http://localhost:18100' },
  });
  expect(reused.status()).toBe(401);

  // Sign out.
  await page.goto('/');
  await page.getByRole('button', { name: 'Sign out' }).click();
  await expect(page.getByRole('button', { name: 'Sign in' })).toBeVisible();
  await expect(page.locator('.tb', { hasText: 'Compose' })).toBeHidden();
  expect((await (await page.request.get('/api/me')).json()).owner).toBe(false);

  // Sign in again with the same passkey.
  await page.getByRole('button', { name: 'Sign in' }).click();
  await page.getByRole('button', { name: 'Sign in with passkey' }).click();
  await expect(page.locator('#signin')).toBeHidden();
  await expect(page.locator('.tb', { hasText: 'Compose' })).toBeVisible();
  expect((await (await page.request.get('/api/me')).json()).owner).toBe(true);

  expect(errors, errors.join('\n')).toEqual([]);
});

test('every page renders with no CSP violations or script errors', async ({ page }) => {
  const errors = watchErrors(page);
  for (const path of ['/', '/topics/rust', '/posts/block-level-access-lists-and-parallel-execution', '/about', '/setup']) {
    await page.goto(path);
    await page.waitForLoadState('networkidle');
  }
  expect(errors, errors.join('\n')).toEqual([]);
});

test('hidden and missing posts show Not found', async ({ page }) => {
  for (const slug of ['epbs-from-a-clients-perspective', 'molten-core-as-a-scheduling-problem', 'nope']) {
    await page.goto(`/posts/${slug}`);
    await expect(page.locator('post-view h1')).toHaveText('Not found');
  }
});

test('the path router changes views without a full page load', async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => { window.__marker = 'same-page'; });
  await page.getByRole('link', { name: 'Rust', exact: true }).click();
  await expect(page).toHaveURL('/topics/rust');
  await expect(page.locator('#list-title')).toHaveText('Writing: Rust');
  await page.getByRole('link', { name: 'Zero-copy SSZ decoding in Rust' }).click();
  await expect(page).toHaveURL('/posts/zero-copy-ssz-decoding-in-rust');
  await expect(page.locator('post-view h1')).toHaveText('Zero-copy SSZ decoding in Rust');
  await page.goBack();
  await expect(page).toHaveURL('/topics/rust');
  expect(await page.evaluate(() => window.__marker)).toBe('same-page');
});
