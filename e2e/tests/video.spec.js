// Video embeds (spec 4.10): each mode shows its design and drives the YouTube
// player through postMessage. A stub page stands in for the player (helpers.js),
// so CI never loads the real YouTube. Before the click, no request leaves the site.
import { test, expect, devices } from '@playwright/test';
import { signIn, stubYouTube, videoPost, watchErrors, YOUTUBE_ID } from './helpers.js';

const { defaultBrowserType: _, ...iPhone } = devices['iPhone 13'];
const SITE = 'http://localhost:18100';
let slug;

test.beforeAll(async ({ browser }) => {
  const page = await browser.newPage();
  await signIn(page);
  slug = await videoPost(page, 'Video embeds');
  await page.close();
});

const embed = (page, scope) => page.locator(`${scope} video-embed`).first();
const players = page => page.frames().filter(f => f.url().startsWith('https://www.youtube-nocookie.com/'));
/** The commands that the stub player received (the first frame, or frame n). */
const commands = (page, n = 0) => players(page)[n]?.evaluate(() => window.commands) ?? [];

test('the facade: no third-party request before the click; one click loads and plays', async ({ page }) => {
  const seen = await stubYouTube(page);
  await page.goto(`/posts/${slug}`);
  const player = embed(page, 'post-view');
  await expect(player.locator('.ve-hint')).toHaveText('Click ▶ to play');
  await expect(player.locator('.ve-clock')).toHaveText('00:00 / --:--');
  await page.waitForLoadState('networkidle');
  await page.waitForTimeout(500);
  expect(seen, 'requests before the click').toEqual([]);
  await expect(page.locator('iframe')).toHaveCount(0);

  await player.getByRole('button', { name: /Play YouTube video: The test clip/ }).click();
  const frame = player.locator('iframe');
  const src = new URL(await frame.getAttribute('src'));
  expect(src.origin + src.pathname).toBe(`https://www.youtube-nocookie.com/embed/${YOUTUBE_ID}`);
  expect(Object.fromEntries(src.searchParams)).toEqual({
    enablejsapi: '1', controls: '0', playsinline: '1', autoplay: '1', rel: '0', origin: SITE,
  });
  await expect(frame).toHaveAttribute('sandbox', 'allow-scripts allow-same-origin allow-presentation allow-popups');
  // The frame request takes the referrer policy of the page header.
  await expect(frame).not.toHaveAttribute('referrerpolicy', /./);
  await expect(player.locator('.ve-state')).toHaveText('Playing');
  await expect(player.locator('.ve-clock')).toHaveText(/^00:0\d \/ 03:00$/);
  expect(await commands(page)).toContain('playVideo');
  expect(seen.every(u => u.startsWith('https://www.youtube-nocookie.com/embed/'))).toBe(true);
});

test('Win98: the buttons, the seek bar, and the status drive the player', async ({ page }) => {
  const errors = watchErrors(page);
  await stubYouTube(page);
  await page.goto(`/posts/${slug}`);
  const player = embed(page, 'post-view');
  await expect(player.locator('.titlebar')).toHaveText(/Media Player - The test clip/);
  await expect(player.locator('.ve-menu')).toHaveAttribute('aria-hidden', 'true');
  await expect(player.locator('.ve-ctl')).toHaveAttribute('aria-hidden', 'true');
  await expect(player.locator('.ve-state')).toHaveText('Stopped');

  // The Play button also loads the player on the first click.
  await player.getByRole('button', { name: 'Play', exact: true }).click();
  await expect(player.locator('.ve-state')).toHaveText('Playing');
  await expect(player.locator('.ve-caption')).toHaveText('The test clip · 3:00');
  await player.getByRole('button', { name: 'Pause', exact: true }).click();
  await expect(player.locator('.ve-state')).toHaveText('Paused');
  await player.getByRole('button', { name: 'Forward 10 seconds' }).click();
  await expect(player.locator('.ve-clock')).toHaveText(/^00:1\d \/ 03:00$/);
  await player.getByRole('button', { name: 'Go to the end' }).click();
  await expect(player.locator('.ve-state')).toHaveText(/Stopped|Paused/);
  await player.getByRole('button', { name: 'Go to the start' }).click();
  await expect(player.locator('.ve-clock')).toHaveText('00:00 / 03:00');
  // The seek bar: half way is 1:30.
  await player.locator('.ve-seek').fill('500');
  await expect(player.locator('.ve-clock')).toHaveText('01:30 / 03:00');
  await player.locator('.ve-volume').fill('0');
  await expect(player.locator('.ve-sound')).toHaveText('Muted');
  await player.getByRole('button', { name: 'Stop', exact: true }).click();
  await expect(player.locator('.ve-state')).toHaveText('Stopped');
  await expect(player.locator('.ve-start')).toBeVisible();
  const sent = await commands(page);
  for (const f of ['playVideo', 'pauseVideo', 'seekTo', 'setVolume', 'mute']) expect(sent).toContain(f);
  expect(errors, errors.join('\n')).toEqual([]);
});

