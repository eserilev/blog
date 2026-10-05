/* Native web components for the Logbook (spec 5).
   Rule: only sanitized HTML goes into innerHTML: body_html from the server, or
   the WASM render() output in the editor (same code, same allow-list). Everything
   else uses textContent. */

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

/* Topic slugs → names (spec 4.2). The server writes the topic links into the page;
   setTopics() changes them after the owner edits the topics. */
const TOPICS = {};
const readTopics = () => {
  for (const k of Object.keys(TOPICS)) delete TOPICS[k];
  document.querySelectorAll('#topic-links a').forEach(a => { TOPICS[a.pathname.split('/').pop()] = a.textContent; });
};
readTopics();
/* Replaces the sidebar links and the editor's topic list. Text only. */
const setTopics = list => {
  const links = document.getElementById('topic-links');
  links.replaceChildren(...list.map(t => {
    const li = document.createElement('li');
    const a = document.createElement('a');
    a.href = `/topics/${t.slug}`;
    a.textContent = t.name;
    li.append(a);
    return li;
  }));
  const select = document.getElementById('ed-topic');
  const current = select.value;
  select.replaceChildren(...list.map(t => new Option(t.name, t.slug)));
  if (list.some(t => t.slug === current)) select.value = current;
  readTopics();
};

/* A post is "new" for 14 days after it is published. */
const isNew = iso => iso && Date.now() - new Date(iso).getTime() < 14 * 864e5;

/* Path → view. */
const ROUTES = [
  [/^\/$/, 'home'],
  [/^\/topics\/([a-z0-9-]+)$/, 'home'],
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
      // Only page paths. Files, the feed, and the API load normally.
      if (url.origin !== location.origin || url.pathname.startsWith('/static/') || url.pathname.startsWith('/api/') || url.pathname.startsWith('/media/') || url.pathname.endsWith('.xml')) return;
      e.preventDefault();
      this.go(url.pathname + url.hash);
    });
    window.addEventListener('popstate', () => this.render(false));
    this.render(false);
  }
  async go(path, { replace = false } = {}) {
    if (path === 'latest') {
      const posts = await Api.posts().catch(() => null);
      path = posts && posts.length ? `/posts/${posts[0].slug}` : '/';
    }
    if (path !== location.pathname + location.hash) history[replace ? 'replaceState' : 'pushState'](null, '', path);
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

/* The editor preview (spec 6.7): the server's render() compiled to WebAssembly.
   Same code as the server, so the preview matches the published post. */
const Preview = {
  _ready: null,
  load() {
    this._ready ||= (async () => {
      // The server adds the version hash to this URL (spec 6.9).
      const url = document.querySelector('meta[name="asset-wasm"]')?.content || '/static/wasm/logbook_render.wasm';
      const res = await fetch(url);
      if (!res.ok) throw new Error(`preview: HTTP ${res.status}`);
      const { instance } = await WebAssembly.instantiateStreaming(res, {});
      return instance.exports;
    })();
    return this._ready;
  },
  /* Markdown → sanitized HTML, through the three WASM functions. */
  async render(md) {
    const { memory, buf_alloc, buf_free, render } = await this.load();
    const input = new TextEncoder().encode(md);
    const p = buf_alloc(input.length);
    new Uint8Array(memory.buffer, p, input.length).set(input);
    const r = render(p, input.length);
    buf_free(p, input.length);
    const hp = Number(r >> 32n), hl = Number(r & 0xffffffffn);
    const html = new TextDecoder().decode(new Uint8Array(memory.buffer, hp, hl));
    buf_free(hp, hl);
    return html;
  },
};

/* Owner post API. Writes send If-Match with the version the editor loaded. */
const Posts = {
  async call(method, url, { body, version } = {}) {
    const headers = { Accept: 'application/json' };
    if (body !== undefined) headers['Content-Type'] = 'application/json';
    if (version !== undefined) headers['If-Match'] = String(version);
    const res = await fetch(url, { method, headers, body: body === undefined ? undefined : JSON.stringify(body), credentials: 'same-origin' });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) { const e = new Error(data.error || `HTTP ${res.status}`); e.status = res.status; throw e; }
    return data;
  },
  list() { return this.call('GET', '/api/owner/posts'); },
  get(id) { return this.call('GET', `/api/owner/posts/${id}`); },
  create() { return this.call('POST', '/api/owner/posts', { body: {} }); },
  save(id, version, body) { return this.call('PUT', `/api/owner/posts/${id}`, { body, version }); },
  setState(id, version, state) { return this.call('POST', `/api/owner/posts/${id}/state`, { body: { state }, version }); },
  remove(id) { return this.call('DELETE', `/api/owner/posts/${id}`); },
};

