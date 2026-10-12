// Notepad for visitors (spec 11.9): Save downloads a file, Open reads one, Find
// finds text, and unsaved changes ask first. Nothing goes to the server.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, expect } from '@playwright/test';
import { watchErrors } from './helpers.js';

const FIXTURES = path.join(path.dirname(fileURLToPath(import.meta.url)), 'fixtures');
const notepad = page => page.getByRole('region', { name: 'Notepad' });
const text = page => notepad(page).getByLabel('Notepad text');

async function openNotepad(page) {
  await page.goto('/');
  await page.getByRole('button', { name: 'Start' }).click();
  const menu = page.getByRole('menu', { name: 'Start' });
  await menu.getByRole('menuitem', { name: 'Programs' }).click();
  await menu.getByRole('menuitem', { name: 'Accessories' }).click();
  await menu.getByRole('menuitem', { name: 'Notepad' }).click();
  await expect(text(page)).toBeFocused();
}
async function menu(page, top, name) {
  await notepad(page).getByRole('button', { name: top, exact: true }).click();
  await page.getByRole('menuitem', { name, exact: true }).click();
}

test('Notepad has only the placeholder, and Save downloads a .md file', async ({ page }) => {
  const errors = watchErrors(page);
  const posts = [];
  page.on('request', r => { if (r.method() !== 'GET') posts.push(r.url()); });
  await openNotepad(page);
  await expect(text(page)).toHaveAttribute('placeholder', 'Write in Markdown');
  await expect(notepad(page).locator('.titlebar .ttl')).toHaveText('Untitled - Notepad');

  await text(page).fill('# Hello\n\nA note from a visitor.');
  await page.keyboard.press('Control+s');
  const dlg = page.getByRole('dialog', { name: 'Save As' });
  await expect(dlg.getByLabel('Save as type:')).toHaveValue('md');
  await dlg.getByLabel('File name:').fill('hello');
  const download = page.waitForEvent('download');
  await dlg.getByRole('button', { name: 'Save' }).click();
  const file = await download;
  expect(file.suggestedFilename()).toBe('hello.md');
  expect(fs.readFileSync(await file.path(), 'utf8')).toBe('# Hello\n\nA note from a visitor.');
  await expect(notepad(page).locator('.titlebar .ttl')).toHaveText('hello.md - Notepad');
  await expect(page.locator('.taskbar [data-task="notepad"]')).toHaveText('hello.md - Notepad');

  // Save again: no dialog, the same name. Save As can choose .txt.
  await text(page).press('End');
  await page.keyboard.type(' More.');
  const again = page.waitForEvent('download');
  await menu(page, 'File', 'Save');
  expect((await again).suggestedFilename()).toBe('hello.md');
  await menu(page, 'File', 'Save As…');
  await dlg.getByLabel('Save as type:').selectOption('txt');
  const txt = page.waitForEvent('download');
  await dlg.getByRole('button', { name: 'Save' }).click();
  expect((await txt).suggestedFilename()).toBe('hello.txt');
  expect(posts).toEqual([]);
  expect(errors, errors.join('\n')).toEqual([]);
});

test('Open reads a local .md file; Find finds text', async ({ page }) => {
  const errors = watchErrors(page);
  await openNotepad(page);
  const chooser = page.waitForEvent('filechooser');
  await menu(page, 'File', 'Open…');
  await (await chooser).setFiles(path.join(FIXTURES, 'note.md'));
  await expect(text(page)).toHaveValue(fs.readFileSync(path.join(FIXTURES, 'note.md'), 'utf8'));
  await expect(notepad(page).locator('.titlebar .ttl')).toHaveText('note.md - Notepad');

  await menu(page, 'Search', 'Find…');
  const find = page.getByRole('dialog', { name: 'Find' });
  await find.getByLabel('Find what:').fill('swell');
  await find.getByRole('button', { name: 'Find Next' }).click();
  const selected = () => text(page).evaluate(t => t.value.slice(t.selectionStart, t.selectionEnd));
  expect(await selected()).toBe('swell');
  await find.getByRole('button', { name: 'Cancel' }).click();
  await expect(find).toBeHidden();

  // F3 finds the next one. A word that is not there gives a message.
  await menu(page, 'Search', 'Find…');
  await find.getByLabel('Find what:').fill('seconds');
  await find.getByLabel('Match case').check();
  await find.getByRole('button', { name: 'Find Next' }).click();
  expect(await selected()).toBe('seconds');
  await find.getByRole('button', { name: 'Find Next' }).click();
  const msg = page.getByRole('dialog', { name: 'Notepad' });
  await expect(msg).toContainText('Cannot find "seconds"');
  await msg.getByRole('button', { name: 'OK' }).click();
  await find.getByRole('button', { name: 'Cancel' }).click();
  expect(errors, errors.join('\n')).toEqual([]);
});

test('unsaved changes ask before New and before close', async ({ page }) => {
  await openNotepad(page);
  await text(page).fill('draft');
  await menu(page, 'File', 'New');
  const ask = page.getByRole('dialog', { name: 'Notepad' });
  await expect(ask).toContainText('The text in the Untitled file has changed.');
  await ask.getByRole('button', { name: 'Cancel' }).click();
  await expect(text(page)).toHaveValue('draft');
  await menu(page, 'File', 'New');
  await ask.getByRole('button', { name: 'No' }).click();
  await expect(text(page)).toHaveValue('');

  await text(page).fill('again');
  await notepad(page).getByRole('button', { name: 'Close' }).click();
  await expect(ask).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(notepad(page)).toBeVisible();
  await notepad(page).getByRole('button', { name: 'Close' }).click();
  await ask.getByRole('button', { name: 'No' }).click();
  await expect(notepad(page)).toBeHidden();
  await expect(page.locator('.taskbar [data-task="notepad"]')).toBeHidden();
});

test('Edit menu: Time/Date, Select All, and Word Wrap', async ({ page }) => {
  await openNotepad(page);
  await page.keyboard.press('F5');
  await expect(text(page)).toHaveValue(/^\d{1,2}:\d\d [AP]M \d{1,2}\/\d{1,2}\/\d{4}$/);
  await menu(page, 'Edit', 'Select All');
  expect(await text(page).evaluate(t => t.selectionEnd - t.selectionStart)).toBeGreaterThan(5);
  await notepad(page).getByRole('button', { name: 'Edit', exact: true }).click();
  await expect(page.getByRole('menuitemcheckbox', { name: 'Word Wrap' })).toHaveAttribute('aria-checked', 'true');
  await page.getByRole('menuitemcheckbox', { name: 'Word Wrap' }).click();
  await expect(text(page)).toHaveClass(/nowrap/);
});