test('a message from another origin changes nothing', async ({ page }) => {
  await stubYouTube(page);
  await page.goto(`/posts/${slug}`);
  const player = embed(page, 'post-view');
  await player.getByRole('button', { name: /Play YouTube video/ }).click();
  await expect(player.locator('.ve-clock')).toHaveText(/ \/ 03:00$/);
  await page.evaluate(() => window.postMessage(JSON.stringify({ event: 'infoDelivery', info: { duration: 999, playerState: 2 } }), '*'));
  await page.waitForTimeout(300);
  await expect(player.locator('.ve-clock')).toHaveText(/ \/ 03:00$/);
  await expect(player.locator('.ve-state')).toHaveText('Playing');
});

test('MS-DOS: PLAY.EXE, text mode, the status line, and the keys', async ({ page }) => {
  const errors = watchErrors(page);
  await stubYouTube(page);
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('logbook.mode', 'night'));
  await page.goto(`/posts/${slug}`);
  const player = embed(page, 'dos-shell');
  await expect(player.locator('.ve-label')).toHaveText('PLAY.EXE ─ THETESTC');
  await expect(player.locator('.ve-frame')).toContainText('► PRESS ENTER');
  await expect(player.locator('.ve-info')).toHaveText('YOUTUBE · --:--');
  await expect(player.locator('.ve-statusline')).toContainText('■ STOP   00:00 / --:--');

  await player.getByRole('button', { name: /Play YouTube video/ }).click();
  await expect(player.locator('.ve-statusline')).toContainText('► PLAY');
  await expect(player.locator('.ve-statusline')).toContainText('/ 03:00');
  // The status line is drawn text; a live region gives screen readers the state.
  await expect(player.locator('.ve-statusline')).toHaveAttribute('aria-hidden', 'true');
  await expect(player.locator('.dos-sr')).toHaveText('Playing');
  // Text mode is on by default: a high-contrast filter and the cell grid over the frame.
  const fx = player.locator('.ve-fx');
  await expect(fx).toBeVisible();
  expect(await fx.evaluate(e => getComputedStyle(e).pointerEvents)).toBe('none');
  expect(await player.locator('iframe').evaluate(f => getComputedStyle(f).filter)).toContain('contrast(1.8)');

  await player.locator('.ve-screen').focus();
  await page.keyboard.press('Enter');
  await expect(player.locator('.ve-statusline')).toContainText('‖ PAUSE');
  await page.keyboard.press('a');
  await expect(player.getByRole('button', { name: 'A Text mode' })).toHaveAttribute('aria-pressed', 'false');
  await expect(fx).toBeHidden();
  await page.keyboard.press('a');
  await expect(fx).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(player.locator('.ve-statusline')).toContainText('■ STOP');
  await expect(player.locator('.ve-start')).toBeVisible();
  await player.getByRole('button', { name: 'ENTER Play' }).click();
  await expect(player.locator('.ve-statusline')).toContainText('► PLAY');
  await expect(player.locator('.ve-statusline')).toContainText(/\[█*░+\]/);

  // The keys stay in the player: the prompt still takes a command.
  const input = page.getByLabel('C:\\LOGBOOK>');
  await input.fill('ver');
  await input.press('Enter');
  await expect(page.locator('dos-shell [data-out]')).toContainText('Logbook DOS Version 6.22');
  expect(errors, errors.join('\n')).toEqual([]);
});

