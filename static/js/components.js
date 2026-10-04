/* Native web components for the Logbook (spec 5).
   Rule: only server-rendered body_html goes into innerHTML. Until steps 2 and 5,
   the sample md() from sample.js stands in for it. Its output is escaped. */

const fmtDate = (iso, style) => {
  const d = new Date(iso + 'T12:00:00');
  if (style === 'short') return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  if (style === 'us') return `${d.getMonth() + 1}/${d.getDate()}/${String(d.getFullYear()).slice(2)}`;
  return d.toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });
};

/* <blog-app>: switches between [data-view] sections. Any [data-goto] element changes the view. */
customElements.define('blog-app', class extends HTMLElement {
  connectedCallback() {
    this.addEventListener('click', e => {
      const t = e.target.closest('[data-goto]');
      if (!t) return;
      e.preventDefault();
      if (t.dataset.post) this.dataset.post = t.dataset.post;
      this.show(t.dataset.goto);
    });
    const start = (location.hash || '').slice(1);
    this.show(this.querySelector(`[data-view="${start}"]`) ? start : 'home', true);
  }
  show(view, quiet) {
    this.querySelectorAll('[data-view]').forEach(s => { s.hidden = s.dataset.view !== view; });
    this.querySelectorAll('[data-goto]').forEach(b => {
      if (b.dataset.nav !== undefined) b.toggleAttribute('aria-current', b.dataset.goto === view);
    });
    this.dataset.current = view;
    this.dispatchEvent(new CustomEvent('viewchange', { detail: view }));
    if (!quiet) window.scrollTo({ top: 0, behavior: 'instant' in window ? 'instant' : 'auto' });
  }
});

/* <post-list>: stamps its <template> once per post. Fields: [data-f], classes: [data-cls]. */
customElements.define('post-list', class extends HTMLElement {
  connectedCallback() {
    if (this._done) return;
    this._done = true;
    const tpl = this.querySelector('template');
    const drafts = this.hasAttribute('drafts');
    const limit = +this.getAttribute('limit') || Infinity;
    const skip = +this.getAttribute('skip') || 0;
    const style = this.getAttribute('date-style');
    const target = document.getElementById(this.getAttribute('target')) || this;
    POSTS.filter(p => drafts || !p.draft).slice(skip, skip + limit).forEach((p, idx) => {
      const n = tpl.content.cloneNode(true);
      n.querySelectorAll('[data-f]').forEach(el => {
        const f = el.dataset.f;
        if (f === 'date') el.textContent = fmtDate(p.date, el.dataset.style || style);
        else if (f === 'tags') el.replaceChildren(...p.tags.flatMap((t, k) => {
          const span = document.createElement('span');
          span.className = 'tag';
          span.textContent = t;
          return k ? [document.createTextNode(el.dataset.sep || ' '), span] : [span];
        }));
        else if (f === 'index') el.textContent = String(idx + 1 + skip).padStart(2, '0');
        else el.textContent = p[f] ?? '';
      });
      n.querySelectorAll('[data-cls]').forEach(el => el.dataset.cls.split(' ').forEach(k => {
        if (k === 'new') { if (p.isNew) el.classList.add('is-new'); }
        else el.classList.add(`${k}-${p[k]}`);
      }));
      n.querySelectorAll('[data-goto]').forEach(el => { el.dataset.post = p.id; });
      n.firstElementChild && (n.firstElementChild.dataset.id = p.id);
      target.appendChild(n);
    });
  }
});

/* <md-render>: renders the sample post, or its own <template> text, as HTML. */
customElements.define('md-render', class extends HTMLElement {
  connectedCallback() {
    const t = this.querySelector('template');
    // Sample stand-in. Step 2: body_html from the API.
    this.innerHTML = md(t ? t.innerHTML : POST_MD);
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
