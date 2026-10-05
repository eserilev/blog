// The Start menu, the Shut Down dialog, and the end screen (spec 11.3).
import { test, expect, devices } from '@playwright/test';
import { watchErrors } from './helpers.js';

// The project runs Chromium. Keep the iPhone screen, touch, and coarse pointer.
const { defaultBrowserType: _, ...iPhone } = devices['iPhone 13'];

const start = page => page.getByRole('button', { name: 'Start' });
const menu = page => page.getByRole('menu', { name: 'Start' });
const item = (page, name) => menu(page).getByRole('menuitem', { name, exact: true });

/** Opens the Start menu, then the Shut Down dialog. */
async function shutDownDialog(page) {
  await start(page).click();
  await item(page, 'Shut Down…').click();
  const dialog = page.getByRole('dialog', { name: 'Shut Down Logbook' });
  await expect(dialog).toBeVisible();
  return dialog;
}

test('Start opens the menu; Escape, Start, and a click outside close it', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await expect(start(page)).toHaveAttribute('aria-expanded', 'false');
  await expect(menu(page)).toBeHidden();

  await start(page).click();
  await expect(menu(page)).toBeVisible();
  await expect(start(page)).toHaveAttribute('aria-expanded', 'true');
  for (const name of ['Home', 'Latest post', 'Topics', 'RSS feed', 'Shut Down…']) await expect(item(page, name)).toBeVisible();
  await expect(menu(page).getByRole('separator')).toHaveCount(1);
  await expect(menu(page).getByText('About')).toHaveCount(0);

  await page.keyboard.press('Escape');
  await expect(menu(page)).toBeHidden();
  await expect(start(page)).toBeFocused();
  await expect(start(page)).toHaveAttribute('aria-expanded', 'false');

  await start(page).click();
  await expect(menu(page)).toBeVisible();
  await start(page).click();
  await expect(menu(page)).toBeHidden();

  await start(page).click();
  await page.locator('.masthead h1').click();
  await expect(menu(page)).toBeHidden();
  await expect(page).toHaveURL('/');
  expect(errors, errors.join('\n')).toEqual([]);
});

test('the keyboard moves through the menu and opens Topics', async ({ page }) => {
  await page.goto('/posts/zero-copy-ssz-decoding-in-rust');
  await start(page).focus();
  await page.keyboard.press('Enter');
  await expect(item(page, 'Home')).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(item(page, 'Latest post')).toBeFocused();
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('ArrowUp');
  await expect(item(page, 'Shut Down…')).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(item(page, 'Home')).toBeFocused();
  await page.keyboard.press('End');
  await expect(item(page, 'Shut Down…')).toBeFocused();
  await page.keyboard.press('Home');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowDown');
  await expect(item(page, 'Topics')).toBeFocused();

  // Right opens the submenu; Left closes it and goes back to Topics.
  await page.keyboard.press('ArrowRight');
  const sub = page.getByRole('menu', { name: 'Topics' });
  await expect(sub).toBeVisible();
  await expect(item(page, 'Topics')).toHaveAttribute('aria-expanded', 'true');
  await expect(sub.getByRole('menuitem').first()).toBeFocused();
  await page.keyboard.press('ArrowLeft');
  await expect(sub).toBeHidden();
  await expect(item(page, 'Topics')).toBeFocused();

  // Left in the main menu closes it.
  await page.keyboard.press('ArrowLeft');
  await expect(menu(page)).toBeHidden();
  await expect(start(page)).toBeFocused();

  // ArrowUp on Start opens the menu at the last item. Enter on Home goes home.
  await page.keyboard.press('ArrowUp');
  await expect(item(page, 'Shut Down…')).toBeFocused();
  await page.keyboard.press('Home');
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL('/');
  await expect(menu(page)).toBeHidden();
  await expect(start(page)).toBeFocused();
});

test('Topics lists the topics and opens a topic page', async ({ page }) => {
  await page.goto('/');
  const names = await page.locator('#topic-links a').allTextContents();
  await start(page).click();
  await item(page, 'Topics').click();
  const sub = page.getByRole('menu', { name: 'Topics' });
  await expect(sub.getByRole('menuitem')).toHaveText(names);
  await sub.getByRole('menuitem', { name: 'Rust', exact: true }).click();
  await expect(page).toHaveURL('/topics/rust');
  await expect(page.locator('#list-title')).toHaveText('Writing: Rust');
  await expect(menu(page)).toBeHidden();

  // The keyboard: Enter on Topics opens it, Enter on a topic opens the page.
  await start(page).focus();
  await page.keyboard.press('Enter');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  await expect(sub.getByRole('menuitem').first()).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  const second = await page.locator('#topic-links a').nth(1).getAttribute('href');
  await expect(page).toHaveURL(second);
});

test('Latest post opens the newest post', async ({ page }) => {
  await page.goto('/');
  const [newest] = await (await page.request.get('/api/posts')).json();
  await start(page).click();
  await item(page, 'Latest post').click();
  await expect(page).toHaveURL(`/posts/${newest.slug}`);
  await expect(page.locator('post-view h1.t')).toHaveText(newest.title);
  await expect(menu(page)).toBeHidden();
});

