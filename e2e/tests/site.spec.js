import { test, expect } from '@playwright/test';
import { signIn, watchErrors } from './helpers.js';

test('the owner edits the title section; guests see it', async ({ page, browser }) => {
  const errors = watchErrors(page);
  await signIn(page);
  const form = page.locator('#settings');
  await expect(form.getByLabel('Title', { exact: true })).not.toHaveValue('');
  await form.getByLabel('Title', { exact: true }).fill('Uncle Bill');
  await form.getByLabel('Subtitle').fill('Software Engineering · Gaming · Random Fun');
  await form.getByLabel('Intro').fill('Hello **world**');
  await form.getByRole('button', { name: 'Save' }).click();
  await expect(page.locator('#settings-status')).toHaveText('Saved.');
  await expect(page).toHaveTitle('Compose · Uncle Bill');

  const guest = await browser.newPage();
  await guest.goto('/');
  await expect(guest).toHaveTitle('Uncle Bill');
  await expect(guest.locator('#site-title')).toHaveText('Uncle Bill');
  await expect(guest.locator('#site-subtitle')).toHaveText('Software Engineering · Gaming · Random Fun');
  await expect(guest.locator('#site-intro strong')).toHaveText('world');
  expect(errors, errors.join('\n')).toEqual([]);
});

test('the owner adds, renames, and deletes a topic', async ({ page, browser }) => {
  const errors = watchErrors(page);
  await signIn(page);
  await page.getByLabel('New topic:').fill('Free Diving');
  await page.locator('#topic-add').getByRole('button', { name: 'Add' }).click();
  await expect(page.locator('#topics-status')).toHaveText('Topic added: /topics/free-diving');
  // The sidebar and the editor's list have it at once.
  await expect(page.locator('#topic-links a[href="/topics/free-diving"]')).toHaveText('Free Diving');
  await expect(page.locator('#ed-topic option[value="free-diving"]')).toHaveText('Free Diving');

  // Rename it and move it to the top.
  await page.getByLabel('Name of topic free-diving').fill('Freediving');
  for (let i = 0; i < 10; i++) await page.getByRole('button', { name: 'Move free-diving up' }).click();
  await page.getByRole('button', { name: 'Save names and order' }).click();
  await expect(page.locator('#topics-status')).toHaveText('Saved.');

  const guest = await browser.newPage();
  await guest.goto('/topics/free-diving');
  await expect(guest).toHaveTitle(/^Freediving · /);
  await expect(guest.locator('#topic-links li').first()).toHaveText('Freediving');
  await expect(guest.locator('#list-title')).toHaveText('Writing: Freediving');

  // A topic with posts stays.
  await page.getByRole('button', { name: 'Delete rust' }).click();
  await expect(page.locator('#topics-status')).toContainText('this topic has posts');
  await page.getByRole('button', { name: 'Delete free-diving' }).click();
  await expect(page.locator('#topics-status')).toHaveText('Topic deleted.');
  await expect(page.locator('#topic-links a[href="/topics/free-diving"]')).toHaveCount(0);
  expect(errors.filter(e => !e.includes('409')), errors.join('\n')).toEqual([]);
});
