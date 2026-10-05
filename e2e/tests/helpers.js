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