test('Shut Down: Stay and Cancel close the dialog; the phone choice is not on a desktop', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  let dialog = await shutDownDialog(page);
  await expect(dialog.getByText('What do you want the computer to do?')).toBeVisible();
  await expect(dialog.getByRole('radio')).toHaveCount(3);
  await expect(dialog.getByLabel('Restart in phone mode')).toHaveCount(0);
  await expect(dialog.getByLabel('Stay in Windows')).toBeChecked();
  await expect(dialog.getByLabel('Stay in Windows')).toBeFocused();
  await dialog.getByRole('button', { name: 'OK' }).click();
  await expect(dialog).toBeHidden();
  await expect(start(page)).toBeFocused();
  await expect(page.locator('html')).toHaveAttribute('data-mode', 'general');

  dialog = await shutDownDialog(page);
  await dialog.getByLabel('Shut down', { exact: true }).check();
  await dialog.getByRole('button', { name: 'Cancel' }).click();
  await expect(dialog).toBeHidden();

  // Escape closes it, and the focus goes back to Start. The choice resets.
  dialog = await shutDownDialog(page);
  await expect(dialog.getByLabel('Stay in Windows')).toBeChecked();
  await page.keyboard.press('Escape');
  await expect(dialog).toBeHidden();
  await expect(start(page)).toBeFocused();
  expect(errors, errors.join('\n')).toEqual([]);
});

test('Shut Down: Restart in MS-DOS mode starts dark mode, and it persists', async ({ page }) => {
  const errors = watchErrors(page);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  const dialog = await shutDownDialog(page);
  await dialog.getByLabel('Restart in MS-DOS mode').check();
  await dialog.getByRole('button', { name: 'OK' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-mode', 'night');
  await expect(page.locator('dos-shell')).toBeVisible();
  await expect(page.locator('.taskbar')).toBeHidden();
  expect(await page.evaluate(() => localStorage.getItem('logbook.mode'))).toBe('night');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-mode', 'night');
  await expect(page.locator('dos-shell')).toBeVisible();
  expect(errors, errors.join('\n')).toEqual([]);
});

test('Shut Down: the restart shows a short black screen', async ({ page }) => {
  await page.goto('/');
  const dialog = await shutDownDialog(page);
  await dialog.getByLabel('Restart in MS-DOS mode').check();
  await page.keyboard.press('Enter');
  await expect(page.locator('[data-blackout]')).toBeVisible();
  await expect(page.locator('dos-shell')).toBeVisible();
  await expect(page.locator('[data-blackout]')).toBeHidden();
});

test('Shut Down: Shut down shows the end screen; a key goes back', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await page.evaluate(() => window.scrollTo(0, 300));
  const before = await page.evaluate(() => window.scrollY);
  const dialog = await shutDownDialog(page);
  await dialog.getByLabel('Shut down', { exact: true }).check();
  await dialog.getByRole('button', { name: 'OK' }).click();
  const screen = page.getByRole('dialog', { name: 'Shut down screen' });
  await expect(screen).toBeVisible();
  const text = screen.getByText('It’s now safe to turn off your computer.');
  await expect(text).toBeVisible();
  expect(await text.evaluate(el => getComputedStyle(el).color)).toBe('rgb(255, 140, 0)');
  await expect(screen.getByText('Press any key to return.')).toBeAttached();
  await expect(screen).toBeFocused();

  await page.keyboard.press('x');
  await expect(screen).toBeHidden();
  await expect(start(page)).toBeFocused();
  expect(await page.evaluate(() => window.scrollY)).toBe(before);
  // The end screen does not change the saved mode.
  expect(await page.evaluate(() => localStorage.getItem('logbook.mode'))).toBeNull();
  await expect(page.locator('html')).toHaveAttribute('data-mode', 'general');

  // A click goes back too.
  await (await shutDownDialog(page)).getByLabel('Shut down', { exact: true }).check();
  await page.getByRole('button', { name: 'OK' }).click();
  await expect(text).toBeVisible();
  await screen.click();
  await expect(screen).toBeHidden();
  expect(errors, errors.join('\n')).toEqual([]);
});

test.describe('phone, Win98 view', () => {
  test.use(iPhone);

  test('the phone choice shows and starts phone mode; the menu fits the screen', async ({ page }) => {
    const errors = watchErrors(page);
    await page.goto('/');
    await page.evaluate(() => localStorage.setItem('logbook.mode', 'general'));
    await page.reload();
    await expect(page.locator('.taskbar')).toBeVisible();
    await start(page).tap();
    await expect(menu(page)).toBeVisible();
    // A tap opens Topics inside the menu, and the menu stays on the screen.
    await item(page, 'Topics').tap();
    await expect(page.getByRole('menu', { name: 'Topics' })).toBeVisible();
    const box = await menu(page).boundingBox();
    const bar = await page.locator('.taskbar').boundingBox();
    const width = page.viewportSize().width;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(width);
    expect(box.y).toBeGreaterThanOrEqual(0);
    expect(box.y + box.height).toBeLessThanOrEqual(bar.y);
    await item(page, 'Topics').tap();
    await expect(page.getByRole('menu', { name: 'Topics' })).toBeHidden();

    await item(page, 'Shut Down…').tap();
    const dialog = page.getByRole('dialog', { name: 'Shut Down Logbook' });
    await expect(dialog.getByRole('radio')).toHaveCount(4);
    await dialog.getByLabel('Restart in phone mode').check();
    await dialog.getByRole('button', { name: 'OK' }).tap();
    await expect(page.locator('html')).toHaveAttribute('data-mode', 'phone');
    await expect(page.locator('phone-shell')).toBeVisible();
    expect(await page.evaluate(() => localStorage.getItem('logbook.mode'))).toBe('phone');
    expect(errors, errors.join('\n')).toEqual([]);
  });
});
