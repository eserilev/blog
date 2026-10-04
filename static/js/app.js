(() => {
  const app = document.querySelector('blog-app');
  const views = {
    home: ["Eitan's Logbook", 'file:///home/eitan/www/index.html'],
    post: ['Block-level access lists and parallel execution', 'file:///home/eitan/www/posts/block-level-access-lists.html'],
    write: ['Compose - epbs-client-view.md', 'file:///home/eitan/www/drafts/epbs-client-view.md'],
  };
  const sync = v => {
    document.getElementById('ns-title').textContent = views[v][0];
    const logo = document.getElementById('logo');
    logo.classList.add('busy');
    clearTimeout(sync.t);
    sync.t = setTimeout(() => logo.classList.remove('busy'), 1400);
    document.getElementById('ns-loc').textContent = views[v][1];
  };
  app.addEventListener('viewchange', e => sync(e.detail));
  sync(app.dataset.current || 'home');
  // Owner mode. In the real build the server decides this from a session cookie,
  // and the editor route and draft API reject every request without one.
  const signin = document.getElementById('signin');
  const setOwner = on => {
    app.toggleAttribute('data-owner', on);
    if (!on && app.dataset.current === 'write') app.show('home');
    if (!on) editNow(false);
  };
  app.addEventListener('viewchange', e => {
    if (e.detail === 'write' && !app.hasAttribute('data-owner')) app.show('home', true);
  });
  if (app.dataset.current === 'write') app.show('home', true);
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

  // Show the "New" label only on new posts.
  document.querySelectorAll('.new.is-new').forEach(el => { el.hidden = false; });
  const clock = () => { document.getElementById('ns-clock').textContent = new Date().toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' }); };
  clock(); setInterval(clock, 30000);
})();