test('phone: the LCD player; tint, play, pause, 10 s back, progress', async ({ browser }) => {
  const context = await browser.newContext({ ...iPhone, baseURL: SITE });
  const page = await context.newPage();
  const errors = watchErrors(page);
  const seen = await stubYouTube(page);
  await page.goto(`/posts/${slug}`);
  const player = embed(page, 'phone-shell');
  await expect(player.locator('.ve-head')).toHaveText('■ VIDEO--:--');
  await page.waitForLoadState('networkidle');
  expect(seen).toEqual([]);

  await player.getByRole('button', { name: /Play YouTube video/ }).tap();
  await expect(player.locator('.ve-line')).toContainText('PLAY');
  await expect(player.locator('.ve-head')).toHaveText('► VIDEO3:00');
  // The LCD tint: a grayscale frame, and the LCD color multiplied over it.
  const lcd = player.getByRole('button', { name: 'LCD tint' });
  await expect(lcd).toHaveAttribute('aria-pressed', 'true');
  const fx = player.locator('.ve-fx');
  expect(await fx.evaluate(e => [getComputedStyle(e).mixBlendMode, getComputedStyle(e).pointerEvents])).toEqual(['multiply', 'none']);
  expect(await player.locator('iframe').evaluate(f => getComputedStyle(f).filter)).toContain('grayscale(1)');
  await lcd.tap();
  await expect(lcd).toHaveAttribute('aria-pressed', 'false');
  await expect(fx).toBeHidden();
  expect(await player.locator('iframe').evaluate(f => getComputedStyle(f).filter)).toBe('none');
  await lcd.tap();

  await expect.poll(() => player.locator('.ve-line span').first().textContent()).toMatch(/^00:0[1-9] \/ 03:00$/);
  await player.getByRole('button', { name: 'Pause' }).tap();
  await expect(player.locator('.ve-line')).toContainText('PAUSE');
  await player.getByRole('button', { name: 'Back 10 seconds' }).tap();
  await expect(player.locator('.ve-line')).toContainText('00:00 / 03:00');
  await player.getByRole('button', { name: 'Play', exact: true }).tap();
  await expect(player.locator('.ve-line')).toContainText('PLAY');
  for (const key of await player.locator('.ve-key').all()) {
    const box = await key.boundingBox();
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.width).toBeGreaterThanOrEqual(44);
  }
  expect(errors, errors.join('\n')).toEqual([]);
  await context.close();
});

test('a mode change pauses the video that it hides', async ({ page }) => {
  await stubYouTube(page);
  await page.goto(`/posts/${slug}`);
  const player = embed(page, 'post-view');
  await player.getByRole('button', { name: /Play YouTube video/ }).click();
  await expect(player.locator('.ve-state')).toHaveText('Playing');
  await page.evaluate(() => Mode.set('night'));
  await expect(player.locator('.ve-state')).toHaveText('Paused');
  await expect(embed(page, 'dos-shell').locator('.ve-label')).toBeVisible();
  await page.evaluate(() => Mode.set('general'));
});

test('the editor preview shows the embed', async ({ page }) => {
  const errors = watchErrors(page);
  const seen = await stubYouTube(page);
  await signIn(page);
  await page.getByRole('button', { name: 'New post' }).click();
  await expect(page.locator('md-editor')).toHaveAttribute('data-loaded', '');
  await page.getByLabel('Post markdown').fill(`\`\`\`video\nhttps://youtu.be/${YOUTUBE_ID}\nPreview clip\n\`\`\`\n`);
  const player = page.locator('[data-preview] video-embed');
  await expect(player.locator('.titlebar')).toHaveText(/Media Player - Preview clip/);
  expect(seen).toEqual([]);
  expect(errors, errors.join('\n')).toEqual([]);
});

