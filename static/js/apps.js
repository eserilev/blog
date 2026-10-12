/* Win98 apps (spec 11.9): the desktop icons, the app windows over the Logbook window,
   Documents, the Recycle Bin, and Notepad. Paint loads on demand (paint.js, paint.css,
   and its WASM core) the first time it opens.

   Rules: all text goes in with textContent. Positions and sizes go through CSSOM (CSP).
   Nothing here changes a post: the Recycle Bin only lists the things that go in it.
   The apps exist only in the Win98 view: CSS hides them in the other modes, and the
   keys do nothing there. Paint uses the helpers of the Win98 object. */
const Win98 = (() => {
  const app = document.querySelector('blog-app');
  const layer = app.querySelector('[data-apps]');
  const modal = layer.querySelector('[data-modal]');
  const icons = layer.querySelector('template[data-icons]').content;
  const $ = (s, r = document) => r.querySelector(s);
  const general = () => Mode.current === 'general';
  const narrow = () => matchMedia('(max-width: 620px)').matches;
  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));

  /* A new element. props: class, text, on<event>, or an attribute. */
  const h = (tag, props = {}, ...kids) => {
    const e = document.createElement(tag);
    for (const [k, v] of Object.entries(props)) {
      if (v == null || v === false) continue;
      if (k === 'class') e.className = v;
      else if (k === 'text') e.textContent = v;
      else if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
      else e.setAttribute(k, v === true ? '' : v);
    }
    e.append(...kids.flat().filter(c => c != null && c !== false));
    return e;
  };
  /* An inline SVG from a list of shapes: [tag, attributes, text]. */
  const SVG_NS = 'http://www.w3.org/2000/svg';
  const svg = (viewBox, cls, shapes) => {
    const s = document.createElementNS(SVG_NS, 'svg');
    s.setAttribute('viewBox', viewBox);
    s.setAttribute('aria-hidden', 'true');
    if (cls) s.setAttribute('class', cls);
    for (const [tag, attrs, text] of shapes) {
      const el = document.createElementNS(SVG_NS, tag);
      for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
      if (text) el.textContent = text;
      s.append(el);
    }
    return s;
  };
  /* A copy of an icon from the page template. */
  const icon = name => icons.querySelector(`[data-i="${name}"]`).cloneNode(true);

  // ---------- Dialogs ----------
  const OKBTN = [{ label: 'OK', value: 'ok', def: true }];
  const OKCANCEL = [{ label: 'OK', value: 'ok', def: true }, { label: 'Cancel', value: 'cancel' }];
  const YNC = [{ label: 'Yes', value: 'yes', def: true }, { label: 'No', value: 'no' }, { label: 'Cancel', value: 'cancel' }];
  /* A modal Win98 dialog. body: a string, a list of lines, or an element.
     Resolves { value, el }. A button's run(el) returns false to keep the dialog open.
     Escape cancels, Enter clicks the default button, and Tab stays in the dialog. */
  const dialog = ({ title, body, buttons, width, icon: ic }) => new Promise(resolve => {
    const prev = document.activeElement;
    const x = h('button', { class: 'ctl raised', type: 'button', 'aria-label': 'Close', text: '×' });
    const main = h('div', { class: 'dlg-main' }, ic ? icon(ic) : null);
    const b = body instanceof Element ? body : h('div', {}, [].concat(body).map(t => h('p', { text: t })));
    b.classList.add('dlg-body');
    main.append(b);
    const btns = h('div', { class: 'dlg-btns' });
    const d = h('div', { class: 'dlg raised', role: 'dialog', 'aria-modal': 'true', 'aria-label': title },
      h('div', { class: 'titlebar' }, h('span', { class: 'ttl', text: title }), x), main, btns);
    if (width) d.style.width = `min(${width}px, 100%)`;
    const key = e => {
      if (modal.lastElementChild !== d || !general()) return;
      if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); done('cancel'); }
      else if (e.key === 'Enter' && !/TEXTAREA|BUTTON/.test(e.target.tagName)) { e.preventDefault(); e.stopPropagation(); btns.querySelector('.default')?.click(); }
      else if (e.key === 'Tab') {
        const f = [...d.querySelectorAll('button, input, select, textarea')].filter(el => !el.disabled && el.getClientRects().length);
        const i = f.indexOf(document.activeElement);
        e.preventDefault();
        f.at(e.shiftKey ? (i <= 0 ? -1 : i - 1) : (i + 1) % f.length)?.focus();
      }
    };
    const done = v => {
      d.remove();
      document.removeEventListener('keydown', key, true);
      if (!modal.children.length) modal.hidden = true;
      prev?.focus?.();
      resolve({ value: v, el: d });
    };
    for (const bt of buttons) {
      btns.append(h('button', {
        class: bt.def ? 'btn98 default' : 'btn98', type: 'button', text: bt.label,
        onclick: async () => { if (bt.run && (await bt.run(d)) === false) return; done(bt.value); },
      }));
    }
    x.addEventListener('click', () => done('cancel'));
    document.addEventListener('keydown', key, true);
    modal.append(d);
    modal.hidden = false;
    (d.querySelector('input:not([type=radio]):not([type=checkbox]):not([type=color]), select') || btns.querySelector('.default') || btns.firstChild)?.focus();
  });
  const tell = (title, body, ic = 'info') => dialog({ title, body, icon: ic, buttons: OKBTN });

  /* Save As: a file name and a type. Resolves { name, type } or null. */
  const saveAsDialog = async ({ name, type, types }) => {
    const nameIn = h('input', { class: 'in98', id: 'sa-name', autocomplete: 'off', spellcheck: 'false' });
    const typeIn = h('select', { class: 'in98', id: 'sa-type' }, types.map(([v, l]) => h('option', { value: v, text: l })));
    nameIn.value = name;
    typeIn.value = type;
    const body = h('div', {},
      h('div', { class: 'frow' }, h('label', { for: 'sa-name', text: 'File name:' }), nameIn),
      h('div', { class: 'frow' }, h('label', { for: 'sa-type', text: 'Save as type:' }), typeIn));
    const r = await dialog({ title: 'Save As', body, width: 400, buttons: [
      { label: 'Save', value: 'ok', def: true, run: async () => {
        const v = nameIn.value.trim();
        if (!v || /[\\/:*?"<>|]/.test(v)) {
          await tell('Save As', ['The file name is not valid.', 'Do not use these characters: \\ / : * ? " < > |'], 'warn');
          return false;
        }
      } },
      { label: 'Cancel', value: 'cancel' }] });
    if (r.value !== 'ok') return null;
    const exts = types.map(t => t[0]).concat(['jpeg', 'markdown']).join('|');
    return { name: nameIn.value.trim().replace(new RegExp(`\\.(${exts})$`, 'i'), ''), type: typeIn.value };
  };

  /* Saves a file on the visitor's computer: a normal download link. */
  const MIME = { png: 'image/png', jpg: 'image/jpeg', md: 'text/markdown', txt: 'text/plain' };
  const saveFile = (filename, data) => {
    const blob = data instanceof Blob ? data : new Blob([data], { type: MIME[filename.split('.').pop()] || 'text/plain' });
    const a = h('a', { href: URL.createObjectURL(blob), download: filename, hidden: true });
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(a.href), 5000);
    return true;
  };
  /* Asks for a file on the visitor's computer. Resolves the File, or null. */
  const pickFile = accept => new Promise(resolve => {
    const i = h('input', { type: 'file', accept, hidden: true });
    i.addEventListener('change', () => { resolve(i.files[0] || null); i.remove(); });
    i.addEventListener('cancel', () => { resolve(null); i.remove(); });
    document.body.append(i);
    i.click();
  });

  // ---------- Pull-down menus ----------
  /* menus: { Name: () => [item | '-'] }. An item: { label, key, check, disabled, run }.
     The lists are made when a menu opens, so they show the current state. */
  const menubar = (bar, menus) => {
    let cur = null, drop = null;
    const close = () => { drop?.remove(); drop = null; cur?.classList.remove('open'); cur = null; };
    const btns = Object.keys(menus).map(name => {
      const b = h('button', { class: 'mb', type: 'button', 'aria-haspopup': 'menu' }, h('u', { text: name[0] }), name.slice(1));
      b.show = focus => {
        close();
        cur = b;
        b.classList.add('open');
        drop = h('div', { class: 'mdrop raised', role: 'menu', 'aria-label': name });
        for (const it of menus[name]()) {
          if (it === '-') { drop.append(h('div', { class: 'sep', role: 'separator' })); continue; }
          drop.append(h('button', {
            class: 'mi', type: 'button', tabindex: '-1', role: it.check === undefined ? 'menuitem' : 'menuitemcheckbox', 'aria-label': it.label,
            'aria-checked': it.check === undefined ? null : String(!!it.check), 'aria-disabled': it.disabled ? 'true' : null,
            onclick: () => { if (it.disabled) return; close(); it.run(); },
          }, h('span', { class: 'ck', text: it.check ? '✓' : '' }), h('span', { text: it.label }), h('span', { class: 'ky', text: it.key || '' })));
        }
        drop.style.left = `${b.offsetLeft}px`;
        drop.style.top = `${b.offsetTop + b.offsetHeight}px`;
        bar.append(drop);
        // A menu at the right edge moves left, so it stays on the screen.
        const over = drop.getBoundingClientRect().right - document.documentElement.clientWidth + 4;
        if (over > 0) drop.style.left = `${Math.max(0, b.offsetLeft - over)}px`;
        if (focus) drop.querySelector('.mi:not([aria-disabled])')?.focus();
        drop.addEventListener('keydown', e => {
          const items = [...drop.querySelectorAll('.mi:not([aria-disabled])')], i = items.indexOf(document.activeElement), n = items.length;
          if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); items[(i + (e.key === 'ArrowDown' ? 1 : n - 1) + n) % n]?.focus(); }
          else if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') { e.preventDefault(); const k = btns.indexOf(b); btns[(k + (e.key === 'ArrowRight' ? 1 : btns.length - 1)) % btns.length].show(true); }
          else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); close(); b.focus(); }
          else if (e.key === 'Tab') close();
        });
      };
      b.addEventListener('click', e => { if (cur === b) close(); else b.show(e.detail === 0); });
      b.addEventListener('pointerenter', () => { if (cur && cur !== b) b.show(false); });
      bar.append(b);
      return b;
    });
    document.addEventListener('pointerdown', e => { if (drop && !bar.contains(e.target)) close(); });
    document.addEventListener('modechange', close);
    return { close };
  };

  // ---------- Windows ----------
  /* id → { el, task, open, min, inFlow, off, canClose, onClose, onOpen, keys }.
     The Logbook window stays in the page flow. The app windows are fixed boxes. */
  const W = {};
  let stack = [], active = null;
  const isActive = id => active === id && W[id]?.open && !W[id].min && general();
  const tbH = () => document.querySelector('.taskbar')?.getBoundingClientRect().height || 34;
  const setTb = () => document.documentElement.style.setProperty('--tb-h', `${tbH()}px`);
  const focusWin = id => {
    if (id) {
      stack = stack.filter(k => k !== id);
      stack.push(id);
      stack.forEach((k, i) => { W[k].el.style.zIndex = String(10 + i); });
    }
    active = id;
    for (const k in W) {
      W[k].el.classList.toggle('inactive', k !== id);
      W[k].task?.setAttribute('aria-pressed', String(k === id && !W[k].min));
    }
  };
  const topVisible = () => [...stack].reverse().find(k => W[k].open && !W[k].min) || null;
  const logbookState = () => {
    const w = W.logbook;
    if (w.open && !w.min) app.removeAttribute('data-logbook');
    else app.dataset.logbook = w.open ? 'min' : 'closed';
  };
  /* A new window opens near the center of the screen, inside it. */
  const place = w => {
    const vw = document.documentElement.clientWidth, vh = window.innerHeight - tbH();
    const ow = w.el.offsetWidth, oh = w.el.offsetHeight;
    w.el.style.left = `${clamp(Math.round((vw - ow) / 2 + w.off[0]), 8, Math.max(8, vw - ow - 8))}px`;
    w.el.style.top = `${clamp(Math.round((vh - oh) / 2 + w.off[1]), 8, Math.max(8, vh - oh - 8))}px`;
  };
  const openWin = id => {
    const w = W[id];
    if (!w.open) {
      w.open = true;
      w.el.hidden = false;
      if (w.task) w.task.hidden = false;
      if (!w.inFlow) place(w);
      w.onOpen?.();
    }
    w.min = false;
    w.el.hidden = false;
    focusWin(id);
    if (id === 'logbook') logbookState();
    else if (!w.el.contains(document.activeElement)) (w.focusEl?.() || w.el).focus({ preventScroll: true });
  };
  const minWin = id => {
    W[id].min = true;
    W[id].el.hidden = true;
    focusWin(topVisible());
    if (id === 'logbook') logbookState();
  };
  const closeWin = async id => {
    const w = W[id];
    if (!w.open || (w.canClose && !(await w.canClose()))) return;
    w.open = false;
    w.min = false;
    w.el.hidden = true;
    if (w.task) w.task.hidden = true;
    setMax(id, false);
    w.onClose?.();
    focusWin(topVisible());
    if (id === 'logbook') logbookState();
  };
  const isMax = id => (W[id].inFlow ? W[id].el.parentElement : W[id].el).classList.contains('max');
  const setMax = (id, on) => {
    (W[id].inFlow ? W[id].el.parentElement : W[id].el).classList.toggle('max', on);
    const b = W[id].el.querySelector(':scope > .titlebar [data-max]');
    b.textContent = on ? '❐' : '□';
    b.setAttribute('aria-label', on ? 'Restore' : 'Maximize');
  };

  /* Adds a window: its buttons, its taskbar button, focus, and dragging by the title bar. */
  const register = (id, el, opts = {}) => {
    const task = document.querySelector(`.taskbar [data-task="${id}"]`);
    const w = W[id] = { el, task, open: false, min: false, off: [0, 0], ...opts };
    const bar = el.querySelector(':scope > .titlebar');
    el.addEventListener('pointerdown', () => { if (active !== id) focusWin(id); }, true);
    el.addEventListener('focusin', () => { if (active !== id) focusWin(id); });
    bar.querySelector('[data-min]').addEventListener('click', e => { e.stopPropagation(); minWin(id); });
    bar.querySelector('[data-max]').addEventListener('click', () => setMax(id, !isMax(id)));
    bar.querySelector('[data-close]').addEventListener('click', e => { e.stopPropagation(); closeWin(id); });
    bar.addEventListener('dblclick', e => { if (!e.target.closest('.ctl')) setMax(id, !isMax(id)); });
    task?.addEventListener('click', () => {
      if (!w.min && active === id) minWin(id);
      else openWin(id);
    });
    if (w.inFlow) return w;
    // Drag by the title bar. The window keeps 40 px on the screen.
    bar.addEventListener('pointerdown', e => {
      if (e.button !== 0 || e.target.closest('.ctl') || el.classList.contains('max') || narrow()) return;
      const x0 = e.clientX - el.offsetLeft, y0 = e.clientY - el.offsetTop;
      const vw = document.documentElement.clientWidth, vh = window.innerHeight - tbH();
      bar.setPointerCapture(e.pointerId);
      const move = ev => {
        el.style.left = `${clamp(ev.clientX - x0, 40 - el.offsetWidth, vw - 40)}px`;
        el.style.top = `${clamp(ev.clientY - y0, 0, vh - 30)}px`;
      };
      const up = () => { bar.removeEventListener('pointermove', move); bar.removeEventListener('pointerup', up); bar.removeEventListener('pointercancel', up); };
      bar.addEventListener('pointermove', move);
      bar.addEventListener('pointerup', up);
      bar.addEventListener('pointercancel', up);
    });
    return w;
  };

  // The Logbook window: the site itself.
  register('logbook', app.querySelector('[data-win="logbook"]'), { inFlow: true, open: true });
  focusWin('logbook');
  // A view change shows the Logbook window: Documents, the Logbook icon, Home, Latest.
  app.addEventListener('viewchange', () => { if (general()) openWin('logbook'); });
  // The apps exist only in the Win98 view. Another mode shows the Logbook window again.
  document.addEventListener('modechange', () => {
    if (general()) return;
    if (!W.logbook.open || W.logbook.min) { W.logbook.open = true; W.logbook.min = false; W.logbook.el.hidden = false; logbookState(); }
  });
  // The taskbar height changes when the fonts load and when the screen turns.
  setTb();
  new ResizeObserver(setTb).observe(document.querySelector('.taskbar'));

  // ---------- Desktop icons, drag and drop, Recycle Bin ----------
  const binIcon = layer.querySelector('[data-icon="bin"]');
  let binItems = [], binSel = null, binSeq = 0;
  const binTarget = (x, y) => document.elementFromPoint(x, y)?.closest('[data-icon="bin"], [data-win="bin"]');
  /* Throwing a thing away does nothing to it. It only shows up in the bin. */
  const trash = item => {
    binItems.push({ ...item, id: ++binSeq });
    renderBin();
    binIcon.classList.remove('gulp');
    void binIcon.offsetWidth;
    binIcon.classList.add('gulp');
  };
  /* Drag with the pointer (mouse, pen, or touch). A drop on the bin throws the item
     away; any other drop calls onMove with the new top-left corner. */
  const draggable = (el, item, onMove) => {
    el.addEventListener('pointerdown', e => {
      if (e.button !== 0) return;
      const x0 = e.clientX, y0 = e.clientY, r = el.getBoundingClientRect();
      let ghost = null;
      const pos = ev => [r.left + ev.clientX - x0, r.top + ev.clientY - y0];
      const move = ev => {
        if (!ghost && Math.hypot(ev.clientX - x0, ev.clientY - y0) < 5) return;
        if (!ghost) {
          ghost = el.cloneNode(true);
          ghost.removeAttribute('data-icon');
          ghost.classList.add('drag-ghost');
          ghost.style.width = `${r.width}px`;
          layer.append(ghost);
        }
        const [x, y] = pos(ev);
        ghost.style.left = `${x}px`;
        ghost.style.top = `${y}px`;
        binIcon.classList.toggle('drop-hot', el !== binIcon && !!binTarget(ev.clientX, ev.clientY));
      };
      const end = ev => {
        el.removeEventListener('pointermove', move);
        el.removeEventListener('pointerup', end);
        el.removeEventListener('pointercancel', end);
        if (!ghost) return;
        ghost.remove();
        binIcon.classList.remove('drop-hot');
        el.dataset.dragged = '1';
        setTimeout(() => { delete el.dataset.dragged; }, 50);
        if (ev.type === 'pointercancel') return;
        if (el !== binIcon && binTarget(ev.clientX, ev.clientY)) trash(item());
        else onMove?.(...pos(ev));
      };
      el.setPointerCapture(e.pointerId);
      el.addEventListener('pointermove', move);
      el.addEventListener('pointerup', end);
      el.addEventListener('pointercancel', end);
    });
  };
  /* A click selects. A double-click, Enter, or a tap on a selected item opens. */
  const activate = (el, group, onOpen, onPick) => {
    let touch = false;
    el.addEventListener('pointerdown', e => { touch = e.pointerType === 'touch'; });
    el.addEventListener('click', () => {
      if (el.dataset.dragged) return;
      const was = el.classList.contains('sel');
      document.querySelectorAll(group).forEach(x => x.classList.toggle('sel', x === el));
      onPick?.();
      if (touch && was) onOpen();
    });
    el.addEventListener('dblclick', () => { if (!el.dataset.dragged) onOpen(); });
    el.addEventListener('keydown', e => { if (e.key === 'Enter') { e.preventDefault(); onOpen(); } });
  };

  const deskItem = el => ({ name: el.querySelector('span').textContent, kind: el.dataset.icon === 'docs' ? 'File Folder' : 'Application', svg: el.querySelector('svg') });
  for (const el of layer.querySelectorAll('.dicon')) {
    const id = el.dataset.icon;
    activate(el, '.dicon', () => openWin(id));
    draggable(el, () => deskItem(el), (x, y) => {
      el.style.left = `${clamp(x, 0, document.documentElement.clientWidth - el.offsetWidth)}px`;
      el.style.top = `${clamp(y, 0, window.innerHeight - tbH() - el.offsetHeight)}px`;
    });
  }
  document.addEventListener('pointerdown', e => {
    if (!e.target.closest('.dicon, [data-modal]')) layer.querySelectorAll('.dicon.sel').forEach(x => x.classList.remove('sel'));
  });

  // ---------- Documents: the public posts as .md files ----------
  const docsEl = layer.querySelector('[data-win="docs"]');
  const docsView = docsEl.querySelector('[data-view-files]');
  const docsStatus = docsEl.querySelector('[data-status]');
  let posts = [], docsMode = 'icons';
  const kb = p => Math.max(1, Math.ceil((p.word_count * 6) / 1024));
  const docsSel = () => posts.find(p => p.slug === docsView.querySelector('.ex-file.sel')?.dataset.slug) || null;
  const showDocsStatus = () => {
    const s = docsSel();
    docsStatus.textContent = s ? `1 object(s) selected · ${kb(s)} KB` : `${posts.length} object(s)`;
  };
  const mdItem = p => ({ name: `${p.slug}.md`, kind: 'Markdown File', svg: icon('md') });
  const openPost = p => { if (p) app.go(`/posts/${p.slug}`); };
  const renderDocs = () => {
    docsView.className = `ex-view sunken ${docsMode}`;
    docsView.replaceChildren();
    if (docsMode === 'details') {
      docsView.append(h('div', { class: 'ex-hdr' }, ['Name', 'Size', 'Type', 'Modified'].map(t => h('span', { text: t }))));
    }
    for (const p of posts) {
      const name = `${p.slug}.md`;
      const b = h('button', { class: 'ex-file', type: 'button', title: p.title, 'data-slug': p.slug },
        docsMode === 'icons'
          ? [icon('md'), h('span', { text: name })]
          : [h('span', { class: 'nm' }, icon('md16'), h('em', { text: name })), h('span', { text: `${kb(p)} KB` }),
            h('span', { text: 'Markdown File' }), h('span', { text: fmtDate(p.published_at, 'us') })]);
      activate(b, '[data-win="docs"] .ex-file', () => openPost(p), showDocsStatus);
      draggable(b, () => mdItem(p));
      docsView.append(b);
    }
    if (!posts.length) docsView.append(h('p', { class: 'empty-note', text: 'This folder is empty.' }));
    showDocsStatus();
  };
  const loadDocs = async () => {
    docsStatus.textContent = 'Loading...';
    try { posts = (await Api.get('/api/posts')) || []; } catch { posts = []; docsStatus.textContent = 'Could not read the folder.'; }
    renderDocs();
  };
  const selectNone = view => { view.querySelectorAll('.sel').forEach(x => x.classList.remove('sel')); };
  docsView.addEventListener('pointerdown', e => { if (e.target === docsView) { selectNone(docsView); showDocsStatus(); } });
  menubar(docsEl.querySelector('[data-menu]'), {
    File: () => [
      { label: 'Open', run: () => openPost(docsSel()), disabled: !docsSel() },
      { label: 'Delete', key: 'Del', run: () => trash(mdItem(docsSel())), disabled: !docsSel() },
      '-',
      { label: 'Close', run: () => closeWin('docs') },
    ],
    Edit: () => [{ label: 'Select None', run: () => { selectNone(docsView); showDocsStatus(); } }],
    View: () => [
      { label: 'Large Icons', check: docsMode === 'icons', run: () => { docsMode = 'icons'; renderDocs(); } },
      { label: 'Details', check: docsMode === 'details', run: () => { docsMode = 'details'; renderDocs(); } },
      '-',
      { label: 'Refresh', key: 'F5', run: loadDocs },
    ],
    Help: () => [{ label: 'About Documents', run: () => tell('Documents', 'Every published post is here as a Markdown file. Double-click a file to read the post in the Logbook.') }],
  });
  register('docs', docsEl, { off: [-40, -30], onOpen: loadDocs, focusEl: () => docsView });

  // ---------- Recycle Bin ----------
  const binEl = layer.querySelector('[data-win="bin"]');
  const binView = binEl.querySelector('[data-view-files]');
  const binStatus = binEl.querySelector('[data-status]');
  const showBinStatus = () => { binStatus.textContent = binSel ? '1 object(s) selected' : `${binItems.length} object(s)`; };
  const renderBin = () => {
    // SVG elements have no hidden property: set the attribute.
    binIcon.querySelector('[data-bin="empty"]').toggleAttribute('hidden', binItems.length > 0);
    binIcon.querySelector('[data-bin="full"]').toggleAttribute('hidden', !binItems.length);
    binView.replaceChildren();
    if (!binItems.length) binView.append(h('p', { class: 'empty-note', text: 'The Recycle Bin is empty.' }));
    for (const it of binItems) {
      const b = h('button', { class: binSel === it.id ? 'ex-file sel' : 'ex-file', type: 'button' }, it.svg.cloneNode(true), h('span', { text: it.name }));
      activate(b, '[data-win="bin"] .ex-file', () => tell('Recycle Bin', [it.name, `Type: ${it.kind}`, 'To use this file, restore it first.']), () => { binSel = it.id; showBinStatus(); });
      binView.append(b);
    }
    showBinStatus();
  };
  const emptyBin = async () => {
    const n = binItems.length;
    if (!n) return;
    const r = await dialog({
      title: n === 1 ? 'Confirm File Delete' : 'Confirm Multiple File Delete', icon: 'q',
      body: n === 1 ? `Are you sure you want to delete '${binItems[0].name}'?` : `Are you sure you want to delete these ${n} items?`,
      buttons: [{ label: 'Yes', value: 'yes', def: true }, { label: 'No', value: 'no' }],
    });
    if (r.value !== 'yes') return;
    binItems = [];
    binSel = null;
    renderBin();
  };
  const restore = () => { binItems = binItems.filter(x => x.id !== binSel); binSel = null; renderBin(); };
  binView.addEventListener('pointerdown', e => { if (e.target === binView) { binSel = null; selectNone(binView); showBinStatus(); } });
  menubar(binEl.querySelector('[data-menu]'), {
    File: () => [
      { label: 'Restore', run: restore, disabled: !binSel },
      { label: 'Empty Recycle Bin', run: emptyBin, disabled: !binItems.length },
      '-',
      { label: 'Close', run: () => closeWin('bin') },
    ],
    Edit: () => [{ label: 'Select None', run: () => { binSel = null; renderBin(); } }],
    Help: () => [{ label: 'About Recycle Bin', run: () => tell('Recycle Bin', 'Nothing you throw away is deleted from the Logbook.') }],
  });
  register('bin', binEl, { off: [20, 10], focusEl: () => binView });
  renderBin();

  // Delete on a selected file or desktop icon throws it away.
  document.addEventListener('keydown', e => {
    if (e.key !== 'Delete' || !modal.hidden || !general()) return;
    const t = e.target.closest?.('[data-win="docs"] .ex-file.sel, .dicon.sel');
    if (!t || t === binIcon) return;
    e.preventDefault();
    const p = posts.find(x => x.slug === t.dataset.slug);
    trash(p ? mdItem(p) : deskItem(t));
  });

  // ---------- Notepad (visitors): plain text, saved as .md or .txt on this computer ----------
  const npEl = layer.querySelector('[data-win="notepad"]');
  const np = npEl.querySelector('textarea');
  const npTask = document.querySelector('.taskbar [data-task="notepad"] span');
  const FONTS = { Cousine: 'Cousine, "Courier New", monospace', Tinos: 'Tinos, "Times New Roman", serif', Tahoma: 'Tahoma, Verdana, sans-serif' };
  const N = { base: 'Untitled', ext: 'md', saved: false, dirty: false, wrap: true, q: '', mcase: false, up: false, clip: '' };
  const npFile = () => (N.saved ? `${N.base}.${N.ext}` : 'Untitled');
  const npTitle = () => {
    const t = `${npFile()} - Notepad`;
    npEl.querySelector('[data-title]').textContent = t;
    npTask.textContent = t;
  };
  np.addEventListener('input', () => { N.dirty = true; });
  const npWrite = () => {
    saveFile(`${N.base}.${N.ext}`, np.value);
    N.saved = true;
    N.dirty = false;
    npTitle();
    return true;
  };
  const npSaveAs = async () => {
    const r = await saveAsDialog({ name: N.saved ? N.base : 'Untitled', type: N.ext, types: [['md', 'Markdown (*.md)'], ['txt', 'Text Documents (*.txt)']] });
    if (!r) return false;
    N.base = r.name;
    N.ext = r.type;
    return npWrite();
  };
  const npSave = () => (N.saved ? npWrite() : npSaveAs());
  /* Unsaved changes: Yes saves, No goes on, Cancel stops. */
  const npAsk = async () => {
    if (!N.dirty) return true;
    const r = await dialog({ title: 'Notepad', icon: 'warn', body: [`The text in the ${npFile()} file has changed.`, 'Do you want to save the changes?'], buttons: YNC });
    if (r.value === 'yes') return npSave();
    return r.value === 'no';
  };
  const npReset = () => { np.value = ''; Object.assign(N, { base: 'Untitled', ext: 'md', saved: false, dirty: false }); npTitle(); };
  const npNew = async () => { if (await npAsk()) npReset(); };
  const npOpen = async () => {
    if (!(await npAsk())) return;
    const f = await pickFile('.md,.markdown,.txt,text/plain,text/markdown');
    if (!f) return;
    let t;
    try { t = await f.text(); } catch { tell('Notepad', `Notepad cannot read ${f.name}.`, 'warn'); return; }
    const m = f.name.match(/^(.*?)(?:\.([^.]+))?$/);
    np.value = t;
    Object.assign(N, { base: m[1] || 'Untitled', ext: /^txt$/i.test(m[2] || '') ? 'txt' : 'md', saved: true, dirty: false });
    npTitle();
    np.focus();
    np.setSelectionRange(0, 0);
    np.scrollTop = 0;
  };
  const hasSel = () => np.selectionStart !== np.selectionEnd;
  /* Text changes go through execCommand where it works, so Undo can undo them. */
  const npInsert = t => {
    np.focus();
    if (!document.execCommand('insertText', false, t)) { np.setRangeText(t, np.selectionStart, np.selectionEnd, 'end'); N.dirty = true; }
  };
  const npCopy = () => {
    if (!hasSel()) return;
    N.clip = np.value.slice(np.selectionStart, np.selectionEnd);
    navigator.clipboard?.writeText?.(N.clip)?.catch(() => {});
  };
  const npDelete = () => {
    if (!hasSel()) return;
    np.focus();
    if (!document.execCommand('delete')) { np.setRangeText('', np.selectionStart, np.selectionEnd, 'end'); N.dirty = true; }
  };
  const npPaste = async () => {
    let t = '';
    try { t = await navigator.clipboard.readText(); } catch { t = N.clip; }
    if (t) npInsert(t);
    else tell('Notepad', ['There is no text to paste.', 'To paste from another program, press Ctrl+V.']);
  };
  const npTime = () => {
    const d = new Date();
    npInsert(`${d.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })} ${d.getMonth() + 1}/${d.getDate()}/${d.getFullYear()}`);
  };
  const npWrap = () => { N.wrap = !N.wrap; np.classList.toggle('nowrap', !N.wrap); np.wrap = N.wrap ? 'soft' : 'off'; };
  const npFont = async () => {
    const cs = getComputedStyle(np);
    const fam = Object.keys(FONTS).find(k => cs.fontFamily.includes(k)) || 'Cousine';
    const famIn = h('select', { class: 'in98', id: 'nf-f' }, [['Cousine', 'Courier New'], ['Tinos', 'Times New Roman'], ['Tahoma', 'Tahoma']].map(([v, l]) => h('option', { value: v, text: l })));
    const sizeIn = h('select', { class: 'in98', id: 'nf-s' }, [10, 12, 13, 14, 16, 18, 20, 24].map(s => h('option', { text: String(s) })));
    const sample = h('span', { class: 'sunken sample', text: 'AaBbYyZz' });
    famIn.value = fam;
    sizeIn.value = String(parseInt(cs.fontSize, 10));
    const show = () => { sample.style.font = `${sizeIn.value}px ${FONTS[famIn.value]}`; };
    const body = h('div', { onchange: show },
      h('div', { class: 'frow' }, h('label', { for: 'nf-f', text: 'Font:' }), famIn),
      h('div', { class: 'frow' }, h('label', { for: 'nf-s', text: 'Size:' }), sizeIn),
      h('div', { class: 'frow' }, h('span', { text: 'Sample:' }), sample));
    show();
    const r = await dialog({ title: 'Font', body, width: 340, buttons: OKCANCEL });
    if (r.value === 'ok') np.style.font = `${sizeIn.value}px/1.45 ${FONTS[famIn.value]}`;
  };
  const findNext = async () => {
    if (!N.q) return npFind();
    const t = N.mcase ? np.value : np.value.toLowerCase(), q = N.mcase ? N.q : N.q.toLowerCase();
    const i = N.up ? (np.selectionStart > 0 ? t.lastIndexOf(q, np.selectionStart - 1) : -1) : t.indexOf(q, np.selectionEnd);
    if (i < 0) { await tell('Notepad', `Cannot find "${N.q}"`); return false; }
    np.focus();
    np.setSelectionRange(i, i + q.length);
    return true;
  };
  const npFind = async () => {
    const q = h('input', { class: 'in98', id: 'f-q', autocomplete: 'off', spellcheck: 'false' });
    const mcase = h('input', { type: 'checkbox' });
    const dir = v => h('input', { type: 'radio', name: 'fdir', value: v });
    const up = dir('up'), down = dir('down');
    q.value = N.q;
    mcase.checked = N.mcase;
    (N.up ? up : down).checked = true;
    const body = h('div', {},
      h('div', { class: 'frow' }, h('label', { for: 'f-q', text: 'Find what:' }), q),
      h('label', { class: 'rad' }, mcase, ' Match case'),
      h('fieldset', { class: 'grp' }, h('legend', { text: 'Direction' }),
        h('div', { class: 'indent flush' }, h('label', { class: 'rad' }, up, ' Up'), h('label', { class: 'rad' }, down, ' Down'))));
    await dialog({ title: 'Find', body, width: 380, buttons: [
      { label: 'Find Next', value: 'ok', def: true, run: async d => {
        N.q = q.value;
        N.mcase = mcase.checked;
        N.up = up.checked;
        if (N.q) await findNext();
        d.querySelector('.default').focus();
        return false;
      } },
      { label: 'Cancel', value: 'cancel' }] });
  };
  menubar(npEl.querySelector('[data-menu]'), {
    File: () => [
      { label: 'New', key: 'Ctrl+N', run: npNew },
      { label: 'Open…', key: 'Ctrl+O', run: npOpen },
      { label: 'Save', key: 'Ctrl+S', run: npSave },
      { label: 'Save As…', run: npSaveAs },
      '-',
      { label: 'Exit', key: 'Alt+F4', run: () => closeWin('notepad') },
    ],
    Edit: () => [
      { label: 'Undo', key: 'Ctrl+Z', run: () => { np.focus(); document.execCommand('undo'); } },
      '-',
      { label: 'Cut', key: 'Ctrl+X', disabled: !hasSel(), run: () => { npCopy(); npDelete(); } },
      { label: 'Copy', key: 'Ctrl+C', disabled: !hasSel(), run: npCopy },
      { label: 'Paste', key: 'Ctrl+V', run: npPaste },
      { label: 'Delete', key: 'Del', disabled: !hasSel(), run: npDelete },
      '-',
      { label: 'Select All', key: 'Ctrl+A', run: () => { np.focus(); np.select(); } },
      { label: 'Time/Date', key: 'F5', run: npTime },
      '-',
      { label: 'Word Wrap', check: N.wrap, run: npWrap },
      { label: 'Set Font…', run: npFont },
    ],
    Search: () => [
      { label: 'Find…', key: 'Ctrl+F', run: npFind },
      { label: 'Find Next', key: 'F3', run: findNext },
    ],
    Help: () => [{ label: 'About Notepad', run: () => tell('About Notepad', ['Notepad', 'Logbook 98']) }],
  });
  register('notepad', npEl, {
    off: [-70, 30], canClose: npAsk, onClose: npReset, focusEl: () => np,
    keys: e => {
      const k = e.key.toLowerCase();
      const fn = e.ctrlKey || e.metaKey ? { n: npNew, o: npOpen, s: npSave, f: npFind }[k] : { f3: findNext, f5: npTime }[k];
      if (fn && !e.altKey) { e.preventDefault(); fn(); }
    },
  });

  // ---------- Paint: loads on demand ----------
  let paintLoad = null;
  const asset = (name, path) => document.querySelector(`meta[name="${name}"]`)?.content || path;
  const fetchTag = (tag, attrs) => new Promise((ok, fail) => {
    const el = h(tag, { ...attrs, onload: ok, onerror: () => fail(new Error(`cannot load ${attrs.src || attrs.href}`)) });
    document.head.append(el);
  });
  const loadPaint = () => (paintLoad ||= (async () => {
    await Promise.all([
      fetchTag('link', { rel: 'stylesheet', href: asset('asset-paint-css', '/static/css/paint.css') }),
      fetchTag('script', { src: asset('asset-paint-js', '/static/js/paint.js') }),
    ]);
    const el = await PaintApp.start(asset('asset-paint-wasm', '/static/wasm/logbook_paint.wasm'));
    layer.insertBefore(el, modal);
    register('paint', el, { off: [40, 0], ...PaintApp.hooks });
  })().catch(e => { paintLoad = null; throw e; }));
  const openPaint = async () => {
    document.documentElement.classList.add('wait');
    try {
      await loadPaint();
      openWin('paint');
    } catch {
      tell('Paint', 'Paint could not start. Check the connection, then try again.', 'warn');
    } finally {
      document.documentElement.classList.remove('wait');
    }
  };

  // ---------- Start menu and keys ----------
  const openApp = id => {
    if (!general()) return;
    if (id === 'paint') openPaint();
    // The owner writes posts in Compose. Visitors get Notepad.
    else if (id === 'notepad' && app.hasAttribute('data-owner')) app.go('/write');
    else openWin(id);
  };
  app.addEventListener('openapp', e => openApp(e.detail.app));
  // The shortcuts of an app work while its window is active.
  document.addEventListener('keydown', e => {
    if (!general() || !modal.hidden || e.defaultPrevented) return;
    const w = W[active];
    if (!w?.keys || !isActive(active) || e.target.closest?.('.mdrop')) return;
    if (!w.el.contains(e.target) && e.target !== document.body) return;
    w.keys(e);
  });

  return { h, svg, icon, dialog, tell, OKBTN, OKCANCEL, YNC, saveAsDialog, saveFile, pickFile, menubar, layer, modal, general, narrow, isActive, closeWin };
})();
