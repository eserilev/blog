// Layout rules (spec 5.1): every view fits the screen at three widths. No page
// scrolls sideways, no element passes the right or left edge (scroll boxes may
// hold wide content), and the items of each bar share one center line.
// Each check saves a full-page screenshot in test-results.
import { test, expect, devices } from '@playwright/test';
import { signIn, stubYouTube, videoBlocks, videoPost, watchErrors } from './helpers.js';

const WIDTHS = {
  phone: { width: 390, height: 844 },
  tablet: { width: 768, height: 1024 },
  desktop: { width: 1280, height: 800 },
};
const CODE = 'zero-copy-ssz-decoding-in-rust';
const BAL = 'block-level-access-lists-and-parallel-execution';
const { defaultBrowserType: _, ...iPhone } = devices['iPhone 13'];

// Rows whose children share one center line: [container, children].
const ROWS = [
  ['.titlebar', ':scope > *'],
  ['.location', ':scope > *'],
  ['.status', ':scope > *'],
  ['.taskbar', ':scope > *'],
  ['.dialog-body .row', ':scope > *'],
  ['.now-foot', ':scope > *'],
  ['.topic-list li', ':scope > *'],
  ['.passkey-list li', ':scope > *'],
  ['.dos-bar', ':scope > *'],
  ['.dos-prompt', ':scope > *'],
  ['.dos-keys', ':scope > button'],
  ['.ph-soft', ':scope > button'],
  ['.start-item', ':scope > *'],
  ['.shutdown-buttons', ':scope > *'],
  ['.ve-buttons', ':scope > *'],
  ['.ve-keys', ':scope > button'],
  ['.ve-head', ':scope > *'],
  ['.ve-line', ':scope > *'],
];

/* Runs in the page. Returns the layout faults it finds. */
function measure({ rows, phone }) {
  const W = document.documentElement.clientWidth;
  const name = el => el.tagName.toLowerCase() + (el.id ? `#${el.id}` : '') + [...el.classList].map(c => `.${c}`).join('');
  const path = el => [el.parentElement, el].filter(Boolean).map(name).join(' > ');
  const shown = el => {
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0 && getComputedStyle(el).visibility !== 'hidden';
  };
  // An element in a box that clips or scrolls sideways: the box holds it, not the page.
  const held = el => {
    for (let a = el.parentElement; a && a !== document.body; a = a.parentElement) {
      if (getComputedStyle(a).overflowX !== 'visible') return true;
    }
    return false;
  };
  const edges = [];
  for (const el of document.body.querySelectorAll('*')) {
    if (!shown(el) || held(el)) continue;
    const r = el.getBoundingClientRect();
    if (r.right > W + 0.5 || r.left < -0.5) edges.push(`${path(el)}: ${Math.round(r.left)}..${Math.round(r.right)} of ${W}`);
  }
  const misaligned = [];
  for (const [box, kids] of rows) {
    for (const row of document.querySelectorAll(box)) {
      if (!shown(row)) continue;
      // Bar-sized items only. Tall items (a textarea) align to the top on purpose.
      const items = [...row.querySelectorAll(kids)].filter(shown).map(el => [el, el.getBoundingClientRect()]).filter(([, r]) => r.height <= 48);
      for (let i = 0; i < items.length; i++) {
        for (let j = i + 1; j < items.length; j++) {
          const [a, ra] = items[i], [b, rb] = items[j];
          if (ra.bottom <= rb.top || rb.bottom <= ra.top) continue;
          const d = Math.abs((ra.top + ra.bottom) / 2 - (rb.top + rb.bottom) / 2);
          if (d > 1) misaligned.push(`${path(a)} / ${name(b)}: centers ${d.toFixed(1)} px apart`);
        }
      }
    }
  }
  // Phone mode: every control is a 44 px touch target. Links in post text are prose, not controls.
  const small = !phone ? [] : [...document.querySelectorAll('phone-shell button, phone-shell a:not(.ph-read a)')].filter(shown)
    .map(el => [el, el.getBoundingClientRect()]).filter(([, r]) => r.height < 44 || r.width < 44)
    .map(([el, r]) => `${path(el)}: ${Math.round(r.width)} x ${Math.round(r.height)}`);
  return { scroll: document.documentElement.scrollWidth, width: W, edges, misaligned, small };
}