/* <post-files>: the owner's post list in the Compose view. */
customElements.define('post-files', class extends HTMLElement {
  connectedCallback() {
    this.rows = this.querySelector('[data-rows]');
    this.querySelector('[data-new]').addEventListener('click', async () => {
      try {
        const p = await Posts.create();
        this.closest('blog-app').go(`/write/${p.id}`);
      } catch (e) {
        alertBox(this, `Could not create a post. ${e.message}`);
      }
    });
    const app = this.closest('blog-app');
    app.addEventListener('viewchange', e => { if (e.detail.view === 'write') this.load(e.detail.param); });
    app.addEventListener('postsaved', () => this.load(app.route?.param));
  }
  async load(current) {
    if (!this.closest('blog-app').hasAttribute('data-owner')) return;
    let posts;
    try { posts = await Posts.list(); } catch { return; }
    this.rows.replaceChildren(...posts.map(p => {
      const tr = document.createElement('tr');
      if (String(p.id) === String(current)) tr.setAttribute('aria-current', 'true');
      const a = document.createElement('a');
      a.href = `/write/${p.id}`;
      a.textContent = p.title;
      const td = document.createElement('td');
      td.append(a);
      const st = document.createElement('td');
      st.textContent = p.state;
      const ch = document.createElement('td');
      ch.textContent = fmtDate(p.updated_at, 'us');
      tr.append(td, st, ch);
      return tr;
    }));
  }
});

/* A message in the editor's Win98 message box. */
function alertBox(el, msg) {
  const box = el.closest('section')?.querySelector('[data-toast]');
  if (!box) return;
  box.textContent = msg;
  box.hidden = false;
  clearTimeout(box._t);
  box._t = setTimeout(() => { box.hidden = true; }, 3200);
}

/* <md-editor>: one post. Loads on "viewchange" to /write/{id}. Markdown left, WASM
   preview right. Save and Publish send the version; a 409 means another device
   saved first (spec 3.8). Fires "postsaved". */
