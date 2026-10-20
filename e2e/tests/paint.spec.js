// Paint (spec 11.9): the WASM core loads on demand, the tools change pixels, and the
// files save and open on the visitor's computer.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { test, expect } from '@playwright/test';
import { watchErrors } from './helpers.js';

const FIXTURES = path.join(path.dirname(fileURLToPath(import.meta.url)), 'fixtures');

const item = (page, name) => page.getByRole('menu', { name: 'Start' }).getByRole('menuitem', { name, exact: true });
const paint = page => page.getByRole('region', { name: 'Paint' });
const canvas = page => paint(page).getByLabel('Drawing area');

async function openPaint(page) {
  await page.getByRole('button', { name: 'Start' }).click();
  await item(page, 'Programs').click();
  await item(page, 'Accessories').click();
  await item(page, 'Paint').click();
  await expect(paint(page)).toBeVisible();
  await expect(paint(page).locator('.p-status')).toContainText('560 × 340');
}

/** The RGBA bytes of one pixel of the picture. */
const pixel = (page, x, y) => canvas(page).evaluate((c, [px, py]) => [...c.getContext('2d').getImageData(px, py, 1, 1).data], [x, y]);
const size = page => canvas(page).evaluate(c => [c.width, c.height]);

/** Drags on the canvas from (x0, y0) to (x1, y1), in picture pixels (zoom 1). */
async function drag(page, [x0, y0], [x1, y1]) {
  const box = await canvas(page).boundingBox();
  await page.mouse.move(box.x + x0 + 0.5, box.y + y0 + 0.5);
  await page.mouse.down();
  await page.mouse.move(box.x + x1 + 0.5, box.y + y1 + 0.5, { steps: 8 });
  await page.mouse.up();
}
const tool = (page, name) => paint(page).getByRole('button', { name, exact: true }).click();
async function menu(page, top, name) {
  await paint(page).getByRole('button', { name: top, exact: true }).click();
  await page.getByRole('menuitem', { name, exact: true }).click();
}

test('nothing of Paint loads before it opens; then the WASM core loads once', async ({ page }) => {
  const errors = watchErrors(page);
  const loaded = [];
  page.on('request', r => { if (/paint/i.test(new URL(r.url()).pathname)) loaded.push(new URL(r.url()).pathname); });
  await page.goto('/');
  await page.waitForLoadState('networkidle');
  expect(loaded).toEqual([]);
  await openPaint(page);
  expect(loaded.sort()).toEqual(['/static/css/paint.css', '/static/js/paint.js', '/static/wasm/logbook_paint.wasm']);
  expect(errors, errors.join('\n')).toEqual([]);
});

test('pencil, fill, undo, and repeat change the pixels', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await openPaint(page);
  expect(await pixel(page, 50, 50)).toEqual([255, 255, 255, 255]);

  // The pencil is the first tool. It draws black, one pixel wide.
  await drag(page, [20, 50], [80, 50]);
  expect(await pixel(page, 50, 50)).toEqual([0, 0, 0, 255]);
  expect(await pixel(page, 50, 52)).toEqual([255, 255, 255, 255]);

  // Fill With Color in red: the white area around the line.
  await paint(page).getByRole('button', { name: 'Color #ff0000' }).click();
  await tool(page, 'Fill With Color');
  await drag(page, [10, 10], [10, 10]);
  expect(await pixel(page, 300, 300)).toEqual([255, 0, 0, 255]);
  expect(await pixel(page, 50, 50)).toEqual([0, 0, 0, 255]);

  // Undo the fill, then the line. Repeat brings the line back.
  await canvas(page).focus();
  await page.keyboard.press('Control+z');
  expect(await pixel(page, 300, 300)).toEqual([255, 255, 255, 255]);
  await menu(page, 'Edit', 'Undo');
  expect(await pixel(page, 50, 50)).toEqual([255, 255, 255, 255]);
  await page.keyboard.press('Control+y');
  expect(await pixel(page, 50, 50)).toEqual([0, 0, 0, 255]);
  expect(errors, errors.join('\n')).toEqual([]);
});

