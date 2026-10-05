(() => {
  const app = document.querySelector('blog-app');
  const SITE = "Eitan's Logbook";
  const titleEl = document.getElementById('ns-title');
  const locEl = document.getElementById('ns-loc');

  // The Location bar shows the path as a file under /home/eitan/www.
  const fileFor = path => {
    if (path === '/') return '/index.html';
    if (path.startsWith('/write')) return '/drafts/';
    return `${path}.html`;
  };
  const topicOf = path => (path.match(/^\/topics\/([a-z-]+)$/) || [])[1];
  const setTitle = t => { titleEl.textContent = t; document.title = t === SITE ? SITE : `${t} · ${SITE}`; };

  const onView = ({ view, path }) => {
    locEl.textContent = `file:///home/eitan/www${fileFor(path)}`;
    if (view === 'about') setTitle('About');
    else if (view === 'write') setTitle('Compose');
    else if (view === 'missing') setTitle('Not found');
    else if (view === 'home') setTitle(TOPICS[topicOf(path)] || (path === '/' ? SITE : 'Not found'));
    const logo = document.getElementById('logo');
    logo.classList.add('busy');
    clearTimeout(app._logoT);
    app._logoT = setTimeout(() => logo.classList.remove('busy'), 1400);
  };
  app.addEventListener('viewchange', e => onView(e.detail));
  // The router fired its first viewchange before this script ran.
  if (app.route) onView(app.route);
  app.addEventListener('postloaded', e => setTitle(e.detail ? e.detail.title : 'Not found'));

  // Owner mode (spec 4.6). The server decides it from the session cookie. Hiding the
  // controls is only for the view; every owner API call checks the session again.
  const signin = document.getElementById('signin');
  const signinStatus = document.getElementById('signin-status');
  const say = (el, msg, error) => { el.textContent = msg; el.classList.toggle('error', !!error); };
  const passkeyError = e => (e && e.name === 'NotAllowedError') ? 'The passkey prompt was cancelled or timed out.' : (e && e.message) || String(e);

  const setOwner = on => {
    app.toggleAttribute('data-owner', on);
    if (!on && app.dataset.current === 'write') app.go('/', { replace: true });
    if (!on) editNow(false);
    // The editor and the post list waited for this: show the route again.
    if (on && app.dataset.current === 'write') app.render(false);
  };
  const refreshOwner = async () => setOwner(await Auth.me().catch(() => false));

  app.addEventListener('viewchange', e => {
    if (e.detail.view === 'write') {
      if (!app.hasAttribute('data-owner')) app.go('/', { replace: true });
      else loadPasskeys();
    }
    if (e.detail.view === 'setup') setTitle('Add Passkey');
  });

  const closeSignin = () => { signin.hidden = true; };
  document.querySelector('[data-signin]').addEventListener('click', () => {
    say(signinStatus, '');
    signin.hidden = false;
    document.getElementById('signin-ok').focus();
  });
  document.querySelector('[data-signout]').addEventListener('click', async () => {
    await Auth.logout().catch(() => {});
    setOwner(false);
  });
  document.getElementById('signin-form').addEventListener('submit', async e => {
    e.preventDefault();
    const ok = document.getElementById('signin-ok');
    ok.disabled = true;
    say(signinStatus, 'Waiting for your passkey...');
    try {
      await Auth.login();
      closeSignin();
      await refreshOwner();
    } catch (err) {
      say(signinStatus, `Sign-in failed. ${passkeyError(err)}`, true);
    } finally {
      ok.disabled = false;
    }
  });
  document.getElementById('signin-cancel').addEventListener('click', closeSignin);
  document.getElementById('signin-x').addEventListener('click', closeSignin);
  signin.addEventListener('keydown', e => { if (e.key === 'Escape') closeSignin(); });

  // Passkey list in the Compose view.
  const list = document.getElementById('passkey-list');
  const pkStatus = document.getElementById('passkeys-status');
  const fmtWhen = iso => iso ? new Date(iso).toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' }) : 'never';
  async function loadPasskeys() {
    try {
      const keys = await Auth.passkeys();
      list.replaceChildren(...keys.map(k => {
        const li = document.createElement('li');
        const name = document.createElement('span');
        name.textContent = k.label;
        const info = document.createElement('small');
        info.textContent = `added ${fmtWhen(k.created_at)} · last used ${fmtWhen(k.last_used_at)}`;
        const del = document.createElement('button');
        del.type = 'button';
        del.className = 'btn98';
        del.textContent = 'Revoke';
        del.disabled = keys.length <= 1;
        del.title = keys.length <= 1 ? 'The last passkey cannot be revoked.' : '';
        del.addEventListener('click', async () => {
          try { await Auth.revoke(k.id); say(pkStatus, 'Passkey revoked.'); } catch (err) { say(pkStatus, err.message, true); }
          loadPasskeys();
        });
        const left = document.createElement('div');
        left.append(name, document.createElement('br'), info);
        li.append(left, del);
        return li;
      }));
    } catch {
      say(pkStatus, 'Could not load the passkeys.', true);
    }
  }
  document.getElementById('passkey-add').addEventListener('click', async () => {
    say(pkStatus, 'Waiting for your new passkey...');
    try { await Auth.register({ label: 'Passkey' }); say(pkStatus, 'Passkey added.'); } catch (err) { say(pkStatus, `Could not add the passkey. ${passkeyError(err)}`, true); }
    loadPasskeys();
  });

  // /setup#<token>: register a passkey with a one-time setup token from the CLI.
  document.getElementById('setup-form').addEventListener('submit', async e => {
    e.preventDefault();
    const status = document.getElementById('setup-status');
    const token = location.hash.slice(1);
    if (!token) { say(status, 'This link has no setup token. Run `logbook setup-link` on the server.', true); return; }
    say(status, 'Waiting for your passkey...');
    try {
      await Auth.register({ setupToken: token, label: document.getElementById('setup-label').value });
      history.replaceState(null, '', '/setup');
      say(status, 'Passkey created. You are signed in.');
      await refreshOwner();
      setTimeout(() => app.go('/write'), 800);
    } catch (err) {
      say(status, `Could not create the passkey. ${passkeyError(err)}`, true);
    }
  });

  refreshOwner();

  // The Now section: markdown that only the owner can edit.
  let nowMd = '- Working on Glamsterdam.\n- Write your own lines here: sign in, then click Edit.';
  const nowView = document.getElementById('now-view'), nowEdit = document.getElementById('now-edit');
  const nowText = document.getElementById('now-text'), nowOpen = document.getElementById('now-open');
  // Sample stand-in. Step 6: body_html from the API.
  const renderNow = () => { nowView.innerHTML = md(nowMd); };
  const editNow = on => { nowEdit.hidden = !on; nowView.hidden = on; nowOpen.hidden = on; if (on) { nowText.value = nowMd; nowText.focus(); } };
  nowOpen.addEventListener('click', () => editNow(true));
  document.getElementById('now-cancel').addEventListener('click', () => editNow(false));
  document.getElementById('now-save').addEventListener('click', () => {
    nowMd = nowText.value;
    document.getElementById('now-date').textContent = 'Updated ' + new Date().toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });
    renderNow();
    editNow(false);
  });
  renderNow();

  const clock = () => { document.getElementById('ns-clock').textContent = new Date().toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' }); };
  clock(); setInterval(clock, 30000);
})();
