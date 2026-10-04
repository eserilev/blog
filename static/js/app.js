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

  // Owner mode. In the real build the server decides this from a session cookie,
  // and the editor route and draft API reject every request without one.
  const signin = document.getElementById('signin');
  const setOwner = on => {
    app.toggleAttribute('data-owner', on);
    if (!on && app.dataset.current === 'write') app.go('/', { replace: true });
    if (!on) editNow(false);
  };
  // Step 3 replaces this mock with GET /api/me.
  app.addEventListener('viewchange', e => {
    if (e.detail.view === 'write' && !app.hasAttribute('data-owner')) app.go('/', { replace: true });
  });
  if (app.dataset.current === 'write') app.go('/', { replace: true });
  const closeSignin = () => { signin.hidden = true; };
  document.querySelector('[data-signin]').addEventListener('click', () => {
    signin.hidden = false;
    document.getElementById('signin-pass').focus();
  });
  document.querySelector('[data-signout]').addEventListener('click', () => setOwner(false));
  document.getElementById('signin-form').addEventListener('submit', e => {
    e.preventDefault();
    closeSignin();
    setOwner(true);
  });
  document.getElementById('signin-cancel').addEventListener('click', closeSignin);
  document.getElementById('signin-x').addEventListener('click', closeSignin);
  signin.addEventListener('keydown', e => { if (e.key === 'Escape') closeSignin(); });

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