test('before the player answers, pause, stop, and a mode change go back to the start screen', async ({ page }) => {
  await stubYouTube(page, { delay: 1500 });
  await page.goto(`/posts/${slug}`);
  const player = embed(page, 'post-view');
  const play = player.getByRole('button', { name: /Play YouTube video/ });

  // Stop before the answer: no frame, and nothing plays later.
  await play.click();
  await expect(player.locator('iframe')).toHaveCount(1);
  await player.getByRole('button', { name: 'Stop', exact: true }).click();
  await expect(player.locator('iframe')).toHaveCount(0);
  await expect(player.locator('.ve-start')).toBeVisible();
  await page.waitForTimeout(2000);
  await expect(player.locator('.ve-state')).toHaveText('Stopped');

  // Pause before the answer: the same.
  await play.click();
  await player.getByRole('button', { name: 'Pause', exact: true }).click();
  await expect(player.locator('iframe')).toHaveCount(0);
  await expect(player.locator('.ve-state')).toHaveText('Stopped');

  // A mode change before the answer: the hidden embed has no frame.
  await play.click();
  await page.evaluate(() => Mode.set('night'));
  await expect(player.locator('iframe')).toHaveCount(0);
  await page.waitForTimeout(2000);
  expect(players(page)).toHaveLength(0);
  await page.evaluate(() => Mode.set('general'));
});

test('a player that never answers: the handshake stops after 10 s', async ({ page }) => {
  test.setTimeout(30_000);
  await stubYouTube(page, { silent: true });
  await page.goto(`/posts/${slug}`);
  const player = embed(page, 'post-view');
  await player.getByRole('button', { name: /Play YouTube video/ }).click();
  await expect(player.locator('iframe')).toHaveCount(1);
  const hellos = await players(page)[0].evaluate(() => new Promise(r => setTimeout(() => r(window.hellos), 2000)));
  expect(hellos).toBeGreaterThan(3);
  await expect(player.locator('iframe')).toHaveCount(0, { timeout: 12_000 });
  await expect(player.locator('.ve-start')).toBeVisible();
  await expect(player.locator('.ve-state')).toHaveText('Stopped');
});

test('the end of the video shows the start screen', async ({ page }) => {
  await stubYouTube(page, { duration: 2 });
  await page.goto(`/posts/${slug}`);
  const player = embed(page, 'post-view');
  await player.getByRole('button', { name: /Play YouTube video/ }).click();
  await expect(player.locator('.ve-state')).toHaveText('Playing');
  await expect(player.locator('.ve-state')).toHaveText('Stopped', { timeout: 5000 });
  await expect(player.locator('.ve-start')).toBeVisible();
  await expect(player.locator('.ve-clock')).toHaveText('00:00 / 00:02');
});

test('two embeds: one plays at a time; the focus moves to Pause', async ({ page }) => {
  await stubYouTube(page);
  await page.goto(`/posts/${slug}`);
  const [a, b] = [page.locator('post-view video-embed').nth(0), page.locator('post-view video-embed').nth(1)];
  await a.getByRole('button', { name: /Play YouTube video/ }).click();
  await expect(a.locator('.ve-state')).toHaveText('Playing');
  await expect(a.getByRole('button', { name: 'Pause', exact: true })).toBeFocused();
  await b.getByRole('button', { name: /Play YouTube video/ }).click();
  await expect(b.locator('.ve-state')).toHaveText('Playing');
  await expect(a.locator('.ve-state')).toHaveText('Paused');
  expect(await commands(page, 0)).toContain('pauseVideo');
});

test('a volume change before the answer goes to the player when it answers', async ({ page }) => {
  await stubYouTube(page, { delay: 1000 });
  await page.goto(`/posts/${slug}`);
  const player = embed(page, 'post-view');
  await player.getByRole('button', { name: /Play YouTube video/ }).click();
  await player.locator('.ve-volume').fill('40');
  await expect(player.locator('.ve-state')).toHaveText('Playing', { timeout: 5000 });
  await expect.poll(() => commands(page)).toContain('setVolume');
  expect(await players(page)[0].evaluate(() => window.volume())).toBe(40);
});