customElements.define('md-editor', class extends HTMLElement {
  connectedCallback() {
    const $ = s => this.querySelector(s);
    this.ta = $('textarea');
    this.pv = $('[data-preview]');
    this.words = $('[data-words]');
    this.status = $('[data-status]');
    this.fields = [...this.querySelectorAll('[data-field]')];

    this.fields.forEach(f => f.addEventListener('input', () => this.dirty(true)));
    this.ta.addEventListener('input', () => this.schedule());
    this.ta.addEventListener('scroll', () => {
      const r = this.ta.scrollTop / Math.max(1, this.ta.scrollHeight - this.ta.clientHeight);
      this.pv.scrollTop = r * (this.pv.scrollHeight - this.pv.clientHeight);
    });
    this.ta.addEventListener('keydown', e => {
      if (e.key === 'Tab' && !e.shiftKey) { e.preventDefault(); this.ta.setRangeText('    ', this.ta.selectionStart, this.ta.selectionEnd, 'end'); this.ta.dispatchEvent(new Event('input')); }
    });
    this.addEventListener('keydown', e => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') { e.preventDefault(); this.save(); }
    });
    this.querySelectorAll('[data-md]').forEach(b => b.addEventListener('click', () => {
      const ta = this.ta, mark = b.dataset.md, s = ta.selectionStart, e = ta.selectionEnd, sel = ta.value.slice(s, e);
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
    // Images: the IMG button, or paste an image into the text.
    const file = $('[data-image-file]');
    $('[data-image]').addEventListener('click', () => file.click());
    file.addEventListener('change', () => { if (file.files[0]) this.upload(file.files[0]); file.value = ''; });
    this.ta.addEventListener('paste', e => {
      const img = [...(e.clipboardData?.files || [])].find(f => f.type.startsWith('image/'));
      if (img) { e.preventDefault(); this.upload(img); }
    });
    $('[data-save]').addEventListener('click', () => this.save());
    $('[data-publish]').addEventListener('click', () => this.publish());
    const del = $('[data-delete]');
    del.addEventListener('click', () => this.remove(del));
    window.addEventListener('beforeunload', e => { if (this._dirty) e.preventDefault(); });

    const app = this.closest('blog-app');
    app.addEventListener('viewchange', e => { if (e.detail.view === 'write') this.open(e.detail.param); });
    if (app.route?.view === 'write') this.open(app.route.param);
  }

  say(msg, error) { this.status.textContent = msg; this.status.classList.toggle('error', !!error); }

  dirty(on) {
    this._dirty = on;
    this.querySelectorAll('[data-dirty]').forEach(el => { el.hidden = !on; });
    if (on) this.say('');
  }

  schedule() {
    this.dirty(true);
    if (this._raf) return;
    this._raf = requestAnimationFrame(() => { this._raf = 0; this.renderPreview(); });
  }

  async renderPreview() {
    const md = this.ta.value;
    try {
      // The WASM output is sanitized by the same ammonia allow-list as the server.
      this.pv.innerHTML = await Preview.render(md);
      const w = (md.match(/\S+/g) || []).filter(t => /[\p{L}\p{N}]/u.test(t)).length;
      this.words.textContent = `Document: Done · ${w} words · ${Math.max(1, Math.ceil(w / 220))} min read`;
    } catch (e) {
      this.words.textContent = `Preview unavailable: ${e.message}`;
    }
  }

  async open(id) {
    this.post = null;
    this.hidden = !id;
    this.removeAttribute('data-loaded');
    if (!id || !this.closest('blog-app').hasAttribute('data-owner')) return;
    try {
      this.show(await Posts.get(id));
    } catch (e) {
      this.say(e.status === 404 ? 'This post does not exist.' : `Could not load the post. ${e.message}`, true);
    }
  }

  show(p) {
    this.post = p;
    const set = (name, v) => { const f = this.querySelector(`[data-field="${name}"]`); if (f) f.value = v; };
    set('title', p.title);
    set('summary', p.summary);
    set('topic', p.topic);
    set('tags', (p.tags || []).join(', '));
    set('body_md', p.body_md);
    this.querySelector('[data-file]').textContent = `${p.slug}.md`;
    const radio = this.querySelector(`input[name="ed-state"][value="${p.state}"]`);
    if (radio) radio.checked = true;
    this.setAttribute('data-loaded', '');
    this.dirty(false);
    this.renderPreview();
  }

  input() {
    const v = name => this.querySelector(`[data-field="${name}"]`).value;
    return {
      title: v('title'),
      summary: v('summary'),
      topic: v('topic'),
      tags: v('tags').split(',').map(t => t.trim().toLowerCase()).filter(Boolean),
      body_md: v('body_md'),
    };
  }

  failed(e, what) {
    if (e.status === 409) {
      alertBox(this, 'This post changed on another device. Reload to see the newer version.');
      this.say('Conflict: reload to see the newer version.', true);
    } else {
      this.say(`Could not ${what}. ${e.message}`, true);
    }
  }

  /* Uploads an image and inserts its markdown at the cursor (spec 6.8). */
  async upload(f) {
    this.say('Uploading image...');
    const form = new FormData();
    form.append('file', f);
    try {
      const res = await fetch('/api/owner/uploads', { method: 'POST', body: form, credentials: 'same-origin' });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) throw new Error(data.error || `HTTP ${res.status}`);
      this.ta.setRangeText(`\n${data.markdown}\n`, this.ta.selectionStart, this.ta.selectionEnd, 'end');
      this.ta.dispatchEvent(new Event('input'));
      this.say('Image added.');
    } catch (e) {
      this.say(`Could not upload the image. ${e.message}`, true);
    }
  }

  async save() {
    if (!this.post) return false;
    try {
      this.show(await Posts.save(this.post.id, this.post.version, this.input()));
      alertBox(this, 'Draft saved.');
      this.dispatchEvent(new CustomEvent('postsaved', { bubbles: true }));
      return true;
    } catch (e) {
      this.failed(e, 'save');
      return false;
    }
  }

  async publish() {
    if (!this.post) return;
    if (this._dirty && !(await this.save())) return;
    const state = this.querySelector('input[name="ed-state"]:checked')?.value || 'draft';
    try {
      this.show(await Posts.setState(this.post.id, this.post.version, state));
      alertBox(this, {
        public: 'Post published. Anyone can read it now.',
        private: 'Saved as private. Only you can see this post.',
        draft: 'Saved as a draft.',
      }[state]);
      this.dispatchEvent(new CustomEvent('postsaved', { bubbles: true }));
    } catch (e) {
      this.failed(e, 'change the state');
    }
  }

  /* Two clicks: the first arms the button, the second deletes. */
  async remove(btn) {
    if (!this.post) return;
    if (!btn.dataset.armed) {
      btn.dataset.armed = '1';
      btn.textContent = 'Click again to delete';
      setTimeout(() => { delete btn.dataset.armed; btn.textContent = 'Delete'; }, 4000);
      return;
    }
    delete btn.dataset.armed;
    btn.textContent = 'Delete';
    try {
      await Posts.remove(this.post.id);
      this.dirty(false);
      this.dispatchEvent(new CustomEvent('postsaved', { bubbles: true }));
      this.closest('blog-app').go('/write');
    } catch (e) {
      this.failed(e, 'delete');
    }
  }
});

/* <surf-report>: NOAA data from /api/surf (spec 4.5). Readouts older than 3 h are
   marked stale. The tide curve is a cosine between the predicted highs and lows. */
customElements.define('surf-report', class extends HTMLElement {
  connectedCallback() {
    this.load();
    setInterval(() => this.load(), 10 * 60 * 1000);
  }
  async load() {
    let data;
    try { data = await (await fetch('/api/surf')).json(); } catch { data = { available: false }; }
    const rating = this.querySelector('[data-rating]');
    if (!data.available) { rating.textContent = 'No NOAA data yet.'; return; }
    const s = data.surf;
    const stale = iso => !iso || Date.now() - new Date(iso).getTime() > 3 * 3600e3;
    const set = (name, value, iso, unit) => {
      const row = this.querySelector(`[data-r="${name}"]`);
      row.querySelector('[data-v]').textContent = value ?? '--';
      if (unit !== undefined) row.querySelector('[data-u]').textContent = unit;
      row.classList.toggle('stale', value != null && stale(iso));
    };
    set('swell', s.swell ? s.swell.height_ft.toFixed(1) : null, s.swell?.observed_at);
    set('period', s.swell ? Math.round(s.swell.period_s) : null, s.swell?.observed_at);
    set('wind', s.wind ? Math.round(s.wind.speed_kt) : null, s.wind?.observed_at, s.wind ? `kt ${s.wind.direction}${s.wind.offshore ? ' off' : ''}` : 'kt');
    set('water', s.waves?.water_c != null ? Math.round(s.waves.water_c * 9 / 5 + 32) : null, s.waves?.observed_at);
    rating.textContent = s.rating || '';
    this.drawTides(s.tides || []);
  }
  drawTides(tides) {
    const svg = this.querySelector('[data-tide]');
    const legend = this.querySelector('[data-tide-legend]');
    const NS = 'http://www.w3.org/2000/svg';
    svg.replaceChildren();
    if (tides.length < 2) return;
    // NOAA local times ("YYYY-MM-DD HH:MM") as minutes; plot the next 24 h.
    const t = tides.map(p => ({ ...p, m: new Date(p.time.replace(' ', 'T')).getTime() / 6e4 }));
    const now = Date.now() / 6e4, end = now + 24 * 60;
    const hs = t.map(p => p.height_ft), lo = Math.min(...hs) - 0.5, hi = Math.max(...hs) + 0.5;
    const x = m => ((m - now) / (end - now)) * 240, y = h => 52 - ((h - lo) / (hi - lo)) * 48;
    const pts = [];
    for (let m = now; m <= end; m += 10) {
      const i = t.findIndex(p => p.m > m);
      if (i <= 0) continue;
      const a = t[i - 1], b = t[i], f = (m - a.m) / (b.m - a.m);
      const h = a.height_ft + (b.height_ft - a.height_ft) * (1 - Math.cos(Math.PI * f)) / 2;
      pts.push(`${x(m).toFixed(1)},${y(h).toFixed(1)}`);
    }
    const path = document.createElementNS(NS, 'polyline');
    path.setAttribute('points', pts.join(' '));
    path.setAttribute('fill', 'none');
    path.setAttribute('stroke', '#000060');
    path.setAttribute('stroke-width', '2');
    svg.append(path);
    const next = t.filter(p => p.m > now);
    const label = p => p ? `${p.kind === 'H' ? 'High' : 'Low'} ${new Date(p.m * 6e4).toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })} · ${p.height_ft.toFixed(1)} ft` : '';
    legend.children[0].textContent = label(next[0]);
    legend.children[1].textContent = label(next[1]);
    svg.setAttribute('aria-label', `Tide chart, approximate: ${label(next[0])}, then ${label(next[1])}`);
    for (const p of next.slice(0, 4)) {
      if (p.m > end) break;
      const c = document.createElementNS(NS, 'circle');
      c.setAttribute('cx', x(p.m).toFixed(1));
      c.setAttribute('cy', y(p.height_ft).toFixed(1));
      c.setAttribute('r', '3');
      c.setAttribute('fill', '#ffffff');
      c.setAttribute('stroke', '#000060');
      c.setAttribute('stroke-width', '2');
      svg.append(c);
    }
  }
});
