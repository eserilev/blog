/* Shared blog runtime: sample data, a tiny markdown renderer, and native web components.
   Every design uses the same components. Only the markup and CSS change. */

(() => {
  const st = document.createElement('style');
  st.textContent = 'blog-app, md-editor, post-list, md-render { display: block; }';
  document.head.prepend(st);
})();

const POSTS = [
  { id: 'bal', title: 'Block-level access lists and parallel execution', slug: 'block-level-access-lists.md',
    date: '2026-09-28', kind: 'eth', topic: 'Ethereum', tags: ['glamsterdam', 'eip-7928', 'clients'],
    mins: 9, words: 1980, vis: 'public', isNew: true,
    excerpt: 'EIP-7928 makes each block declare the accounts and storage slots it touches. What this gives clients, and what it costs builders.' },
  { id: 'ssz', title: 'Zero-copy SSZ decoding in Rust', slug: 'zero-copy-ssz.md',
    date: '2026-09-14', kind: 'rust', topic: 'Rust', tags: ['rust', 'ssz', 'performance'],
    mins: 12, words: 2640, vis: 'public', isNew: true,
    excerpt: 'Decoding beacon state without allocations: borrowing from the input buffer, the lifetime problems that follow, and where the change paid off.' },
  { id: 'ui', title: 'What classic WoW addons got right about interfaces', slug: 'classic-wow-interfaces.md',
    date: '2026-09-02', kind: 'wow', topic: 'Classic WoW', tags: ['design', 'wow'],
    mins: 7, words: 1530, vis: 'public',
    excerpt: 'Raid frames and threat meters solved information density problems that many dashboards still get wrong.' },
  { id: 'bjj', title: 'Four years of jiu jitsu', slug: 'four-years-of-jiu-jitsu.md',
    date: '2026-08-19', kind: 'bjj', topic: 'Jiu jitsu', tags: ['bjj'],
    mins: 5, words: 1120, vis: 'public',
    excerpt: 'Notes on training consistently while working on a protocol, and on what the mat teaches about hard problems.' },
  { id: 'surf', title: 'Surf log, summer 2026', slug: 'surf-log-summer-2026.md',
    date: '2026-07-30', kind: 'surf', topic: 'Surf', tags: ['surf'],
    mins: 3, words: 610, vis: 'public',
    excerpt: 'Conditions, a new board, and why I keep my mornings free of calls.' },
  { id: 'split', title: 'Splitboarding: end of season notes', slug: 'end-of-season.md',
    date: '2026-04-11', kind: 'snow', topic: 'Snowboarding', tags: ['snow', 'backcountry'],
    mins: 6, words: 1290, vis: 'public',
    excerpt: 'Route, gear, and conditions from the last backcountry tour of the season.' },
  { id: 'epbs', title: "ePBS from a client's perspective", slug: 'epbs-client-view.md',
    date: '2026-10-01', kind: 'eth', topic: 'Ethereum', tags: ['glamsterdam', 'epbs'],
    mins: 4, words: 412, vis: 'private', draft: true,
    excerpt: 'Draft. Fork choice now has to track the payload, not only the block.' },
];

const POST_MD = `An execution client receives a block today with no information about the state it touches. It must execute the transactions in order, because any transaction can depend on the result of the one before it.

EIP-7928 changes this. Each block carries a **block-level access list** (BAL): every account and storage slot that the block reads or writes, with the values after each transaction.

## What clients gain

- Clients can load state from disk in parallel, before execution starts.
- Transactions with separate access sets can execute in parallel.
- Some sync paths can apply state changes without full re-execution.

## What it costs

The list adds data to every block. Builders must produce it, and clients must make sure that it matches the result of execution. A block with a wrong list is invalid.

## A rough shape

This is not the spec type. It is a simplified version for discussion:

\`\`\`rust
pub struct AccountChanges {
    pub address: Address,
    pub storage_writes: Vec<SlotWrites>,
    pub storage_reads: Vec<B256>,
    pub balance_changes: Vec<(u16, U256)>,
}
\`\`\`

## Why it matters

Block validation time is one of the main limits on the gas limit. If clients validate blocks faster, the network can raise the limit safely.`;

const DRAFT_MD = `# ePBS from a client's perspective

Glamsterdam splits the block in two. The proposer commits to a builder bid. The builder reveals the payload later.

## What changes for fork choice

- Fork choice now tracks **payload presence**, not only the block.
- The payload timeliness committee (PTC) votes on whether the payload arrived on time.
- An *empty* slot is now different from a *missed* slot.

## First pass at the type

\`\`\`rust
pub enum PayloadStatus {
    Pending,
    Revealed { block_hash: Hash256 },
    Withheld,
}
\`\`\`

> TODO: add the fork choice diagram before publishing.`;

/* Tiny markdown renderer. It covers headings, paragraphs, lists, quotes, fences, rules,
   bold, italic, inline code, and links. A real build swaps in a full parser. */
function md(src) {
  const esc = s => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  const inline = s => esc(s)
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
    .replace(/(^|[^*])\*([^*\s][^*]*)\*/g, '$1<em>$2</em>')
    .replace(/\[([^\]]+)\]\((https?:[^)\s]+|#[^)\s]*)\)/g, '<a href="$2">$1</a>');
  const block = /^(#{1,4}\s|```|>|[-*]\s|\d+\.\s|---+\s*$)/;
  const lines = src.replace(/\r/g, '').split('\n');
  const out = [];
  let i = 0;
  while (i < lines.length) {
    const l = lines[i];
    if (l.startsWith('```')) {
      const lang = l.slice(3).trim(), buf = [];
      i++;
      while (i < lines.length && !lines[i].startsWith('```')) buf.push(lines[i++]);
      i++;
      out.push(`<pre data-lang="${esc(lang)}"><code>${esc(buf.join('\n'))}</code></pre>`);
      continue;
    }
    const h = l.match(/^(#{1,4})\s+(.*)/);
    if (h) { out.push(`<h${h[1].length}>${inline(h[2])}</h${h[1].length}>`); i++; continue; }
    if (/^>/.test(l)) {
      const buf = [];
      while (i < lines.length && /^>/.test(lines[i])) buf.push(lines[i++].replace(/^>\s?/, ''));
      out.push(`<blockquote><p>${inline(buf.join(' '))}</p></blockquote>`);
      continue;
    }
    const list = /^[-*]\s+/.test(l) ? ['ul', /^[-*]\s+/] : /^\d+\.\s+/.test(l) ? ['ol', /^\d+\.\s+/] : null;
    if (list) {
      const buf = [];
      while (i < lines.length && list[1].test(lines[i])) buf.push(`<li>${inline(lines[i++].replace(list[1], ''))}</li>`);
      out.push(`<${list[0]}>${buf.join('')}</${list[0]}>`);
      continue;
    }
    if (/^---+\s*$/.test(l)) { out.push('<hr>'); i++; continue; }
    if (!l.trim()) { i++; continue; }
    const buf = [];
    while (i < lines.length && lines[i].trim() && !block.test(lines[i])) buf.push(lines[i++]);
    out.push(`<p>${inline(buf.join(' '))}</p>`);
  }
  return out.join('\n');
}

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
        else if (f === 'tags') el.innerHTML = p.tags.map(t => `<span class="tag">${t}</span>`).join(el.dataset.sep || ' ');
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
