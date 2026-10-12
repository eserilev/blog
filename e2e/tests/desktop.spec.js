// The desktop (spec 11.9): icons, Documents (the public posts as .md files), and the
// Recycle Bin, which never deletes or changes anything.
import { test, expect } from '@playwright/test';
import { watchErrors } from './helpers.js';

const win = (page, name) => page.getByRole('region', { name, exact: true });
const icon = (page, name) => page.getByRole('button', { name, exact: true }).and(page.locator('.dicon'));
const files = page => win(page, 'Documents').locator('.ex-file');
// Seeded posts that are not public (draft and private).
const HIDDEN = ['epbs-from-a-clients-perspective', 'molten-core-as-a-scheduling-problem'];

/** Drags `from` onto the center of `to` with the mouse, in small steps. */
async function dragOnto(page, from, to) {
  const a = await from.boundingBox(), b = await to.boundingBox();
  await page.mouse.move(a.x + a.width / 2, a.y + a.height / 2);
  await page.mouse.down();
  await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2, { steps: 10 });
  await page.mouse.up();
}

test('icons: a click selects, a double-click or Enter opens, and a drag moves', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  const docs = icon(page, 'Documents');
  await docs.click();
  await expect(docs).toHaveClass(/sel/);
  await expect(win(page, 'Documents')).toBeHidden();
  await docs.press('Enter');
  await expect(win(page, 'Documents')).toBeVisible();
  await win(page, 'Documents').getByRole('button', { name: 'Close' }).click();

  const before = await docs.boundingBox();
  await page.mouse.move(before.x + 40, before.y + 20);
  await page.mouse.down();
  await page.mouse.move(before.x + 40 + 6, before.y + 20 + 300, { steps: 8 });
  await page.mouse.up();
  const after = await docs.boundingBox();
  expect(Math.round(after.y - before.y)).toBe(300);
  expect(Math.round(after.x - before.x)).toBe(6);
  // The drag does not open it.
  await expect(win(page, 'Documents')).toBeHidden();
  expect(errors, errors.join('\n')).toEqual([]);
});

test('Documents lists the public posts only, and a double-click opens one', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  const pub = await (await page.request.get('/api/posts')).json();
  await icon(page, 'Documents').dblclick();
  await expect(files(page)).toHaveCount(pub.length);
  await expect(files(page).locator('span')).toHaveText(pub.map(p => `${p.slug}.md`));
  for (const slug of HIDDEN) await expect(win(page, 'Documents').getByText(`${slug}.md`)).toHaveCount(0);
  await expect(win(page, 'Documents').locator('[data-status]')).toHaveText(`${pub.length} object(s)`);

  // Details: size, type, and date.
  await win(page, 'Documents').getByRole('button', { name: 'View', exact: true }).click();
  await page.getByRole('menuitemcheckbox', { name: 'Details' }).click();
  await expect(win(page, 'Documents').locator('.ex-hdr span')).toHaveText(['Name', 'Size', 'Type', 'Modified']);
  await expect(files(page).first()).toContainText('Markdown File');
  await expect(files(page).first()).toContainText(/\d+ KB/);

  // A click selects; a double-click opens the post in the Logbook window.
  const first = files(page).first();
  await first.click();
  await expect(win(page, 'Documents').locator('[data-status]')).toContainText('1 object(s) selected');
  await first.dblclick();
  await expect(page).toHaveURL(`/posts/${pub[0].slug}`);
  await expect(page.locator('post-view h1.t')).toHaveText(pub[0].title);
  await expect(page.locator('[data-win="logbook"]')).not.toHaveClass(/inactive/);
  expect(errors, errors.join('\n')).toEqual([]);
});

test('the Recycle Bin takes files and icons, restores them, and deletes nothing', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  const [post] = await (await page.request.get('/api/posts')).json();
  const bin = icon(page, 'Recycle Bin');
  await expect(bin.locator('[data-bin="empty"]')).toBeVisible();

  // Drag a file from Documents onto the bin icon.
  await icon(page, 'Documents').dblclick();
  await dragOnto(page, win(page, 'Documents').getByRole('button', { name: `${post.slug}.md` }), bin);
  await expect(bin.locator('[data-bin="full"]')).toBeVisible();
  // The file is still in Documents, and the post is still public.
  await expect(win(page, 'Documents').getByRole('button', { name: `${post.slug}.md` })).toBeVisible();
  expect((await page.request.get(`/api/posts/${post.slug}`)).status()).toBe(200);

  // Drag a desktop icon onto the bin. The icon stays where it was.
  await win(page, 'Documents').getByRole('button', { name: 'Close' }).click();
  const lb = await icon(page, 'Logbook.exe').boundingBox();
  await dragOnto(page, icon(page, 'Logbook.exe'), bin);
  expect(await icon(page, 'Logbook.exe').boundingBox()).toEqual(lb);

  // The Delete key throws a selected icon away.
  await icon(page, 'Documents').click();
  await page.keyboard.press('Delete');

  await bin.dblclick();
  const view = win(page, 'Recycle Bin').locator('[data-view-files]');
  await expect(view.locator('.ex-file span')).toHaveText([`${post.slug}.md`, 'Logbook.exe', 'Documents']);
  await expect(win(page, 'Recycle Bin').locator('[data-status]')).toHaveText('3 object(s)');

  // Restore one.
  await view.getByRole('button', { name: 'Logbook.exe' }).click();
  await win(page, 'Recycle Bin').getByRole('button', { name: 'File', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Restore' }).click();
  await expect(view.locator('.ex-file span')).toHaveText([`${post.slug}.md`, 'Documents']);

  // Empty the bin: a confirm box first. No keeps the items.
  const empty = async () => {
    await win(page, 'Recycle Bin').getByRole('button', { name: 'File', exact: true }).click();
    await page.getByRole('menuitem', { name: 'Empty Recycle Bin' }).click();
    return page.getByRole('dialog', { name: 'Confirm Multiple File Delete' });
  };
  let confirm = await empty();
  await expect(confirm).toContainText('Are you sure you want to delete these 2 items?');
  await confirm.getByRole('button', { name: 'No' }).click();
  await expect(view.locator('.ex-file')).toHaveCount(2);
  confirm = await empty();
  await confirm.getByRole('button', { name: 'Yes' }).click();
  await expect(view.locator('.ex-file')).toHaveCount(0);
  await expect(view).toContainText('The Recycle Bin is empty.');
  await expect(bin.locator('[data-bin="empty"]')).toBeVisible();

  // Nothing changed on the server.
  expect((await page.request.get(`/api/posts/${post.slug}`)).status()).toBe(200);
  await expect(icon(page, 'Logbook.exe')).toBeVisible();
  await expect(icon(page, 'Documents')).toBeVisible();
  expect(errors, errors.join('\n')).toEqual([]);
});

test('a tap on a selected icon opens it (touch)', async ({ browser }) => {
  const context = await browser.newContext({ hasTouch: true, viewport: { width: 1024, height: 768 } });
  const page = await context.newPage();
  await page.goto('/');
  const docs = icon(page, 'Documents');
  await docs.tap();
  await expect(docs).toHaveClass(/sel/);
  await expect(win(page, 'Documents')).toBeHidden();
  await docs.tap();
  await expect(win(page, 'Documents')).toBeVisible();
  await context.close();
});
