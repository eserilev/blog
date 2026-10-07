// Video embeds (spec 4.10): each mode shows its design, the controls drive the
// video, MS-DOS mode draws ASCII art, and a YouTube embed makes no third-party
// request before the click.
import { test, expect, devices } from '@playwright/test';
import { signIn, videoPost, watchErrors, YOUTUBE_ID } from './helpers.js';

const { defaultBrowserType: _, ...iPhone } = devices['iPhone 13'];
const SITE = 'http://localhost:18100';
let slug;

test.beforeAll(async ({ browser }) => {
  const page = await browser.newPage();
  await signIn(page);
  slug = await videoPost(page, 'Video embeds');
  await page.close();
});

/** Records and blocks every request that leaves the site. */
async function blockThirdParty(page) {
  const seen = [];
  await page.route(url => url.protocol.startsWith('http') && url.origin !== SITE, route => {
    seen.push(route.request().url());
    return route.abort();
  });
  return seen;
}

const fileEmbed = (page, scope) => page.locator(`${scope} video-embed[data-kind="file"]`);
const ytEmbed = (page, scope) => page.locator(`${scope} video-embed[data-kind="youtube"]`);

test('Win98: a Media Player window; play, pause, and stop change the status', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto(`/posts/${slug}`);
  const player = fileEmbed(page, 'post-view');
  await expect(player.locator('.titlebar')).toHaveText(/Media Player - The test clip/);
  await expect(player.locator('.ve-menu')).toHaveAttribute('aria-hidden', 'true');
  await expect(player.locator('.ve-ctl')).toHaveAttribute('aria-hidden', 'true');
  await expect(player.locator('.ve-hint')).toHaveText('Click ▶ to play · 0:04');
  await expect(player.locator('.ve-state')).toHaveText('Stopped');
  await expect(player.locator('.ve-clock')).toHaveText('00:00 / 00:04');

  await player.getByRole('button', { name: 'Play video: The test clip' }).click();
  await expect(player.locator('.ve-state')).toHaveText('Playing');
  await expect(player.locator('.ve-start')).toBeHidden();
  await expect(player.locator('.ve-clock')).toHaveText(/^00:0[1-4] \/ 00:04$/);
  await expect.poll(() => player.locator('.ve-seek').evaluate(s => Number(s.value))).toBeGreaterThan(0);

  await player.getByRole('button', { name: 'Pause', exact: true }).click();
  await expect(player.locator('.ve-state')).toHaveText('Paused');
  const paused = await player.locator('video').evaluate(v => v.currentTime);
  await page.waitForTimeout(400);
  expect(await player.locator('video').evaluate(v => v.currentTime)).toBe(paused);

  // Prev and next go to the start and the end of the one video.
  await player.getByRole('button', { name: 'Go to the start' }).click();
  await expect.poll(() => player.locator('video').evaluate(v => v.currentTime)).toBe(0);
  // Forward 10 s in a 4 s video: the end, where the video stops.
  await player.getByRole('button', { name: 'Forward 10 seconds' }).click();
  await expect.poll(async () => (await player.locator('.ve-state').textContent()) === 'Stopped' ||
    (await player.locator('video').evaluate(v => v.currentTime)) === 4).toBe(true);

  await player.getByRole('button', { name: 'Play', exact: true }).click();
  await expect(player.locator('.ve-state')).toHaveText(/Playing|Stopped/);
  await player.getByRole('button', { name: 'Stop', exact: true }).click();
  await expect(player.locator('.ve-state')).toHaveText('Stopped');
  await expect(player.locator('.ve-start')).toBeVisible();
  await expect(player.locator('.ve-clock')).toHaveText('00:00 / 00:04');
  expect(errors, errors.join('\n')).toEqual([]);
});

