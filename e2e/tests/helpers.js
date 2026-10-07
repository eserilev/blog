import { execFileSync } from 'node:child_process';
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
export const YOUTUBE_ID = 'dQw4w9WgXcQ';

/** The markdown of two video blocks: a YouTube video with a title, and one without. */
export const videoBlocks = () =>
  `\`\`\`video\nhttps://www.youtube.com/watch?v=${YOUTUBE_ID}&t=1\nThe test clip\n\`\`\`\n\nA short one:\n\n\`\`\`video\nhttps://youtube.com/shorts/${YOUTUBE_ID}\n\`\`\`\n`;

/** Signed in: publishes a post with the two video blocks. Returns its slug. */
export async function videoPost(page, title) {
  const call = async (method, path, data, version) => {
    const headers = version === undefined ? ORIGIN : { ...ORIGIN, 'If-Match': String(version) };
    const res = await page.request.fetch(path, { method, headers, data });
    if (!res.ok()) throw new Error(`${method} ${path}: HTTP ${res.status()} ${await res.text()}`);
    return res.json();
  };
  const post = await call('POST', '/api/owner/posts', {});
  const body_md = `A post with videos.\n\n${videoBlocks()}\nThe end.\n`;
  const saved = await call('PUT', `/api/owner/posts/${post.id}`, { title, summary: 'Video embeds.', topic: 'rust', tags: [], body_md }, post.version);
  const published = await call('POST', `/api/owner/posts/${post.id}/state`, { state: 'public' }, saved.version);
  return published.slug;
}

/* A stand-in for the YouTube player: the same postMessage protocol, a 180 s
   video, a gray picture. CI never loads the real YouTube. */
const STUB = `<!doctype html><html><body style="margin:0;height:100vh;background:linear-gradient(90deg,#c22,#2c2,#22c)">
<script>
let state = -1, t = 0, vol = 100, muted = false, listening = false, last = Date.now();
const d = 180, autoplay = new URLSearchParams(location.search).get('autoplay') === '1';
window.commands = [];
const send = o => parent.postMessage(JSON.stringify(o), '*');
const info = () => ({ currentTime: t, duration: d, playerState: state, volume: vol, muted });
setInterval(() => {
  const now = Date.now();
  if (state === 1) { t = Math.min(d, t + (now - last) / 1000); if (t >= d) state = 0; }
  last = now;
  if (listening) send({ event: 'infoDelivery', info: info() });
}, 100);
addEventListener('message', e => {
  let m;
  try { m = JSON.parse(e.data); } catch { return; }
  if (m.event === 'listening' && !listening) {
    listening = true;
    if (autoplay) state = 1;
    send({ event: 'onReady', info: null });
    send({ event: 'initialDelivery', info: info() });
  }
  if (m.event === 'command') {
    window.commands.push(m.func);
    if (m.func === 'playVideo') state = 1;
    if (m.func === 'pauseVideo') state = 2;
    if (m.func === 'seekTo') t = m.args[0];
    if (m.func === 'setVolume') vol = m.args[0];
    if (m.func === 'mute') muted = true;
    if (m.func === 'unMute') muted = false;
    send({ event: 'infoDelivery', info: info() });
  }
});
</script></body></html>`;

/** Serves the stub player for youtube-nocookie.com and blocks every other host.
    Returns the list of requests that left the site. */
export async function stubYouTube(page) {
  const seen = [];
  await page.route(url => url.protocol.startsWith('http') && url.origin !== 'http://localhost:18100', route => {
    const url = route.request().url();
    seen.push(url);
    if (url.startsWith('https://www.youtube-nocookie.com/embed/')) {
      return route.fulfill({ status: 200, contentType: 'text/html', body: STUB });
    }
    return route.abort();
  });
  return seen;
}