/** Saves a screenshot and checks the layout of the page as it is now. */
async function check(page, label) {
  await page.evaluate(() => document.fonts.ready);
  await page.screenshot({ path: test.info().outputPath(`${label}.png`), fullPage: true });
  const phone = await page.evaluate(() => document.documentElement.dataset.mode === 'phone');
  const r = await page.evaluate(measure, { rows: ROWS, phone });
  expect.soft(r.scroll, `${label}: the page scrolls sideways`).toBeLessThanOrEqual(r.width);
  expect.soft(r.edges, `${label}: elements past the edge of the screen`).toEqual([]);
  expect.soft(r.misaligned, `${label}: bar items off the center line`).toEqual([]);
  expect.soft(r.small, `${label}: touch targets under 44 px`).toEqual([]);
}

async function go(page, path, ready) {
  await page.goto(path);
  await expect(ready(page)).toBeVisible();
  await page.waitForLoadState('networkidle');
}

for (const [size, viewport] of Object.entries(WIDTHS)) {
  test.describe(`${size} ${viewport.width} px`, () => {
    test.use({ viewport });

    test('Win98: guest views', async ({ page }) => {
      const errors = watchErrors(page);
      await go(page, '/', p => p.locator('#ns-rows a.ttl').first());
      await check(page, `${size}-home`);
      await go(page, `/posts/${CODE}`, p => p.locator('post-view .prose pre').first());
      await check(page, `${size}-post`);
      await go(page, '/topics/rust', p => p.locator('#ns-rows a.ttl').first());
      await check(page, `${size}-topic`);
      await go(page, '/posts/nope', p => p.locator('post-view h1.t', { hasText: 'Not found' }));
      await check(page, `${size}-404`);
      await go(page, '/setup', p => p.locator('#setup-form'));
      await check(page, `${size}-setup`);

      // The Start menu, the Topics submenu, the Shut Down dialog, and the end screen.
      await go(page, '/', p => p.locator('#ns-rows a.ttl').first());
      await page.getByRole('button', { name: 'Start' }).click();
      await check(page, `${size}-start`);
      await page.getByRole('menuitem', { name: 'Topics' }).click();
      await expect(page.getByRole('menu', { name: 'Topics' })).toBeVisible();
      await check(page, `${size}-start-topics`);
      await page.getByRole('menuitem', { name: 'Shut Down…' }).click();
      const dialog = page.getByRole('dialog', { name: 'Shut Down Logbook' });
      await check(page, `${size}-shutdown`);
      await dialog.getByLabel('Shut down', { exact: true }).check();
      await dialog.getByRole('button', { name: 'OK' }).click();
      await expect(page.getByText('It’s now safe to turn off your computer.')).toBeVisible();
      await check(page, `${size}-safe-off`);
      await page.keyboard.press('Enter');
      expect(errors.filter(e => !e.includes('404')), errors.join('\n')).toEqual([]);
    });
  });

  test.describe(`MS-DOS ${size} ${viewport.width} px`, () => {
    // A phone screen is a touch screen: it shows the F8 Mobile key.
    test.use(size === 'phone' ? { viewport, isMobile: true, hasTouch: true } : { viewport });

    test('MS-DOS: prompt, HELP, DIR, TYPE', async ({ page }) => {
      const errors = watchErrors(page);
      await page.goto('/');
      await page.evaluate(() => localStorage.setItem('logbook.mode', 'night'));
      await page.reload();
      await expect(page.locator('dos-shell')).toBeVisible();
      await check(page, `${size}-dos-prompt`);
      const input = page.getByLabel('C:\\LOGBOOK>');
      for (const [cmd, ready, label] of [
        ['help', '.dos-help', 'help'],
        ['dir', '.dos-row', 'dir'],
        [`type ${BAL}`, '.dos-prose pre', 'type'],
      ]) {
        await input.fill(cmd);
        await input.press('Enter');
        await expect(page.locator(ready).first()).toBeVisible();
        await check(page, `${size}-dos-${label}`);
      }
      expect(errors, errors.join('\n')).toEqual([]);
    });
  });
}