test('MS-DOS: PLAY.EXE draws ASCII art from the frames; keys work in the player', async ({ page }) => {
  const errors = watchErrors(page);
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('logbook.mode', 'night'));
  await page.goto(`/posts/${slug}`);
  const player = fileEmbed(page, 'dos-shell');
  await expect(player.locator('.ve-label')).toHaveText('PLAY.EXE ─ THETESTC.WEBM');
  await expect(player.locator('.ve-frame')).toContainText('► PRESS ENTER');
  await expect(player.locator('.ve-info')).toHaveText(/^0:04 · 96x54 · \d+ KB$/);
  await expect(player.locator('.ve-statusline')).toContainText('■ STOP');

  await player.getByRole('button', { name: 'Play video: The test clip' }).click();
  await expect(player.locator('.ve-statusline')).toContainText('► PLAY');
  // ASCII mode is on by default: the frames become characters of the ramp.
  const ascii = player.locator('.ve-ascii');
  await expect(ascii).toBeVisible();
  await expect.poll(() => ascii.textContent().then(t => t.replace(/\s/g, '').length)).toBeGreaterThan(100);
  expect(await ascii.textContent()).toMatch(/^[ .:\-=+*#%@\n]+$/);
  await expect(player.locator('.ve-statusline')).toContainText(/\[█+░*\]/);

  // Keys work while the focus is in the player.
  await player.locator('.ve-screen').focus();
  await page.keyboard.press('Enter');
  await expect(player.locator('.ve-statusline')).toContainText('‖ PAUSE');
  await page.keyboard.press('a');
  await expect(player.getByRole('button', { name: 'A ASCII' })).toHaveAttribute('aria-pressed', 'false');
  await expect(ascii).toBeHidden();
  const video = player.locator('video');
  await expect(video).toBeVisible();
  expect(await video.evaluate(v => getComputedStyle(v).filter)).toContain('grayscale(1)');
  await page.keyboard.press('a');
  await expect(ascii).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(player.locator('.ve-statusline')).toContainText('■ STOP');
  await expect(player.locator('.ve-start')).toBeVisible();
  await player.getByRole('button', { name: 'ENTER Play' }).click();
  await expect(player.locator('.ve-statusline')).toContainText('► PLAY');

  // The keys stay in the player: the prompt still takes a command.
  const input = page.getByLabel('C:\\LOGBOOK>');
  await input.fill('ver');
  await input.press('Enter');
  await expect(page.locator('dos-shell [data-out]')).toContainText('Logbook DOS Version 6.22');

  // YouTube: the double box, a dim note, and the grayscale filter.
  const yt = ytEmbed(page, 'dos-shell');
  await expect(yt.locator('.ve-label')).toHaveText('PLAY.EXE ─ YOUTUBE');
  await expect(yt.locator('.ve-note')).toHaveText('ASCII needs a local file');
  expect(errors, errors.join('\n')).toEqual([]);
});

test('MS-DOS: the ASCII loop stops when the video pauses', async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('logbook.mode', 'night'));
  await page.goto(`/posts/${slug}`);
  const player = fileEmbed(page, 'dos-shell');
  await player.getByRole('button', { name: 'Play video: The test clip' }).click();
  await expect.poll(() => player.locator('.ve-ascii').textContent().then(t => t.trim().length)).toBeGreaterThan(0);
  await player.getByRole('button', { name: 'ENTER Pause' }).click();
  await expect(player.locator('.ve-statusline')).toContainText('‖ PAUSE');
  await page.waitForTimeout(200);
  const before = await player.locator('.ve-ascii').textContent();
  expect(await player.evaluate(el => el.raf)).toBe(0);
  await page.waitForTimeout(500);
  expect(await player.locator('.ve-ascii').textContent()).toBe(before);
});

test('phone: the LCD player; LCD tint toggles; play and pause', async ({ browser }) => {
  const context = await browser.newContext({ ...iPhone, baseURL: SITE });
  const page = await context.newPage();
  const errors = watchErrors(page);
  await page.goto(`/posts/${slug}`);
  const player = fileEmbed(page, 'phone-shell');
  await expect(player.locator('.ve-head')).toHaveText(/■ VIDEO\s*0:04/);
  const lcd = player.getByRole('button', { name: 'LCD tint' });
  await expect(lcd).toHaveAttribute('aria-pressed', 'true');
  await expect(player.locator('.ve-screen')).toHaveClass(/tint/);
  expect(await player.locator('video').evaluate(v => getComputedStyle(v).mixBlendMode)).toBe('multiply');
  await lcd.tap();
  await expect(lcd).toHaveAttribute('aria-pressed', 'false');
  expect(await player.locator('video').evaluate(v => getComputedStyle(v).filter)).toBe('none');
  await lcd.tap();
  await expect(lcd).toHaveAttribute('aria-pressed', 'true');

  await player.getByRole('button', { name: 'Play video: The test clip' }).tap();
  await expect(player.locator('.ve-line')).toContainText('PLAY');
  await expect(player.getByRole('button', { name: 'Pause' })).toBeVisible();
  await expect.poll(() => player.locator('.ve-blocks i.on').count()).toBeGreaterThan(0);
  await player.getByRole('button', { name: 'Pause' }).tap();
  await expect(player.locator('.ve-line')).toContainText('PAUSE');
  await player.getByRole('button', { name: 'Back 10 seconds' }).tap();
  await expect.poll(() => player.locator('video').evaluate(v => v.currentTime)).toBe(0);
  // Every key is a 44 px touch target.
  for (const key of await player.locator('.ve-key').all()) {
    const box = await key.boundingBox();
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.width).toBeGreaterThanOrEqual(44);
  }
  expect(errors, errors.join('\n')).toEqual([]);
  await context.close();
});