test('a selection moves, and flip and rotate change the size', async ({ page }) => {
  await page.goto('/');
  await openPaint(page);
  // A solid black rectangle.
  await tool(page, 'Rectangle');
  await paint(page).getByRole('button', { name: 'Solid' }).click();
  await drag(page, [10, 10], [29, 29]);
  expect(await pixel(page, 20, 20)).toEqual([0, 0, 0, 255]);

  // Select it and drag it 100 px to the right.
  await tool(page, 'Select');
  await drag(page, [5, 5], [35, 35]);
  await drag(page, [20, 20], [120, 20]);
  await tool(page, 'Pencil');
  expect(await pixel(page, 20, 20)).toEqual([255, 255, 255, 255]);
  expect(await pixel(page, 120, 20)).toEqual([0, 0, 0, 255]);

  // Rotate the picture by 90°: the sides swap.
  await menu(page, 'Image', 'Flip/Rotate…');
  const dlg = page.getByRole('dialog', { name: 'Flip and Rotate' });
  await dlg.getByLabel('Rotate by angle').check();
  await dlg.getByLabel('90°').check();
  await dlg.getByRole('button', { name: 'OK' }).click();
  expect(await size(page)).toEqual([340, 560]);
  await expect(paint(page).locator('.p-status')).toContainText('340 × 560');

  // Stretch to 50 % wide, then skew by 10°: the picture grows by tan(10°) × height.
  await menu(page, 'Image', 'Stretch/Skew…');
  const st = page.getByRole('dialog', { name: 'Stretch and Skew' });
  await st.getByLabel('Horizontal:').first().fill('50');
  await st.getByRole('button', { name: 'OK' }).click();
  expect(await size(page)).toEqual([170, 560]);
  await menu(page, 'Image', 'Stretch/Skew…');
  await st.getByLabel('Horizontal:').nth(1).fill('10');
  await st.getByRole('button', { name: 'OK' }).click();
  expect(await size(page)).toEqual([170 + Math.floor((559 * 176) / 1000), 560]);

  // Attributes sets the size; Undo goes back.
  await menu(page, 'Image', 'Attributes…');
  const at = page.getByRole('dialog', { name: 'Attributes' });
  await at.getByLabel('Width:').fill('64');
  await at.getByLabel('Height:').fill('48');
  await at.getByRole('button', { name: 'OK' }).click();
  expect(await size(page)).toEqual([64, 48]);
  await canvas(page).focus();
  await page.keyboard.press('Control+z');
  expect((await size(page))[1]).toBe(560);
});

test('Save downloads a PNG; Open loads a PNG at its size', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await openPaint(page);
  await drag(page, [20, 20], [60, 60]);
  await menu(page, 'File', 'Save');
  const dlg = page.getByRole('dialog', { name: 'Save As' });
  await expect(dlg.getByLabel('File name:')).toHaveValue('untitled');
  await dlg.getByLabel('File name:').fill('waves');
  const download = page.waitForEvent('download');
  await dlg.getByRole('button', { name: 'Save' }).click();
  const file = await download;
  expect(file.suggestedFilename()).toBe('waves.png');
  const bytes = fs.readFileSync(await file.path());
  expect([...bytes.subarray(0, 8)]).toEqual([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  await expect(paint(page).locator('.titlebar .ttl')).toHaveText('waves - Paint');

  // Open: no unsaved changes, so no question. The picture takes the size of the file.
  const chooser = page.waitForEvent('filechooser');
  await menu(page, 'File', 'Open…');
  await (await chooser).setFiles(path.join(FIXTURES, 'small.png'));
  await expect(paint(page).locator('.titlebar .ttl')).toHaveText('small - Paint');
  expect(await size(page)).toEqual([37, 23]);
  expect(await pixel(page, 5, 5)).toEqual([0, 128, 0, 255]);
  expect(errors, errors.join('\n')).toEqual([]);
});

test('closing with changes asks to save; No closes and the next open is new', async ({ page }) => {
  await page.goto('/');
  await openPaint(page);
  await drag(page, [20, 20], [60, 60]);
  await paint(page).getByRole('button', { name: 'Close' }).click();
  const ask = page.getByRole('dialog', { name: 'Paint' });
  await expect(ask).toContainText('Save changes to untitled?');
  await ask.getByRole('button', { name: 'Cancel' }).click();
  await expect(paint(page)).toBeVisible();
  await paint(page).getByRole('button', { name: 'Close' }).click();
  await ask.getByRole('button', { name: 'No' }).click();
  await expect(paint(page)).toBeHidden();
  await openPaint(page);
  expect(await pixel(page, 40, 40)).toEqual([255, 255, 255, 255]);
});

test('Set As Wallpaper puts the picture behind the desktop, and it stays after a reload', async ({ page }) => {
  const errors = watchErrors(page);
  const wall = () => page.locator('[data-wallpaper]').evaluate(w => [w.style.backgroundImage.slice(0, 27), w.style.backgroundRepeat]);
  await page.goto('/');
  await openPaint(page);
  await drag(page, [20, 20], [60, 60]);
  await menu(page, 'File', 'Set As Wallpaper (Centered)');
  expect(await wall()).toEqual(['url("data:image/png;base64,', 'no-repeat']);
  await page.reload();
  await expect.poll(wall).toEqual(['url("data:image/png;base64,', 'no-repeat']);
  expect(errors, errors.join('\n')).toEqual([]);
});
