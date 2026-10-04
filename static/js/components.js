/* Native web components for the Logbook (spec 5).
   Rule: only server-rendered body_html goes into innerHTML. Everything else
   uses textContent. (The editor and the Now box use the sample md() until
   steps 5 and 6; its output is escaped.) */

/* API calls. Lists are cached for the page's lifetime. */
const Api = {
  _cache: new Map(),
  async get(url) {
    const res = await fetch(url, { headers: { Accept: 'application/json' } });
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
    return res.json();
  },
  posts() {
    if (!this._cache.has('posts')) this._cache.set('posts', this.get('/api/posts'));
    return this._cache.get('posts');
  },
  topic(t) { return this.get(`/api/topics/${encodeURIComponent(t)}`); },
  post(slug) { return this.get(`/api/posts/${encodeURIComponent(slug)}`); },
};

/* Passkeys (spec 6.6). The server sends WebAuthn options as JSON with base64url
   byte fields; the browser API wants ArrayBuffers. These helpers convert both ways. */
const b64u = {
  decode(s) {
    const pad = '='.repeat((4 - (s.length % 4)) % 4);
    const bin = atob(s.replace(/-/g, '+').replace(/_/g, '/') + pad);
    return Uint8Array.from(bin, c => c.charCodeAt(0)).buffer;
  },
  encode(buf) {
    let bin = '';
    for (const b of new Uint8Array(buf)) bin += String.fromCharCode(b);
    return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  },
};

const Auth = {
  /* POST JSON. The browser adds the Origin header, which the server checks. */
  async post(url, body = {}) {
    const res = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
      body: JSON.stringify(body),
      credentials: 'same-origin',
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
    return data;
  },
  async me() {
    const res = await fetch('/api/me', { credentials: 'same-origin' });
    return res.ok ? (await res.json()).owner === true : false;
  },
  async register({ setupToken, label } = {}) {
    const start = await this.post('/auth/register/start', setupToken ? { setup_token: setupToken } : {});
    const pk = structuredClone(start.options.publicKey);
    pk.challenge = b64u.decode(pk.challenge);
    pk.user.id = b64u.decode(pk.user.id);
    (pk.excludeCredentials || []).forEach(c => { c.id = b64u.decode(c.id); });
    const cred = await navigator.credentials.create({ publicKey: pk });
    const credential = {
      id: cred.id,
      rawId: b64u.encode(cred.rawId),
      type: cred.type,
      extensions: cred.getClientExtensionResults(),
      response: {
        attestationObject: b64u.encode(cred.response.attestationObject),
        clientDataJSON: b64u.encode(cred.response.clientDataJSON),
      },
    };
    return this.post('/auth/register/finish', { ceremony: start.ceremony, credential, label });
  },
  async login() {
    const start = await this.post('/auth/login/start');
    const pk = structuredClone(start.options.publicKey);
    pk.challenge = b64u.decode(pk.challenge);
    (pk.allowCredentials || []).forEach(c => { c.id = b64u.decode(c.id); });
    const cred = await navigator.credentials.get({ publicKey: pk });
    const credential = {
      id: cred.id,
      rawId: b64u.encode(cred.rawId),
      type: cred.type,
      extensions: cred.getClientExtensionResults(),
      response: {
        authenticatorData: b64u.encode(cred.response.authenticatorData),
        clientDataJSON: b64u.encode(cred.response.clientDataJSON),
        signature: b64u.encode(cred.response.signature),
        userHandle: cred.response.userHandle ? b64u.encode(cred.response.userHandle) : null,
      },
    };
    return this.post('/auth/login/finish', { ceremony: start.ceremony, credential });
  },
  logout() { return this.post('/auth/logout'); },
  async passkeys() {
    const res = await fetch('/api/owner/passkeys', { credentials: 'same-origin' });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return res.json();
  },
  async revoke(id) {
    const res = await fetch(`/api/owner/passkeys/${encodeURIComponent(id)}`, { method: 'DELETE', credentials: 'same-origin' });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
  },
};

/* Dates: "M/D/YY", or "Month D, YYYY". Input: RFC 3339. */
const fmtDate = (iso, style) => {
  if (!iso) return '';
  const d = new Date(iso);
  if (style === 'us') return `${d.getMonth() + 1}/${d.getDate()}/${String(d.getFullYear()).slice(2)}`;
  return d.toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });
};

/* Topic slugs → names (same list as the server, spec 4.2). */
const TOPICS = {
  ethereum: 'Ethereum', rust: 'Rust', surf: 'Surf',
  snowboarding: 'Snowboarding', 'jiu-jitsu': 'Jiu jitsu', 'classic-wow': 'Classic WoW',
};

/* A post is "new" for 14 days after it is published. */
const isNew = iso => iso && Date.now() - new Date(iso).getTime() < 14 * 864e5;

/* Path → view. */
const ROUTES = [
  [/^\/$/, 'home'],
  [/^\/topics\/([a-z-]+)$/, 'home'],
  [/^\/posts\/([a-z0-9-]+)$/, 'post'],
  [/^\/about$/, 'about'],
  [/^\/setup$/, 'setup'],
  [/^\/write(?:\/(\d+))?$/, 'write'],
];