test('YouTube: no third-party request before the click; then a sandboxed frame', async ({ page }) => {
  const seen = await blockThirdParty(page);
  await page.goto(`/posts/${slug}`);
  const yt = ytEmbed(page, 'post-view');
  await expect(yt.locator('.ve-hint')).toHaveText('Click ▶ to play · YouTube');
  await page.waitForLoadState('networkidle');
  await page.waitForTimeout(500);
  expect(seen, 'requests before the click').toEqual([]);
  await expect(page.locator('iframe')).toHaveCount(0);

  await yt.getByRole('button', { name: /Play YouTube video: A YouTube video/ }).click();
  const frame = yt.locator('iframe');
  await expect(frame).toHaveAttribute('src', `https://www.youtube-nocookie.com/embed/${YOUTUBE_ID}?autoplay=1`);
  await expect(frame).toHaveAttribute('sandbox', 'allow-scripts allow-same-origin allow-presentation allow-popups');
  await expect(frame).toHaveAttribute('referrerpolicy', 'strict-origin-when-cross-origin');
  await expect(frame).toHaveAttribute('allow', /autoplay/);
  await expect.poll(() => seen.length).toBeGreaterThan(0);
  expect(seen.every(u => u.startsWith('https://www.youtube-nocookie.com/embed/'))).toBe(true);
});

test('YouTube: phone and MS-DOS mode also wait for the click', async ({ browser }) => {
  const context = await browser.newContext({ ...iPhone, baseURL: SITE });
  const page = await context.newPage();
  const seen = await blockThirdParty(page);
  await page.goto(`/posts/${slug}`);
  await expect(ytEmbed(page, 'phone-shell').locator('.ve-head')).toHaveText(/VIDEO\s*YOUTUBE/);
  await page.evaluate(() => Mode.set('night'));
  await expect(ytEmbed(page, 'dos-shell').locator('.ve-note')).toBeVisible();
  await page.waitForLoadState('networkidle');
  expect(seen).toEqual([]);
  await context.close();
});

test('a mode change stops the video that it hides', async ({ page }) => {
  await page.goto(`/posts/${slug}`);
  const player = fileEmbed(page, 'post-view');
  await player.getByRole('button', { name: 'Play video: The test clip' }).click();
  await expect(player.locator('.ve-state')).toHaveText('Playing');
  await page.evaluate(() => Mode.set('night'));
  await expect.poll(() => player.locator('video').evaluate(v => v.paused)).toBe(true);
  // The MS-DOS view shows its own copy, in its own design.
  await expect(fileEmbed(page, 'dos-shell').locator('.ve-label')).toBeVisible();
  await page.evaluate(() => Mode.set('general'));
});

test('the editor preview shows the embed', async ({ page }) => {
  const errors = watchErrors(page);
  await signIn(page);
  await page.getByRole('button', { name: 'New post' }).click();
  await expect(page.locator('md-editor')).toHaveAttribute('data-loaded', '');
  await page.locator('[data-video-file]').setInputFiles('fixtures/clip.webm');
  await expect(page.locator('md-editor [data-status]')).toHaveText('Video added.');
  await expect(page.getByLabel('Post markdown')).toHaveValue(/```video\n\/media\/[0-9a-f]{64}\.webm\n```/);
  const player = page.locator('[data-preview] video-embed');
  await expect(player.locator('.titlebar')).toHaveText(/Media Player/);
  await expect(player.locator('.ve-hint')).toHaveText('Click ▶ to play · 0:04');
  expect(errors, errors.join('\n')).toEqual([]);
});
