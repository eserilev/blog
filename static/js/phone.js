/* Phone mode (spec 11.8): the site as a monochrome phone from 2000.
   html[data-mode="phone"] shows <phone-shell> and hides the Win98 desktop. Only a
   phone gets this mode (mode.js). The shell uses the same data code as the other
   views (Api, TOPICS, matchRoute). Only server-sanitized body_html goes into
   innerHTML. All other text uses textContent.

   Screens: standby, menu, posts, topics, topic, post, now, surf, profiles, games,
   missing. Three of them have their own address: / (standby), /posts/{slug}, and
   /topics/{t}. The others live at / with their name in history.state.
   Each screen change is a history entry, so the Back softkey and the browser Back
   button do the same thing. */
(() => {
  const SIZES = ['s', 'm', 'l'];
  const SIZE_NAMES = { s: 'Small', m: 'Normal', l: 'Large' };
  const BOARD_W = 20, BOARD_H = 16;
  /* The menu: label, screen, and a 16x12 pixel icon. */
  const MENU = [
    ['Posts', 'posts', 'M2 1h10v10H2zM3 2v8h8V2zM4 3h6v1H4zM4 5h6v1H4zM4 7h4v1H4zM13 2h1v9h-1z'],
    ['Topics', 'topics', 'M1 2h5l1 1h7v8H1zM2 5v5h12V5z'],
    ['Now', 'now', 'M6 1h4v1h2v1h1v2h1v4h-1v1h-1v1h-2v1H6v-1H4v-1H3V9H2V5h1V3h1V2h2zM7 3v4h3V6H8V3z'],
    ['Surf', 'surf', 'M0 7h1V6h2v1h1V6h2v1h1V6h2v1h1V6h2v1h1V6h2v1h1v2H0zM0 10h16v1H0zM9 1h3v1h1v2h-1V3H9z'],
    ['Profiles', 'profiles', 'M3 1h10v10H3zM4 2v8h8V2zM5 3h1v6H5zM7 5h1v4H7zM9 4h1v5H9zM11 6h1v3h-1z'],
    ['Games', 'games', 'M1 4h3v1h1v1h4V5h1V4h3v1h1v4h-1v1h-1v1h-1v-1H6v1H5v-1H4V9H3V5h1zM4 6v1h1V6zM11 6v1h1V6zM10 7v1h1V7z'],
  ];
  /* Back from a screen that has no earlier history entry (a shared link). */
  const PARENT = {
    menu: ['standby', '/'], posts: ['menu', '/'], topics: ['menu', '/'], now: ['menu', '/'],
    surf: ['menu', '/'], profiles: ['menu', '/'], games: ['menu', '/'],
    topic: ['topics', '/'], post: ['posts', '/'], missing: ['standby', '/'],
  };
  const AT_ROOT = ['standby', 'menu', 'posts', 'topics', 'now', 'surf', 'profiles', 'games'];
  const SEEN_KEY = 'logbook.seen';
  const two = n => String(n).padStart(2, '0');
  const store = {
    get(k) { try { return localStorage.getItem(k); } catch { return null; } },
    set(k, v) { try { localStorage.setItem(k, v); } catch { /* This page only. */ } },
  };

  customElements.define('phone-shell', class extends HTMLElement {
    connectedCallback() {
      this.app = this.closest('blog-app');
      const $ = s => this.querySelector(s);
      this.el = {
        time: $('[data-time]'), batt: $('[data-batt]'), clock: $('[data-clock]'), date: $('[data-date]'),
        site: $('[data-site]'), fresh: $('[data-new]'), menuIndex: $('[data-menu-index]'),
        menuOpen: $('[data-menu-open]'), menuIcon: $('[data-menu-icon]'), menuLabel: $('[data-menu-label]'),
        listTitle: $('[data-list-title]'), list: $('[data-list]'), listEmpty: $('[data-list-empty]'),
        textTitle: $('[data-text-title]'), pages: $('[data-pages]'), read: $('[data-read]'),
        thumb: $('[data-thumb]'), options: $('[data-options]'), optionItems: $('[data-option-items]'),
        toast: $('[data-toast]'), left: $('[data-soft="left"]'), right: $('[data-soft="right"]'),
        board: $('[data-board]'), snake: $('[data-snake]'), food: $('[data-food]'), banner: $('[data-banner]'),
        score: $('[data-score]'), snakeStatus: $('[data-snake-status]'), pause: $('[data-dir="5"]'),
      };
      this.sections = Object.fromEntries([...this.querySelectorAll('[data-screen]')].map(s => [s.dataset.screen, s]));
      this.screen = null;
      this.sel = 0;
      this.size = 'm';
      this.clean = false;
      this.optionsOpen = null;

      this.tick();
      this.clockTimer = setInterval(() => this.tick(), 15000);

      this.el.left.addEventListener('click', () => this.leftKey());
      this.el.right.addEventListener('click', () => this.rightKey());
      this.el.fresh.addEventListener('click', () => this.readNew());
      this.el.menuOpen.addEventListener('click', () => this.openMenu(this.sel));
      this.querySelectorAll('[data-step]').forEach(b => b.addEventListener('click', () => this.step(Number(b.dataset.step))));
      this.querySelectorAll('[data-profile]').forEach((b, i) => {
        b.addEventListener('click', () => this.pickProfile(b.dataset.profile));
        b.addEventListener('focus', () => this.select(i, false));
      });
      this.querySelectorAll('[data-dir]').forEach(b => b.addEventListener('click', () => this.snakeKey(b.dataset.dir)));
      this.swipe(this.el.menuOpen, (dx) => { if (Math.abs(dx) > 40) { this.step(dx < 0 ? 1 : -1); return true; } return false; });
      this.swipe(this.el.board, (dx, dy) => {
        if (Math.max(Math.abs(dx), Math.abs(dy)) < 24) return false;
        this.turn(Math.abs(dx) > Math.abs(dy) ? [Math.sign(dx), 0] : [0, Math.sign(dy)]);
        return true;
      });
      // One scroll container: the page. The reading area is not a scroll box.
      window.addEventListener('scroll', () => this.measure(), { passive: true });
      // Images change the height of a post when they load.
      this.el.read.addEventListener('load', () => this.measure(), true);
      window.addEventListener('resize', () => this.measure());
      // The pixel font changes the height of a post when it loads.
      document.fonts?.ready.then(() => this.measure());
      // Page links inside the shell (lists, post bodies) become history entries here.
      this.addEventListener('click', e => this.onLink(e));
      document.addEventListener('keydown', e => this.onKey(e));
      document.addEventListener('visibilitychange', () => { if (document.hidden) this.setPaused(true); });
      document.addEventListener('modechange', () => {
        this.stopSnake();
        if (this.on()) { this.screen = null; this.at = null; this.route(this.app.route); }
      });
      this.app.addEventListener('viewchange', e => { if (this.on()) this.route(e.detail); });
      if (this.on() && this.app.route) this.route(this.app.route);
    }

    on() { return Mode.current === 'phone'; }

    /* The clock on standby and in the status row. */
    tick() {
      const now = new Date();
      const hm = `${two(now.getHours())}:${two(now.getMinutes())}`;
      this.el.clock.textContent = hm;
      this.el.time.textContent = this.screen === 'standby' ? '' : hm;
      this.el.date.textContent = `${now.toLocaleDateString('en-US', { weekday: 'short' })} ${two(now.getDate())}.${two(now.getMonth() + 1)}.${now.getFullYear()}`;
    }

    /* Touch swipes. fn(dx, dy) returns true when it used the swipe; then no click follows. */
    swipe(el, fn) {
      let x = 0, y = 0;
      el.addEventListener('touchstart', e => { x = e.changedTouches[0].clientX; y = e.changedTouches[0].clientY; }, { passive: true });
      el.addEventListener('touchend', e => {
        const t = e.changedTouches[0];
        if (fn(t.clientX - x, t.clientY - y)) e.preventDefault();
      });
    }

    /* ---- Navigation ---- */

    /* Opens a screen as a new history entry. The current entry keeps its selection,
       so Back shows the same row again. */
    go(screen, { path = '/', replace = false, render = true } = {}) {
      const st = history.state || {};
      const depth = st.depth || 0;
      if (!replace) history.replaceState({ ...st, phone: this.screen, sel: this.sel, depth }, '', location.pathname);
      history[replace ? 'replaceState' : 'pushState']({ phone: screen, sel: 0, depth: replace ? depth : depth + 1 }, '', path);
      if (render) { this.app.render(false); return; }
      // More entries follow before the next render.
      this.screen = screen;
      this.sel = 0;
    }

    back() {
      // Back from the section list shows the main options again.
      if (this.optionsOpen === 'sections') { this.openOptions(); return; }
      if (this.optionsOpen) { this.closeOptions(); return; }
      if ((history.state?.depth || 0) > 0) { history.back(); return; }
      const parent = PARENT[this.screen];
      if (parent) this.go(parent[0], { path: parent[1], replace: true });
    }

    onLink(e) {
      if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
      const a = e.target.closest('a[href]');
      if (!a || a.target || a.hasAttribute('download')) return;
      const url = new URL(a.href, location.href);
      if (url.origin !== location.origin) return;
      const r = matchRoute(url.pathname);
      if (!['post', 'home'].includes(r.view)) return;
      e.preventDefault();
      this.go(this.screenFor(r), { path: url.pathname });
    }

    screenFor(r) {
      if (r.view === 'post') return 'post';
      if (r.view === 'home' && r.param) return 'topic';
      if (r.view === 'home') {
        const s = history.state?.phone;
        return AT_ROOT.includes(s) ? s : 'standby';
      }
      return 'missing';
    }

    /* The router calls this on each view change. */
    route(r) {
      if (r.view === 'write' || r.view === 'setup') return;
      const screen = this.screenFor(r);
      const key = `${screen} ${r.path}`;
      if (key === this.at && screen !== 'standby') return;
      this.at = key;
      this.stopSnake();
      this.closeOptions(false);
      this.screen = screen;
      this.sel = history.state?.sel || 0;
      this.tick();
      ({
        standby: () => this.showStandby(),
        menu: () => this.showMenu(),
        posts: () => this.showPosts(null),
        topic: () => this.showPosts(r.param),
        topics: () => this.showTopics(),
        post: () => this.showPost(r.param),
        now: () => this.showNow(),
        surf: () => this.showSurf(),
        profiles: () => this.showProfiles(),
        games: () => this.showSnake(),
        missing: () => this.showText('Not found', [this.para('There is nothing at this address.')]),
      })[screen]();
    }

    /* Shows one section. Moves the focus there unless the page just loaded. */
    show(name, focus) {
      for (const [k, s] of Object.entries(this.sections)) s.hidden = k !== name;
      // A new screen starts at the top of the page.
      window.scrollTo({ top: 0 });
      this.el.thumb.hidden = true;
      this.setBattery(75);
      this.soft();
      if (focus && this.loaded) focus.focus({ preventScroll: true });
      this.loaded = true;
    }

    /* The softkey labels for the current screen. */
    soft() {
      let left = 'Select', right = 'Back';
      if (this.screen === 'standby') [left, right] = this.unread ? ['Read', 'Menu'] : ['Menu', 'Posts'];
      else if (this.optionsOpen) left = 'Select';
      else if (this.sections.text && !this.sections.text.hidden) left = 'Options';
      else if (this.screen === 'games') left = this.game?.over ? 'Again' : 'Restart';
      this.el.left.firstElementChild.textContent = left;
      this.el.right.firstElementChild.textContent = right;
    }

    leftKey() {
      if (this.optionsOpen) { this.optionButtons()[this.optSel]?.click(); return; }
      switch (this.screen) {
        case 'standby': if (this.unread) this.readNew(); else this.go('menu'); break;
        case 'menu': this.openMenu(this.sel); break;
        case 'posts': case 'topic': case 'topics': this.listLinks()[this.sel]?.click(); break;
        case 'profiles': this.querySelectorAll('[data-profile]')[this.sel]?.click(); break;
        case 'games': this.startSnake(); break;
        default: if (!this.sections.text.hidden) this.openOptions();
      }
    }

    rightKey() {
      if (this.screen === 'standby' && !this.optionsOpen) { this.go(this.unread ? 'menu' : 'posts'); return; }
      this.back();
    }

    /* Keyboard: arrows and Page Up/Down move, Enter is the left softkey, Esc and
       Backspace go back. Number keys open menu items and steer the snake. */
    onKey(e) {
      if (!this.on() || !this.getClientRects().length || !this.screen || e.altKey || e.ctrlKey || e.metaKey) return;
      if (e.target.closest?.('input, textarea, select, [contenteditable]')) return;
      const k = e.key;
      const run = fn => { e.preventDefault(); fn(); };
      if (this.screen === 'games' && !this.optionsOpen) {
        if (['2', '4', '5', '6', '8'].includes(k)) return run(() => this.snakeKey(k));
        const d = { ArrowUp: [0, -1], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowRight: [1, 0] }[k];
        if (d) return run(() => this.turn(d));
      }
      if (k === 'Escape' || k === 'Backspace') return run(() => this.back());
      if (k === 'Enter') {
        // A focused button or link does its own job on Enter.
        if (e.target.closest?.('button, a[href]')) return;
        return run(() => this.leftKey());
      }
      const up = k === 'ArrowUp' || k === 'PageUp', down = k === 'ArrowDown' || k === 'PageDown';
      if (this.optionsOpen && (up || down)) return run(() => this.moveOption(down ? 1 : -1));
      if (this.screen === 'menu') {
        if (/^[1-9]$/.test(k)) return run(() => this.openMenu(Number(k) - 1));
        if (up || k === 'ArrowLeft') return run(() => this.step(-1));
        if (down || k === 'ArrowRight') return run(() => this.step(1));
      }
      if (!this.sections.text.hidden && (up || down)) return run(() => this.page(down ? 1 : -1));
      if (['posts', 'topic', 'topics', 'profiles'].includes(this.screen) && (up || down)) return run(() => this.step(down ? 1 : -1));
    }

    /* ---- Standby ---- */

    async showStandby() {
      this.el.site.textContent = document.getElementById('site-title')?.textContent || "Eitan's Logbook";
      this.show('standby', this.querySelector('.ph-main'));
      const posts = await Api.posts().catch(() => null);
      if (this.screen !== 'standby') return;
      this.newest = posts && posts[0];
      const seen = store.get(SEEN_KEY);
      this.unread = !!this.newest && (!seen || new Date(this.newest.published_at) > new Date(seen));
      this.el.fresh.hidden = !this.unread;
      this.soft();
    }

    /* Read: the newest post. Menu and Posts go into the history first, so Back
       goes to the post list. */
    readNew() {
      if (!this.newest) return;
      this.go('menu', { render: false });
      this.go('posts', { render: false });
      this.go('post', { path: `/posts/${this.newest.slug}` });
    }

    /* ---- Menu ---- */

    showMenu() {
      this.sel = Math.min(this.sel, MENU.length - 1);
      this.drawMenu();
      this.show('menu', this.el.menuOpen);
    }

    drawMenu() {
      const [label, , icon] = MENU[this.sel];
      this.el.menuIndex.textContent = String(this.sel + 1);
      this.el.menuLabel.textContent = label;
      this.el.menuIcon.setAttribute('d', icon);
      this.el.menuOpen.setAttribute('aria-label', `Open ${label}, item ${this.sel + 1} of ${MENU.length}`);
    }

    openMenu(i) {
      if (!MENU[i]) return;
      this.sel = i;
      const screen = MENU[i][1];
      this.go(screen, { path: '/' });
    }

    /* Moves the selection in the menu or a list, with wrap-around. */
    step(d) {
      const n = this.screen === 'menu' ? MENU.length
        : this.screen === 'profiles' ? 3 : this.listLinks().length;
      if (!n) return;
      this.select((this.sel + d + n) % n, true);
    }

    select(i, focus) {
      this.sel = i;
      if (this.screen === 'menu') { this.drawMenu(); return; }
      const rows = this.screen === 'profiles' ? [...this.querySelectorAll('[data-profile]')] : this.listLinks();
      rows.forEach((r, k) => r.classList.toggle('on', k === i));
      if (focus && rows[i] && document.activeElement !== rows[i]) rows[i].focus({ preventScroll: true });
      rows[i]?.scrollIntoView({ block: 'nearest' });
    }

    /* ---- Lists ---- */

    listLinks() { return [...this.el.list.querySelectorAll('a')]; }

    drawList(title, items, empty) {
      this.el.listTitle.textContent = title;
      this.el.list.replaceChildren(...items.map(([label, href], i) => {
        const li = document.createElement('li');
        const a = document.createElement('a');
        a.href = href;
        a.textContent = label;
        a.addEventListener('focus', () => this.select(i, false));
        li.append(a);
        return li;
      }));
      this.el.listEmpty.textContent = empty || '';
      this.el.listEmpty.hidden = items.length > 0;
      this.sel = Math.min(this.sel, Math.max(0, items.length - 1));
      this.show('list', this.listLinks()[this.sel] || this.querySelector('.ph-main'));
      // After show: Back to a list scrolls the selected row into view again.
      this.select(this.sel, false);
    }

    async showPosts(topic) {
      const screen = this.screen;
      this.drawList(topic ? (TOPICS[topic] || topic) : 'Posts', [], 'Loading...');
      let posts;
      try { posts = topic ? await Api.topic(topic) : await Api.posts(); } catch { posts = undefined; }
      if (this.screen !== screen) return;
      const empty = posts === undefined ? 'Could not load the posts.' : posts === null ? 'This topic does not exist.' : 'No posts yet.';
      this.drawList(topic ? (TOPICS[topic] || topic) : 'Posts', (posts || []).map(p => [p.title, `/posts/${p.slug}`]), empty);
    }

    showTopics() {
      this.drawList('Topics', Object.entries(TOPICS).map(([slug, name]) => [name, `/topics/${slug}`]), 'No topics yet.');
    }

    /* ---- Text screens: post, now, surf ---- */

    para(text, cls) {
      const p = document.createElement('p');
      if (cls) p.className = cls;
      p.textContent = text;
      return p;
    }

    /* Puts nodes in the reading area. url is the address for Copy link. */
    showText(title, nodes, url) {
      this.el.textTitle.textContent = title;
      this.url = url || null;
      const read = this.el.read;
      read.dataset.size = this.size;
      read.toggleAttribute('data-clean', this.clean);
      read.replaceChildren(...nodes);
      read.setAttribute('aria-label', title);
      this.show('text', read);
      this.measure();
    }

    async showPost(slug) {
      this.showText('Post', [this.para('Loading...')]);
      let post;
      try { post = await Api.post(slug); } catch { post = undefined; }
      if (this.screen !== 'post' || this.at !== `post /posts/${slug}`) return;
      if (!post) {
        this.showText('Post', [this.para(post === null ? 'There is no public post at this address.' : 'Could not load this post.')]);
        return;
      }
      const h = document.createElement('h2');
      h.className = 'ph-title';
      h.textContent = post.title;
      const meta = this.para(`${fmtDate(post.published_at, 'us')} · ${post.topic_name} · ${post.reading_minutes} min read${byAuthor}`, 'ph-meta');
      const body = document.createElement('div');
      body.className = 'ph-body';
      // body_html is rendered and sanitized on the server (spec 6.7).
      body.innerHTML = post.body_html;
      this.showText('Post', [h, meta, body], `/posts/${slug}`);
      // Reading the newest post clears "1 new post" on standby.
      const newest = (await Api.posts().catch(() => null))?.[0];
      if (newest?.slug === slug) store.set(SEEN_KEY, newest.published_at);
    }

    async showNow() {
      this.showText('Now', [this.para('Loading...')]);
      let now;
      try { now = await Api.get('/api/now'); } catch { now = null; }
      if (this.screen !== 'now') return;
      if (!now) { this.showText('Now', [this.para('Could not load the Now box.')]); return; }
      if (!now.body_html) { this.showText('Now', [this.para('Nothing here yet.')]); return; }
      const body = document.createElement('div');
      body.className = 'ph-body';
      // body_html is rendered and sanitized on the server.
      body.innerHTML = now.body_html;
      const nodes = [body];
      if (now.updated_at) nodes.push(this.para(`Updated ${fmtDate(now.updated_at, 'us')}`, 'ph-meta'));
      this.showText('Now', nodes);
    }

    /* The surf report as text (spec 4.5). A readout older than 3 h is marked stale. */
    async showSurf() {
      this.showText('Surf', [this.para('Loading...')]);
      let data;
      try { data = await Api.get('/api/surf'); } catch { data = null; }
      if (this.screen !== 'surf') return;
      if (!data || !data.available) { this.showText('Surf', [this.para('No NOAA data yet.')]); return; }
      const s = data.surf;
      const stale = iso => !iso || Date.now() - new Date(iso).getTime() > 3 * 3600e3;
      const nodes = [this.para('Redondo Beach', 'ph-title')];
      const item = (label, value, iso) => nodes.push(this.para(`${label} ${value ?? '--'}${value != null && stale(iso) ? ' (stale)' : ''}`));
      item('Swell', s.swell ? `${s.swell.height_ft.toFixed(1)} ft @ ${Math.round(s.swell.period_s)} s ${s.swell.direction}` : null, s.swell?.observed_at);
      item('Wind', s.wind ? `${Math.round(s.wind.speed_kt)} kt ${s.wind.direction}${s.wind.offshore ? ' offshore' : ''}` : null, s.wind?.observed_at);
      const c = s.waves?.water_c;
      item('Water', c != null ? `${Math.round(c * 9 / 5 + 32)} °F` : null, s.waves?.observed_at);
      const next = (s.tides || []).map(p => ({ ...p, at: new Date(p.time.replace(' ', 'T')) })).filter(p => p.at > Date.now()).slice(0, 2);
      for (const p of next) {
        nodes.push(this.para(`${p.kind === 'H' ? 'High' : 'Low'} ${p.at.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })} · ${p.height_ft.toFixed(1)} ft`));
      }
      nodes.push(this.para('Data: NOAA NDBC, NWS, CO-OPS', 'ph-meta'));
      this.showText('Surf', nodes);
    }

    /* The height of the post that shows between the sticky title and the softkeys. */
    view() {
      const h = el => el.getBoundingClientRect().height;
      const bars = h(this.querySelector('.ph-status')) + h(this.sections.text.querySelector('.ph-head')) + h(this.querySelector('.ph-soft'));
      return Math.max(1, window.innerHeight - bars);
    }

    /* Page number, scroll bar, and battery from the scroll position of the page. */
    measure() {
      if (!this.on() || this.sections.text.hidden) return;
      const h = this.view();
      const pages = Math.max(1, Math.ceil((this.el.read.scrollHeight - 2) / h));
      const max = document.documentElement.scrollHeight - window.innerHeight;
      const frac = max > 0 ? Math.min(1, window.scrollY / max) : 0;
      // The top of the page is page 1, the end is the last page.
      const page = Math.min(pages, Math.round(frac * (pages - 1)) + 1);
      this.el.pages.textContent = `${page}/${pages}`;
      this.el.pages.setAttribute('aria-label', `Page ${page} of ${pages}`);
      this.el.thumb.hidden = pages < 2;
      this.el.thumb.firstElementChild.style.setProperty('top', `calc(${(frac * 100).toFixed(1)}% - ${Math.round(frac * 28)}px)`);
      this.setBattery(pages > 1 ? Math.round(100 - frac * 85) : 75);
    }

    setBattery(pct) { this.el.batt.style.setProperty('width', `${pct}%`); }

    /* Page Up/Down: 85 % of the visible height. */
    page(d) {
      window.scrollBy({ top: d * this.view() * 0.85, behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth' });
    }

    /* ---- Options ---- */

    optionButtons() { return [...this.el.optionItems.querySelectorAll('button')]; }

    openOptions(kind = 'main') {
      const read = this.el.read;
      const smooth = matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth';
      const heads = [...read.querySelectorAll('.ph-body h1, .ph-body h2, .ph-body h3')];
      const items = kind === 'sections'
        ? heads.map(h => [`§ ${h.textContent}`, () => { this.closeOptions(); h.scrollIntoView({ behavior: smooth, block: 'start' }); }])
        : [
          ['Top', () => { this.closeOptions(); window.scrollTo({ top: 0, behavior: smooth }); }],
          ...(heads.length ? [['Jump to section', () => this.openOptions('sections')]] : []),
          [`Text size: ${SIZE_NAMES[this.size]}`, () => {
            this.size = SIZES[(SIZES.indexOf(this.size) + 1) % SIZES.length];
            read.dataset.size = this.size;
            this.openOptions();
            this.measure();
          }],
          [`Font: ${this.clean ? 'Clean' : 'Pixel'}`, () => {
            this.clean = !this.clean;
            read.toggleAttribute('data-clean', this.clean);
            this.openOptions();
            this.measure();
          }],
          ...(this.url ? [['Copy link', () => this.copyLink()]] : []),
        ];
      this.optionsOpen = kind;
      this.el.optionItems.replaceChildren(...items.map(([label, fn], i) => {
        const b = document.createElement('button');
        b.type = 'button';
        b.setAttribute('role', 'menuitem');
        b.textContent = label;
        b.addEventListener('click', fn);
        b.addEventListener('focus', () => this.moveOption(i - this.optSel));
        return b;
      }));
      this.el.options.hidden = false;
      this.optSel = 0;
      this.moveOption(0);
      this.soft();
    }

    moveOption(d) {
      const items = this.optionButtons();
      if (!items.length) return;
      this.optSel = Math.max(0, Math.min(items.length - 1, this.optSel + d));
      items.forEach((b, i) => b.classList.toggle('on', i === this.optSel));
      if (document.activeElement !== items[this.optSel]) items[this.optSel].focus({ preventScroll: true });
      items[this.optSel].scrollIntoView({ block: 'nearest' });
    }

    closeOptions(focus = true) {
      if (!this.optionsOpen) return;
      this.optionsOpen = null;
      this.el.options.hidden = true;
      this.soft();
      if (focus) this.el.read.focus({ preventScroll: true });
    }

    async copyLink() {
      this.closeOptions();
      try {
        await navigator.clipboard.writeText(location.origin + this.url);
        this.toast('Link copied.');
      } catch {
        this.toast('Could not copy the link.');
      }
    }

    toast(msg) {
      this.el.toast.textContent = msg;
      clearTimeout(this.toastT);
      this.toastT = setTimeout(() => { this.el.toast.textContent = ''; }, 2500);
    }

    /* ---- Profiles ---- */

    showProfiles() {
      this.querySelectorAll('[data-profile]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.profile === Mode.current)));
      this.sel = Math.min(this.sel, 2);
      this.select(this.sel, false);
      this.show('profiles', this.querySelectorAll('[data-profile]')[this.sel]);
    }

    pickProfile(mode) {
      if (mode === 'phone') { this.back(); return; }
      // The other views start at the address of this entry: / for Profiles.
      Mode.set(mode);
    }

    /* ---- Snake: 20 x 16, +9 per food, no wrap. ---- */

    showSnake() {
      this.show('snake', this.el.board);
      this.startSnake();
    }

    startSnake() {
      this.stopSnake();
      this.game = { body: [[6, 8], [5, 8], [4, 8]], dir: [1, 0], last: [1, 0], food: [12, 8], over: false, won: false, paused: false, score: 0 };
      this.timer = setInterval(() => this.snakeStep(), 220);
      this.drawSnake();
      this.soft();
    }

    stopSnake() { clearInterval(this.timer); this.timer = 0; }

    snakeStep() {
      const g = this.game;
      if (!g || g.over || g.paused) return;
      const head = [g.body[0][0] + g.dir[0], g.body[0][1] + g.dir[1]];
      const ate = head[0] === g.food[0] && head[1] === g.food[1];
      // The tail moves away in this step, so the head can take its cell.
      const rest = ate ? g.body : g.body.slice(0, -1);
      const hit = head[0] < 0 || head[1] < 0 || head[0] >= BOARD_W || head[1] >= BOARD_H || rest.some(p => p[0] === head[0] && p[1] === head[1]);
      if (hit) { this.endSnake(false); return; }
      g.body = [head, ...rest];
      g.last = g.dir;
      if (ate) {
        g.score += 9;
        if (g.body.length >= BOARD_W * BOARD_H) { this.endSnake(true); return; }
        const free = [];
        for (let y = 0; y < BOARD_H; y++) for (let x = 0; x < BOARD_W; x++) if (!g.body.some(p => p[0] === x && p[1] === y)) free.push([x, y]);
        g.food = free[Math.floor(Math.random() * free.length)];
      }
      this.drawSnake();
    }

    endSnake(won) {
      this.game.over = true;
      this.game.won = won;
      this.stopSnake();
      this.drawSnake();
      this.soft();
    }

    drawSnake() {
      const g = this.game;
      this.el.snake.setAttribute('d', g.body.map(([x, y]) => `M${x} ${y}h1v1h-1z`).join(''));
      this.el.food.setAttribute('x', String(g.food[0] + 0.15));
      this.el.food.setAttribute('y', String(g.food[1] + 0.15));
      this.el.score.textContent = String(g.score).padStart(4, '0');
      const banner = g.won ? `You win! ${g.score}` : g.over ? `Game over · ${g.score}` : g.paused ? 'Paused · press 5' : '';
      this.el.banner.textContent = banner;
      this.el.banner.hidden = !banner;
      this.el.pause.setAttribute('aria-label', g.paused ? 'Resume (5)' : 'Pause (5)');
      const status = g.won ? `You win. Score ${g.score}` : g.over ? `Game over. Score ${g.score}` : g.paused ? `Paused. Score ${g.score}` : `Score ${g.score}`;
      // Speak only on a change of score or state, not on each step.
      if (this.el.snakeStatus.textContent !== status) this.el.snakeStatus.textContent = status;
    }

    turn(d) {
      const g = this.game;
      if (!g || g.over || g.paused) return;
      if (g.last[0] === -d[0] && g.last[1] === -d[1]) return;
      g.dir = d;
    }

    setPaused(on) {
      const g = this.game;
      if (!g || g.over || !this.timer || g.paused === on) return;
      g.paused = on;
      this.drawSnake();
    }

    snakeKey(k) {
      if (k === '5') { if (this.game && !this.game.over) this.setPaused(!this.game.paused); return; }
      this.turn({ 2: [0, -1], 4: [-1, 0], 6: [1, 0], 8: [0, 1] }[k]);
    }
  });
})();
