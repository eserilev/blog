import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');

/** Runs `logbook setup-link` against the test database. Returns the link. */
export function setupLink() {
  const out = execFileSync(path.join(root, 'target/debug/logbook'), ['setup-link'], {
    env: { ...process.env, LOGBOOK_DB: path.join(root, 'e2e/.tmp/e2e.db'), LOGBOOK_ORIGIN: 'http://localhost:18100' },
    encoding: 'utf8',
  });
  const link = out.split('\n').find(l => l.startsWith('http'));
  if (!link) throw new Error(`no link in: ${out}`);
  return link;
}

/** Adds a virtual passkey authenticator to the page (Chromium DevTools protocol). */
export async function virtualAuthenticator(page) {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('WebAuthn.enable');
  const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2',
      transport: 'internal',
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
  return { cdp, authenticatorId };
}

/** Collects CSP violations and page errors. */
export function watchErrors(page) {
  const errors = [];
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  page.on('pageerror', e => errors.push(String(e)));
  return errors;
}

/** Registers a new passkey with a setup link and leaves the page signed in. */
export async function signIn(page) {
  await virtualAuthenticator(page);
  await page.goto(setupLink());
  await page.getByRole('button', { name: 'Create passkey' }).click();
  await page.waitForURL('**/write');
}

const ORIGIN = { Origin: 'http://localhost:18100' };
/** The test clip: 4 s, 96 x 54, VP9 WebM, no sound. */
export const CLIP = fs.readFileSync(path.join(root, 'e2e/fixtures/clip.webm'));
export const YOUTUBE_ID = 'dQw4w9WgXcQ';

/** Signed in: uploads the test clip. Returns its /media/ address. */
export async function uploadClip(page) {
  const res = await page.request.post('/api/owner/uploads', {
    headers: ORIGIN,
    multipart: { file: { name: 'clip.webm', mimeType: 'video/webm', buffer: CLIP } },
  });
  if (!res.ok()) throw new Error(`upload: HTTP ${res.status()} ${await res.text()}`);
  return (await res.json()).url;
}

/** The markdown of two video blocks: the test clip and a YouTube video. */
export const videoBlocks = url =>
  `\`\`\`video\n${url}\nThe test clip\n\`\`\`\n\nAnd one from YouTube:\n\n\`\`\`video\nhttps://youtu.be/${YOUTUBE_ID}\nA YouTube video\n\`\`\`\n`;

/** Signed in: publishes a post with the two video blocks. Returns its slug. */
export async function videoPost(page, title) {
  const url = await uploadClip(page);
  const call = async (method, path, data, version) => {
    const headers = version === undefined ? ORIGIN : { ...ORIGIN, 'If-Match': String(version) };
    const res = await page.request.fetch(path, { method, headers, data });
    if (!res.ok()) throw new Error(`${method} ${path}: HTTP ${res.status()} ${await res.text()}`);
    return res.json();
  };
  const post = await call('POST', '/api/owner/posts', {});
  const body_md = `A post with videos.\n\n${videoBlocks(url)}\nThe end.\n`;
  const saved = await call('PUT', `/api/owner/posts/${post.id}`, { title, summary: 'Video embeds.', topic: 'rust', tags: [], body_md }, post.version);
  const published = await call('POST', `/api/owner/posts/${post.id}/state`, { state: 'public' }, saved.version);
  return published.slug;
}