// One sign-in for all widths: the server limits sign-in attempts.
test('Win98: owner views at every width', async ({ page }) => {
  test.setTimeout(90_000);
  await signIn(page);
  // A post with a long title, a long code line, a long address, a wide table, and videos.
  await stubYouTube(page);
  await page.getByRole('button', { name: 'New post' }).click();
  await expect(page.locator('md-editor')).toHaveAttribute('data-loaded', '');
  const editUrl = new URL(page.url()).pathname;
  await page.locator('md-editor').getByLabel('Title', { exact: true }).fill('Layout test: a long title that wraps on a phone screen');
  const long = 'let x = '.padEnd(240, 'y') + ';';
  await page.getByLabel('Post markdown').fill(`# Layout\n\nA long address: https://example.com/${'a'.repeat(120)}\n\n\`\`\`rust\n${long}\n\`\`\`\n\n| a | b | c |\n|---|---|---|\n| ${'w'.repeat(80)} | ${'v'.repeat(80)} | c |\n\n${videoBlocks()}`);
  // Save first: a save resets the state choice to the saved state.
  await page.locator('md-editor').getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.locator('[data-dirty]')).toBeHidden();
  await page.getByLabel('Public: anyone can read it').check();
  await page.getByRole('button', { name: 'Publish' }).click();
  await expect(page.locator('md-editor [data-toast]')).toHaveText('Post published. Anyone can read it now.');
  const slug = (await page.locator('[data-file]').textContent()).replace(/\.md$/, '');

  for (const [size, viewport] of Object.entries(WIDTHS)) {
    await page.setViewportSize(viewport);
    await go(page, '/write', p => p.locator('#passkey-list li').first());
    await expect(page.locator('#topic-list li').first()).toBeVisible();
    await check(page, `${size}-write`);
    await go(page, editUrl, p => p.locator('[data-preview] pre'));
    await check(page, `${size}-editor`);
    await go(page, `/posts/${slug}`, p => p.locator('post-view .prose table'));
    await expect(page.locator('post-view .ve-general')).toHaveCount(2);
    await check(page, `${size}-post-wide`);
    // A playing video shows its controls and status.
    await page.locator('post-view video-embed .ve-start').first().click();
    await expect(page.locator('post-view .ve-state').first()).toHaveText('Playing');
    await check(page, `${size}-post-video`);
    await page.evaluate(() => localStorage.setItem('logbook.mode', 'night'));
    await go(page, `/posts/${slug}`, p => p.locator('.dos-prose table'));
    await expect(page.locator('dos-shell .ve-night')).toHaveCount(2);
    await check(page, `${size}-dos-type-wide`);
    await page.locator('dos-shell video-embed .ve-start').first().click();
    await expect(page.locator('dos-shell .ve-fx').first()).toBeVisible();
    await check(page, `${size}-dos-video`);
    await page.evaluate(() => localStorage.setItem('logbook.mode', 'general'));
    await go(page, '/', p => p.locator('#now-open'));
    await check(page, `${size}-home-owner`);
  }
});

test.describe('phone mode 390 px', () => {
  test.use({ ...iPhone, viewport: WIDTHS.phone });
  const shell = page => page.locator('phone-shell');
  const soft = (page, side) => page.locator(`phone-shell [data-soft="${side}"]`);

  test('phone mode: every screen', async ({ page }) => {
    const errors = watchErrors(page);
    await page.goto('/');
    await expect(shell(page).locator('[data-clock]')).toHaveText(/\d\d:\d\d/);
    await check(page, 'phone-standby');
    await soft(page, 'right').tap();
    await expect(shell(page).locator('[data-menu-label]')).toHaveText('Posts');
    await check(page, 'phone-menu');
    for (const [key, label] of [['1', 'posts'], ['2', 'topics'], ['5', 'profiles']]) {
      await page.keyboard.press(key);
      await expect(shell(page).locator('[data-screen="list"], [data-screen="profiles"]').locator('visible=true')).toHaveCount(1);
      await check(page, `phone-${label}`);
      await page.keyboard.press('Escape');
    }
    await page.keyboard.press('6');
    await page.keyboard.press('5');
    await expect(shell(page).locator('[data-banner]')).toBeVisible();
    await check(page, 'phone-snake');

    await page.goto(`/posts/${CODE}`);
    await expect(shell(page).locator('.ph-body pre')).toBeVisible();
    await check(page, 'phone-post');
    await soft(page, 'left').tap();
    await expect(shell(page).getByRole('menu', { name: 'Options' })).toBeVisible();
    await check(page, 'phone-options');
    expect(errors, errors.join('\n')).toEqual([]);
  });

  test('phone mode: a post with videos', async ({ page }) => {
    const errors = watchErrors(page);
    await stubYouTube(page);
    await signIn(page);
    const slug = await videoPost(page, 'Layout test: videos on a phone');
    await page.goto(`/posts/${slug}`);
    await expect(shell(page).locator('.ve-phone')).toHaveCount(2);
    await check(page, 'phone-video');
    await shell(page).locator('video-embed .ve-start').first().tap();
    await expect(shell(page).locator('.ve-line').first()).toContainText('PLAY');
    await check(page, 'phone-video-play');
    expect(errors, errors.join('\n')).toEqual([]);
  });
});