const matchRoute = path => {
  for (const [re, view] of ROUTES) {
    const m = path.match(re);
    if (m) return { view, param: m[1] || null, path };
  }
  return { view: 'missing', param: null, path };
};

/* <blog-app>: the path router (History API).
   - Same-origin <a href="/..."> clicks change the view without a page load.
   - [data-goto] buttons go to a path. data-goto="latest" opens the newest post.
   - [data-nav="<view>"] elements get aria-current when that view shows.
   Fires "viewchange" with { view, param, path }. */
customElements.define('blog-app', class extends HTMLElement {
  connectedCallback() {
    this.addEventListener('click', e => {
      if (e.defaultPrevented || e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
      const goto = e.target.closest('[data-goto]');
      if (goto) { e.preventDefault(); this.go(goto.dataset.goto); return; }
      const a = e.target.closest('a[href]');
      if (!a || a.target || a.hasAttribute('download')) return;
      const url = new URL(a.href, location.href);
      if (url.origin !== location.origin || url.pathname.startsWith('/static/') || url.pathname.startsWith('/api/')) return;
      e.preventDefault();
      this.go(url.pathname);
    });
    window.addEventListener('popstate', () => this.render(false));
    this.render(false);
  }
  async go(path, { replace = false } = {}) {
    if (path === 'latest') {
      const posts = await Api.posts().catch(() => null);
      path = posts && posts.length ? `/posts/${posts[0].slug}` : '/';
    }
    if (path !== location.pathname) history[replace ? 'replaceState' : 'pushState'](null, '', path);
    this.render(true);
  }
  render(scroll) {
    const route = matchRoute(location.pathname);
    this.querySelectorAll('[data-view]').forEach(s => { s.hidden = s.dataset.view !== route.view; });
    this.querySelectorAll('[data-nav]').forEach(b => b.toggleAttribute('aria-current', b.dataset.nav === route.view));
    this.dataset.current = route.view;
    this.route = route;
    this.dispatchEvent(new CustomEvent('viewchange', { detail: route }));
    if (scroll) window.scrollTo({ top: 0 });
  }
});

/* <post-list>: the post table. Reloads on "viewchange" to the home view.
   Fills its <template> per post with textContent only.
   Template fields: [data-f="date|title|summary|topic|minutes"], [data-new]. */
customElements.define('post-list', class extends HTMLElement {
  connectedCallback() {
    this.tpl = this.querySelector('template');
    this.target = document.getElementById(this.getAttribute('target')) || this;
    this.titleEl = document.getElementById(this.getAttribute('title-target'));
    this.closest('blog-app')?.addEventListener('viewchange', e => {
      if (e.detail.view === 'home') this.load(e.detail.path.startsWith('/topics/') ? e.detail.param : null);
    });
    const app = this.closest('blog-app');
    if (app?.route?.view === 'home') this.load(app.route.path.startsWith('/topics/') ? app.route.param : null);
  }
  async load(topic) {
    const token = (this._token = Symbol());
    let posts;
    try {
      posts = topic ? await Api.topic(topic) : await Api.posts();
    } catch {
      posts = undefined;
    }
    if (token !== this._token) return;
    if (this.titleEl) this.titleEl.textContent = topic && TOPICS[topic] ? `Writing: ${TOPICS[topic]}` : 'Writing';
    this.target.replaceChildren();
    if (!posts || !posts.length) {
      const row = document.createElement('tr');
      const td = document.createElement('td');
      td.colSpan = 4;
      td.className = 'empty';
      td.textContent = posts === undefined ? 'Could not load the posts. Reload the page to try again.'
        : posts === null ? 'This topic does not exist.' : 'No posts yet.';
      row.append(td);
      this.target.append(row);
      return;
    }
    for (const p of posts) {
      const n = this.tpl.content.cloneNode(true);
      const set = (f, v) => n.querySelectorAll(`[data-f="${f}"]`).forEach(el => { el.textContent = v; });
      set('date', fmtDate(p.published_at, 'us'));
      set('title', p.title);
      set('summary', p.summary);
      set('topic', p.topic_name);
      set('minutes', `${p.reading_minutes} min`);
      n.querySelectorAll('a[data-f="title"]').forEach(a => { a.href = `/posts/${p.slug}`; });
      n.querySelectorAll('[data-new]').forEach(el => { el.hidden = !isNew(p.published_at); });
      this.target.append(n);
    }
  }
});

/* <post-view>: one post. Loads on "viewchange" to the post view.
   Fields: [data-f="title|byline|topic|body"]. Fires "postloaded" with the post (or null). */
customElements.define('post-view', class extends HTMLElement {
  connectedCallback() {
    this.closest('blog-app')?.addEventListener('viewchange', e => {
      if (e.detail.view === 'post') this.load(e.detail.param);
    });
    const app = this.closest('blog-app');
    if (app?.route?.view === 'post') this.load(app.route.param);
  }
  f(name) { return this.querySelector(`[data-f="${name}"]`); }
  async load(slug) {
    const token = (this._token = Symbol());
    this.setAttribute('aria-busy', 'true');
    let post;
    try {
      post = await Api.post(slug);
    } catch {
      post = undefined;
    }
    if (token !== this._token) return;
    this.removeAttribute('aria-busy');
    const topic = this.f('topic');
    if (!post) {
      this.f('title').textContent = post === null ? 'Not found' : 'Could not load this post';
      this.f('byline').textContent = '';
      topic.textContent = '';
      topic.removeAttribute('href');
      const p = document.createElement('p');
      p.textContent = post === null ? 'There is no public post at this address.' : 'Reload the page to try again.';
      this.f('body').replaceChildren(p);
    } else {
      this.f('title').textContent = post.title;
      this.f('byline').textContent = `${fmtDate(post.published_at)} · ${post.reading_minutes} min read`;
      topic.textContent = post.topic_name;
      topic.href = `/topics/${post.topic}`;
      // body_html is rendered and sanitized on the server (spec 6.7).
      this.f('body').innerHTML = post.body_html;
    }
    this.dispatchEvent(new CustomEvent('postloaded', { bubbles: true, detail: post || null }));
  }
});

/* <md-editor>: markdown on one side, live preview on the other.
   Parts: textarea, [data-preview], [data-words], [data-gutter], [data-md] buttons,
   [data-vis] buttons, [data-publish], [data-save], [data-toast]. */
customElements.define('md-editor', class extends HTMLElement {
  connectedCallback() {
    const $ = s => this.querySelector(s);
    const ta = $('textarea'), pv = $('[data-preview]'), gutter = $('[data-gutter]');
    if (!ta.value.trim()) ta.value = DRAFT_MD;
    this.dataset.visibility ||= 'private';

    const render = () => {
      // Sample stand-in. Step 5: the WASM render(), which sanitizes.
      pv.innerHTML = md(ta.value);
      const w = (ta.value.match(/\S+/g) || []).length;
      this.querySelectorAll('[data-words]').forEach(el => {
        el.textContent = (el.dataset.words || '{w} words · {m} min read')
          .replace('{w}', w).replace('{m}', Math.max(1, Math.round(w / 220)));
      });
      if (gutter) gutter.textContent = ta.value.split('\n').map((_, i) => i + 1).join('\n');
      this.querySelectorAll('[data-dirty]').forEach(el => { el.hidden = !this._dirty; });
    };
    ta.addEventListener('input', () => { this._dirty = true; render(); });
    ta.addEventListener('scroll', () => {
      const r = ta.scrollTop / Math.max(1, ta.scrollHeight - ta.clientHeight);
      pv.scrollTop = r * (pv.scrollHeight - pv.clientHeight);
      if (gutter) gutter.scrollTop = ta.scrollTop;
    });
    ta.addEventListener('keydown', e => {
      if (e.key === 'Tab' && !e.shiftKey) { e.preventDefault(); ta.setRangeText('    ', ta.selectionStart, ta.selectionEnd, 'end'); ta.dispatchEvent(new Event('input')); }
    });

    this.querySelectorAll('[data-md]').forEach(b => b.addEventListener('click', () => {
      const mark = b.dataset.md, s = ta.selectionStart, e = ta.selectionEnd, sel = ta.value.slice(s, e);
      if (mark.endsWith(' ')) {
        const ls = ta.value.lastIndexOf('\n', s - 1) + 1;
        ta.setRangeText(mark, ls, ls, 'end');
      } else if (mark === 'link') {
        ta.setRangeText(`[${sel || 'link text'}](https://)`, s, e, 'end');
      } else {
        ta.setRangeText(mark + (sel || 'text') + mark, s, e, 'end');
      }
      ta.focus();
      ta.dispatchEvent(new Event('input'));
    }));

    const syncVis = () => this.querySelectorAll('[data-vis]').forEach(x => {
      const on = x.dataset.vis === this.dataset.visibility;
      x.setAttribute('aria-pressed', on);
      if ('checked' in x) x.checked = on;
    });
    this.querySelectorAll('[data-vis]').forEach(b => b.addEventListener('click', () => {
      this.dataset.visibility = b.dataset.vis;
      syncVis();
    }));
    syncVis();

    const toast = $('[data-toast]');
    const say = key => {
      if (!toast) return;
      toast.textContent = toast.dataset[key] || key;
      toast.hidden = false;
      toast.classList.remove('pop'); void toast.offsetWidth; toast.classList.add('pop');
      clearTimeout(this._t);
      this._t = setTimeout(() => { toast.hidden = true; }, 2800);
    };
    $('[data-publish]')?.addEventListener('click', () => {
      this._dirty = false; render();
      say(this.dataset.visibility === 'public' ? 'public' : 'private');
    });
    $('[data-save]')?.addEventListener('click', () => { this._dirty = false; render(); say('saved'); });
    render();
  }
});
