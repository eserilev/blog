/* Paint (spec 11.9). apps.js loads this file the first time Paint opens.

   The picture lives in WASM memory (paint-wasm, which wraps logbook-paint). Target 0
   is the picture. Target 1 is the floating selection. The canvas on the screen is a
   copy of target 0: draw() puts it there after each change.

   Each pixel change goes through a WASM function where the core has one: pencil,
   lines, curves, polygon edges, the square brush, the eraser and the color eraser,
   airbrush dots, fill, rectangles, pick color, flip, rotate, invert, black and white,
   stretch, skew, resize, the selection, undo, and repeat. The core has no ellipse,
   rounded rectangle, round brush, text, free-form mask, or image files. These use
   Canvas 2D on the screen canvas, which holds the same pixels as the core. pull()
   then copies the changed pixels back into WASM memory.

   Rules: text goes in with textContent. Sizes go through CSSOM (CSP). A view on WASM
   memory is new after each call, because a call can move the buffers. */
const PaintApp = (() => {
  const { h, svg, dialog, tell, OKBTN, OKCANCEL, YNC, saveAsDialog, saveFile, pickFile, menubar, layer, modal, isActive, closeWin } = Win98;
  let X = null;

  // ---------- Colors and the WASM core ----------
  const WHITE = 0xffffffff;
  const hex = c => [1, 3, 5].map(i => parseInt(c.slice(i, i + 2), 16));
  /* '#rrggbb' → an opaque RGBA pixel, little-endian (the byte order of ImageData). */
  const px = c => { const [r, g, b] = hex(c); return (r | (g << 8) | (b << 16) | (255 << 24)) >>> 0; };
  const toHex = v => `#${[v & 255, (v >>> 8) & 255, (v >>> 16) & 255].map(n => n.toString(16).padStart(2, '0')).join('')}`;
  const dims = t => [X.paint_width(t), X.paint_height(t)];
  const view = t => new Uint8ClampedArray(X.memory.buffer, X.paint_ptr(t), 4 * X.paint_width(t) * X.paint_height(t));
  const MAX = 4096;

  // ---------- The window ----------
  const ctl = (attr, label, text) => h('button', { class: 'ctl raised', type: 'button', [attr]: true, 'aria-label': label, text });
  const titleEl = h('span', { class: 'ttl', text: 'untitled - Paint' });
  const cv = h('canvas', { class: 'p-cv', width: '560', height: '340', 'aria-label': 'Drawing area' });
  const ov = h('canvas', { class: 'p-ov', width: '560', height: '340', 'aria-hidden': 'true' });
  const ctx = cv.getContext('2d', { willReadFrequently: true });
  const octx = ov.getContext('2d');
  const gridEl = h('div', { class: 'p-grid', hidden: true });
  const txt = h('textarea', { class: 'p-txt', 'aria-label': 'Text', spellcheck: 'false', hidden: true });
  const ghost = h('div', { class: 'p-ghost', hidden: true });
  const handles = ['r', 'b', 'rb'].map(d => h('span', { class: 'p-h', 'data-rs': d }));
  const sheet = h('div', { class: 'p-sheet' }, cv, ov, gridEl, txt, handles, ghost);
  const well = h('div', { class: 'p-well sunken' }, sheet);
  const toolsEl = h('div', { class: 'p-tools', role: 'toolbar', 'aria-label': 'Tools' });
  const optsEl = h('div', { class: 'p-opts sunken', role: 'group', 'aria-label': 'Tool options' });
  const toolbox = h('div', { class: 'p-toolbox' }, toolsEl, optsEl);
  const fgEl = h('i', { class: 'fg' }), bgEl = h('i', { class: 'bg' });
  const palEl = h('div', { class: 'p-palette', role: 'group', 'aria-label': 'Color box' });
  const colorbox = h('div', { class: 'p-colorbar' }, h('button', { class: 'p-cur sunken', type: 'button', 'aria-label': 'Current colors. Click to swap.', onclick: () => { [P.fg, P.bg] = [P.bg, P.fg]; showColors(); } }, fgEl, bgEl), palEl);
  const HELP = 'For Help, click Help Topics on the Help Menu.';
  const helpEl = h('span', { text: HELP }), xyEl = h('span'), sizeEl = h('span');
  const statusEl = h('div', { class: 'p-status' }, helpEl, xyEl, sizeEl);
  const fam = h('select', { class: 'in98', 'aria-label': 'Font' }, [['Tinos', 'Times New Roman'], ['Tahoma', 'Tahoma'], ['Cousine', 'Courier New'], ['Comic', 'Comic Sans MS']].map(([v, l]) => h('option', { value: v, text: l })));
  const fsize = h('select', { class: 'in98', 'aria-label': 'Font size' }, [8, 10, 12, 14, 16, 18, 20, 24, 28, 36, 48, 72].map(s => h('option', { text: String(s) })));
  const fbtn = (k, label, tag) => h('button', { class: 'btn98', type: 'button', 'aria-pressed': 'false', 'aria-label': label, onclick: () => { P.font[k] = !P.font[k]; setFont(); } }, h(tag, { text: label[0] }));
  const fontBtns = { b: fbtn('b', 'Bold', 'b'), i: fbtn('i', 'Italic', 'i'), u: fbtn('u', 'Underline', 'u') };
  const fontbar = h('div', { class: 'p-fontbar', hidden: true }, fam, fsize, Object.values(fontBtns));
  const menuEl = h('div', { class: 'mbar' });
  const el = h('section', { class: 'app raised app-paint', 'data-win': 'paint', 'aria-label': 'Paint', tabindex: '-1', hidden: true },
    h('div', { class: 'titlebar' },
      svg('0 0 16 16', 'ico', [['path', { d: 'M1 3h12v11H1z', fill: '#fff', stroke: '#000' }], ['path', { d: 'M3 6h3v3H3z', fill: '#f00' }], ['path', { d: 'M7 9h3v3H7z', fill: '#00f' }],
        ['path', { d: 'M8 5h3v3H8z', fill: '#ff0' }], ['path', { d: 'M15 0 9 8l1 1 6-7z', fill: '#804000' }], ['path', { d: 'M9 8l-2 3 3-2z', fill: '#808080' }]]),
      titleEl, ctl('data-min', 'Minimize', '_'), ctl('data-max', 'Maximize', '□'), ctl('data-close', 'Close', '×')),
    menuEl, fontbar, h('div', { class: 'p-work' }, toolbox, well), colorbox, statusEl);

  // ---------- State ----------
  const PAL = ['#000000', '#808080', '#800000', '#808000', '#008000', '#008080', '#000080', '#800080', '#808040', '#004040', '#0080ff', '#004080', '#8000ff', '#804000',
    '#ffffff', '#c0c0c0', '#ff0000', '#ffff00', '#00ff00', '#00ffff', '#0000ff', '#ff00ff', '#ffff80', '#00ff80', '#80ffff', '#8080ff', '#ff0080', '#ff8040'];
  const BRUSHES = [[7, 'round'], [4, 'round'], [2, 'round'], [7, 'square'], [4, 'square'], [2, 'square']];
  const FONTS = { Tinos: 'Tinos, "Times New Roman", serif', Tahoma: 'Tahoma, Verdana, sans-serif', Cousine: 'Cousine, "Courier New", monospace', Comic: '"Comic Sans MS", "Comic Neue", cursive' };
  const P = {
    zoom: 1, tool: 'pencil', prev: 'pencil', fg: '#000000', bg: '#ffffff', pal: 0,
    opaque: true, fillMode: 0, lineW: 1, eraser: 8, brush: BRUSHES[1], spray: 10,
    name: 'untitled', type: 'png', saved: false, dirty: false, grid: false,
    view: { toolbox: true, colorbox: true, status: true, textbar: true },
    font: { family: 'Tinos', size: 16, b: false, i: false, u: false },
  };
  fsize.value = '16';
  /* sel: { x, y, w, h, path, float }. A floating selection keeps its pixels in target 1. */
  let sel = null, act = null, curve = null, poly = null, tbox = null, clip = null, sprayT = null;
  const floatCv = h('canvas');

  const mkCanvas = (w, hh) => { const c = h('canvas', { width: String(Math.max(1, Math.round(w))), height: String(Math.max(1, Math.round(hh))) }); c.getContext('2d').imageSmoothingEnabled = false; return c; };
  const clr = () => octx.clearRect(0, 0, ov.width, ov.height);
  const box = (a, b) => [Math.min(a[0], b[0]), Math.min(a[1], b[1]), Math.abs(b[0] - a[0]) + 1, Math.abs(b[1] - a[1]) + 1];
  const fontStr = s => `${P.font.i ? 'italic ' : ''}${P.font.b ? 700 : 400} ${s}px ${FONTS[P.font.family]}`;
  const setTitle = () => {
    const t = `${P.name} - Paint`;
    titleEl.textContent = t;
    document.querySelector('.taskbar [data-task="paint"] span').textContent = t;
  };
  const applyZoom = () => {
    const z = P.zoom;
    for (const c of [cv, ov]) { c.style.width = `${cv.width * z}px`; c.style.height = `${cv.height * z}px`; }
    gridEl.hidden = !(P.grid && z >= 4);
    gridEl.style.backgroundSize = `${z}px ${z}px`;
    sizeEl.textContent = `${cv.width} × ${cv.height}`;
    if (tbox) placeText();
  };
  /* The screen canvas shows target 0. */
  const draw = () => {
    const [w, hh] = dims(0);
    if (cv.width !== w || cv.height !== hh) { cv.width = ov.width = w; cv.height = ov.height = hh; applyZoom(); }
    ctx.putImageData(new ImageData(view(0), w, hh), 0, 0);
  };
  /* Copies a rectangle of the screen canvas back into target 0, after Canvas 2D drew on it. */
  const pull = (x, y, w, hh) => {
    const [cw, ch] = dims(0);
    const x0 = Math.max(0, Math.floor(x)), y0 = Math.max(0, Math.floor(y));
    const x1 = Math.min(cw, Math.ceil(x + w)), y1 = Math.min(ch, Math.ceil(y + hh));
    if (x1 <= x0 || y1 <= y0) return;
    const src = ctx.getImageData(x0, y0, x1 - x0, y1 - y0).data, dst = view(0), row = (x1 - x0) * 4;
    for (let r = 0; r < y1 - y0; r++) dst.set(src.subarray(r * row, (r + 1) * row), ((y0 + r) * cw + x0) * 4);
    draw();
  };
  const setZoom = (z, p) => {
    P.zoom = z;
    applyZoom();
    if (p) { well.scrollLeft = p[0] * z - well.clientWidth / 2; well.scrollTop = p[1] * z - well.clientHeight / 2; }
    if (P.tool === 'zoom') renderOpts();
  };

  // Undo keeps whole pictures, in WASM. A snapshot comes before each change.
  const snap = () => { X.paint_snapshot(); P.dirty = true; };
  const undo = () => { endPending(); if (X.paint_undo()) { draw(); P.dirty = true; } };
  const redo = () => { endPending(); if (X.paint_redo()) { draw(); P.dirty = true; } };

  // ---------- Drawing ----------
  /* The points of a line (Bresenham), for the previews and the round brush. */
  const walk = (a, b, fn) => {
    let [x0, y0] = a;
    const [x1, y1] = b, dx = Math.abs(x1 - x0), dy = -Math.abs(y1 - y0), sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1;
    let err = dx + dy;
    for (;;) {
      fn(x0, y0);
      if (x0 === x1 && y0 === y1) return;
      const e2 = 2 * err;
      if (e2 >= dy) { err += dy; x0 += sx; }
      if (e2 <= dx) { err += dx; y0 += sy; }
    }
  };
  /* A preview line on the overlay. The picture gets lines from X.paint_line. */
  const line2d = (c, a, b, w, col) => { c.fillStyle = col; const o = Math.floor((w - 1) / 2); walk(a, b, (x, y) => c.fillRect(x - o, y - o, w, w)); };
  const brushAt = (a, b, col) => {
    const [s, shape] = P.brush;
    if (shape === 'square' || s <= 2) { X.paint_line(a[0], a[1], b[0], b[1], s, px(col)); draw(); return; }
    ctx.fillStyle = col;
    walk(a, b, (x, y) => { ctx.beginPath(); ctx.arc(x + 0.5, y + 0.5, s / 2, 0, Math.PI * 2); ctx.fill(); });
    pull(Math.min(a[0], b[0]) - s, Math.min(a[1], b[1]) - s, Math.abs(b[0] - a[0]) + 2 * s + 1, Math.abs(b[1] - a[1]) + 2 * s + 1);
  };
  /* Left-drag erases to the back color. Right-drag changes only the front color. */
  const eraseAt = (a, b, colorOnly) => {
    const s = P.eraser, o = s >> 1, f = px(P.fg), bk = px(P.bg);
    walk(a, b, (x, y) => {
      if (colorOnly) X.paint_color_replace(x - o, y - o, s, s, f, bk);
      else X.paint_fill_rect(x - o, y - o, s, s, bk);
    });
    draw();
  };
  const spray = (p, col) => {
    const r = P.spray, c = px(col);
    for (let i = 0; i < r; i++) {
      const a = Math.random() * Math.PI * 2, d = Math.random() * r;
      X.paint_fill_rect(Math.round(p[0] + Math.cos(a) * d), Math.round(p[1] + Math.sin(a) * d), 1, 1, c);
    }
    draw();
  };
  /* Ellipse and rounded rectangle: Canvas 2D. Fill style: 0 outline, 1 outline with back-color fill, 2 solid. */
  const shape2d = (c, kind, a, b, col, alt) => {
    const [x, y, w, hh] = box(a, b), lw = P.lineW, m = P.fillMode;
    c.beginPath();
    if (kind === 'ellipse') c.ellipse(x + w / 2, y + hh / 2, Math.max(0.5, (w - lw) / 2), Math.max(0.5, (hh - lw) / 2), 0, 0, Math.PI * 2);
    else c.roundRect(x + lw / 2, y + lw / 2, Math.max(1, w - lw), Math.max(1, hh - lw), Math.max(0, Math.min(10, w / 3, hh / 3)));
    if (m) { c.fillStyle = m === 1 ? alt : col; c.fill(); }
    if (m < 2) { c.strokeStyle = col; c.lineWidth = lw; c.stroke(); }
  };
  /* The preview of a shape on the overlay. */
  const preview = (kind, a, b, col, alt) => {
    const [x, y, w, hh] = box(a, b), lw = P.lineW, m = P.fillMode;
    if (kind === 'line') return line2d(octx, a, b, lw, col);
    if (kind !== 'rect') return shape2d(octx, kind, a, b, col, alt);
    if (m) { octx.fillStyle = m === 1 ? alt : col; octx.fillRect(x, y, w, hh); }
    if (m < 2) { octx.fillStyle = col; octx.fillRect(x, y, w, lw); octx.fillRect(x, y + hh - lw, w, lw); octx.fillRect(x, y, lw, hh); octx.fillRect(x + w - lw, y, lw, hh); }
  };
  const commitShape = (kind, a, b, col, alt) => {
    snap();
    const [x, y, w, hh] = box(a, b), lw = P.lineW, m = P.fillMode;
    if (kind === 'line') X.paint_line(a[0], a[1], b[0], b[1], lw, px(col));
    else if (kind === 'rect') {
      if (m) X.paint_fill_rect(x, y, w, hh, px(m === 1 ? alt : col));
      if (m < 2) X.paint_rect_outline(x, y, w, hh, lw, px(col));
    } else { shape2d(ctx, kind, a, b, col, alt); pull(x - lw, y - lw, w + 2 * lw, hh + 2 * lw); return; }
    draw();
  };
  /* Polygon: the fill is Canvas 2D, the edges are WASM lines. */
  const drawPoly = (pts, closed, col, alt, target) => {
    const m = closed ? P.fillMode : 0;
    if (m) {
      const c = target ? ctx : octx;
      c.fillStyle = m === 1 ? alt : col;
      c.beginPath();
      pts.forEach(([x, y], i) => c[i ? 'lineTo' : 'moveTo'](x + 0.5, y + 0.5));
      c.closePath();
      c.fill();
      if (target) { const xs = pts.map(q => q[0]), ys = pts.map(q => q[1]); pull(Math.min(...xs) - 1, Math.min(...ys) - 1, Math.max(...xs) - Math.min(...xs) + 3, Math.max(...ys) - Math.min(...ys) + 3); }
    }
    if (m === 2) return;
    const edges = pts.slice(1).map((p, i) => [pts[i], p]);
    if (closed) edges.push([pts[pts.length - 1], pts[0]]);
    for (const [a, b] of edges) {
      if (target) X.paint_line(a[0], a[1], b[0], b[1], P.lineW, px(col));
      else line2d(octx, a, b, P.lineW, col);
    }
    if (target) draw();
  };
  const curvePts = k => {
    const { a, b } = k, c1 = k.c1 || a, c2 = k.c2 || c1;
    const n = Math.max(8, Math.ceil((Math.hypot(b[0] - a[0], b[1] - a[1]) + Math.hypot(c1[0] - a[0], c1[1] - a[1]) + Math.hypot(c2[0] - b[0], c2[1] - b[1])) / 3));
    const out = [a];
    for (let i = 1; i <= n; i++) {
      const t = i / n, u = 1 - t, f = j => Math.round(u * u * u * a[j] + 3 * u * u * t * c1[j] + 3 * u * t * t * c2[j] + t * t * t * b[j]);
      out.push([f(0), f(1)]);
    }
    return out;
  };
  const previewCurve = () => { clr(); const p = curvePts(curve); for (let i = 1; i < p.length; i++) line2d(octx, p[i - 1], p[i], P.lineW, curve.col); };
  const commitCurve = () => {
    snap();
    const p = curvePts(curve), c = px(curve.col);
    for (let i = 1; i < p.length; i++) X.paint_line(p[i - 1][0], p[i - 1][1], p[i][0], p[i][1], P.lineW, c);
    draw();
  };
  /* Shift: lines snap to 45°, other shapes become squares and circles. */
  const constrain = (a, p, e, kind) => {
    if (!e.shiftKey) return p;
    const dx = p[0] - a[0], dy = p[1] - a[1];
    if (kind === 'line') {
      const ang = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4), len = Math.hypot(dx, dy);
      return [a[0] + Math.round(Math.cos(ang) * len), a[1] + Math.round(Math.sin(ang) * len)];
    }
    const m = Math.max(Math.abs(dx), Math.abs(dy));
    return [a[0] + Math.sign(dx || 1) * m, a[1] + Math.sign(dy || 1) * m];
  };

  // ---------- Selection ----------
  const inSel = p => sel && p[0] >= sel.x && p[0] < sel.x + sel.w && p[1] >= sel.y && p[1] < sel.y + sel.h;
  const dash = (x, y, w, hh) => {
    octx.save();
    octx.lineWidth = 1;
    octx.setLineDash([]);
    octx.strokeStyle = '#fff';
    octx.strokeRect(x - 0.5, y - 0.5, w + 1, hh + 1);
    octx.setLineDash([4, 3]);
    octx.strokeStyle = '#000080';
    octx.strokeRect(x - 0.5, y - 0.5, w + 1, hh + 1);
    octx.restore();
  };
  /* The floating copy on the screen. In transparent mode the back color shows through. */
  const refreshFloat = () => {
    const [w, hh] = dims(1), data = view(1).slice();
    if (!P.opaque) {
      const key = px(P.bg), d = new Uint32Array(data.buffer);
      for (let i = 0; i < d.length; i++) if (d[i] === key) d[i] = 0;
    }
    floatCv.width = w;
    floatCv.height = hh;
    floatCv.getContext('2d').putImageData(new ImageData(data, w, hh), 0, 0);
    sel.w = w;
    sel.h = hh;
  };
  const drawSel = () => {
    clr();
    if (!sel) return;
    if (sel.float) octx.drawImage(floatCv, sel.x, sel.y);
    dash(sel.x, sel.y, sel.w, sel.h);
  };
  const pathOn = (c, path, dx, dy) => {
    c.beginPath();
    path.forEach(([x, y], i) => c[i ? 'lineTo' : 'moveTo'](x - dx + 0.5, y - dy + 0.5));
    c.closePath();
  };
  /* Free-form select: pixels of target 1 outside the path become transparent. */
  const maskSel = () => {
    const [w, hh] = dims(1), m = mkCanvas(w, hh), mx = m.getContext('2d');
    pathOn(mx, sel.path, sel.x, sel.y);
    mx.fill();
    const a = mx.getImageData(0, 0, w, hh).data, d = view(1);
    for (let i = 3; i < a.length; i += 4) if (a[i] < 128) d.fill(0, i - 3, i + 1);
  };
  const clearRegion = s => {
    if (!s.path) { X.paint_fill_rect(s.x, s.y, s.w, s.h, px(P.bg)); draw(); return; }
    ctx.fillStyle = P.bg;
    pathOn(ctx, s.path, 0, 0);
    ctx.fill();
    pull(s.x - 1, s.y - 1, s.w + 2, s.h + 2);
  };
  /* Copies the selected pixels into target 1. */
  const grab = () => { X.paint_sel_copy(sel.x, sel.y, sel.w, sel.h); if (sel.path) maskSel(); };
  const lift = keep => { snap(); grab(); if (!keep) clearRegion(sel); sel.float = true; refreshFloat(); };
  /* Draws the floating selection onto the picture. Draw Transparent skips the back color. */
  const commitSel = () => {
    if (sel?.float) { X.paint_sel_blit(sel.x, sel.y, P.opaque ? 0 : 1, px(P.bg)); P.dirty = true; draw(); }
    sel = null;
    clr();
  };
  const setOpaque = v => { P.opaque = v; if (sel?.float) { refreshFloat(); drawSel(); } if (tbox) placeText(); };

  // ---------- Text ----------
  const placeText = () => {
    const z = P.zoom;
    Object.assign(txt.style, {
      left: `${tbox.x * z}px`, top: `${tbox.y * z}px`, width: `${tbox.w * z}px`, height: `${tbox.h * z}px`,
      font: fontStr(P.font.size * z), lineHeight: `${Math.round(P.font.size * 1.2) * z}px`, color: P.fg,
      background: P.opaque ? P.bg : 'transparent', textDecoration: P.font.u ? 'underline' : 'none',
    });
  };
  const startText = (a, b) => {
    const [x, y, w, hh] = box(a, b);
    tbox = { x, y, w: Math.max(w, 140), h: Math.max(hh, Math.round(P.font.size * 1.2) + 6) };
    txt.value = '';
    txt.hidden = false;
    placeText();
    txt.focus();
  };
  txt.addEventListener('input', () => {
    if (!tbox) return;
    const need = Math.ceil(txt.scrollHeight / P.zoom);
    if (need > tbox.h) { tbox.h = need; placeText(); }
  });
  const wrapLines = (c, text, maxW) => {
    const out = [];
    for (const par of text.split('\n')) {
      let ln = '';
      for (const tok of par.split(/(\s+)/)) {
        const t = ln + tok;
        if (c.measureText(t).width > maxW && ln.trim()) { out.push(ln.trimEnd()); ln = tok.trimStart(); } else ln = t;
      }
      out.push(ln);
    }
    return out;
  };
  const commitText = () => {
    if (!tbox) return;
    const v = txt.value, t = tbox;
    tbox = null;
    txt.hidden = true;
    if (!v.trim()) return;
    snap();
    ctx.save();
    ctx.beginPath();
    ctx.rect(t.x, t.y, t.w, t.h);
    ctx.clip();
    if (P.opaque) { ctx.fillStyle = P.bg; ctx.fillRect(t.x, t.y, t.w, t.h); }
    const s = P.font.size, lh = Math.round(s * 1.2);
    ctx.font = fontStr(s);
    ctx.textBaseline = 'top';
    ctx.fillStyle = P.fg;
    wrapLines(ctx, v, t.w - 4).forEach((ln, i) => {
      const y = t.y + 3 + i * lh + Math.round((lh - s) / 2);
      ctx.fillText(ln, t.x + 3, y);
      if (P.font.u) ctx.fillRect(t.x + 3, y + s, ctx.measureText(ln).width, Math.max(1, Math.round(s / 14)));
    });
    ctx.restore();
    pull(t.x, t.y, t.w, t.h);
  };
  const setFont = () => {
    if (tbox) { placeText(); txt.focus(); }
    for (const k of ['b', 'i', 'u']) fontBtns[k].setAttribute('aria-pressed', String(P.font[k]));
  };
  fam.addEventListener('change', () => { P.font.family = fam.value; setFont(); });
  fsize.addEventListener('change', () => { P.font.size = Number(fsize.value); setFont(); });

  const finishShapes = () => {
    if (curve) { clr(); commitCurve(); curve = null; }
    if (poly) { clr(); snap(); drawPoly(poly.pts, poly.pts.length > 2, poly.col, poly.alt, true); poly = null; }
  };
  const endPending = () => { commitText(); finishShapes(); commitSel(); };
  const cancelPending = () => {
    curve = null;
    poly = null;
    if (tbox) { tbox = null; txt.hidden = true; }
    if (sel) commitSel(); else clr();
  };

  // ---------- Tools ----------
  const S = (d, a = {}) => ['path', { d, stroke: '#000', fill: 'none', ...a }];
  const TOOLS = [
    ['free', 'Free-Form Select', 'Selects a free-form part of the picture to move, copy, or edit.', [S('M3 3h4M9 3h4M13 5v3M13 10v3M3 13h3M3 5v3', { 'stroke-dasharray': '2 1' })]],
    ['select', 'Select', 'Selects a rectangular part of the picture to move, copy, or edit.', [S('M2.5 2.5h11v11h-11z', { 'stroke-dasharray': '2 1' })]],
    ['eraser', 'Eraser/Color Eraser', 'Erases part of the picture. Right-drag changes only the front color.', [S('M2 10 8 4h6l-6 6v3H2z', { fill: '#ff9' }), S('M2 10h6v3', { fill: '#fff' })]],
    ['fill', 'Fill With Color', 'Fills an area with the current drawing color.', [S('M3 7 8 2l5 5-5 5z', { fill: '#fff' }), ['path', { d: 'M13 7v6', stroke: '#00f', 'stroke-width': '2' }]]],
    ['picker', 'Pick Color', 'Picks up a color from the picture for drawing.', [S('M12 2l2 2-2 2-1-1-6 6H3v-2l6-6-1-1z', { fill: '#c0c0c0' })]],
    ['zoom', 'Magnifier', 'Changes the magnification.', [['circle', { cx: '6', cy: '6', r: '4', fill: '#9cf', stroke: '#000' }], ['path', { d: 'M9 9l5 5', stroke: '#804000', 'stroke-width': '2' }]]],
    ['pencil', 'Pencil', 'Draws a free-form line one pixel wide.', [S('M3 13l1-4 7-7 3 3-7 7z', { fill: '#ff0' }), S('M3 13l1-4 3 3z', { fill: '#fc9' })]],
    ['brush', 'Brush', 'Draws with a brush of the selected shape and size.', [['path', { d: 'M13 2 7 9l1 1 7-6z', fill: '#804000' }], ['path', { d: 'M7 9c-3 0-3 3-5 5 3 0 6-1 6-4z', fill: '#000' }]]],
    ['spray', 'Airbrush', 'Draws with an airbrush of the selected size.', [S('M8 6h5v8H8z', { fill: '#c0c0c0' }), ['path', { d: 'M9 3h3v3H9z', fill: '#000' }], ['path', { d: 'M2 2h1v1H2zM4 4h1v1H4zM2 6h1v1H2zM5 1h1v1H5z', fill: '#00f' }]]],
    ['text', 'Text', 'Inserts text into the picture.', [['text', { x: '8', y: '13', 'text-anchor': 'middle', 'font-family': 'Tinos, serif', 'font-weight': '700', 'font-size': '13' }, 'A']]],
    ['line', 'Line', 'Draws a straight line with the selected line width. Hold Shift for 45°.', [S('M2 14 14 2', { 'stroke-width': '1.5' })]],
    ['curve', 'Curve', 'Draws a curve. Drag a line, then drag twice to bend it.', [S('M2 13C5 0 11 16 14 3', { 'stroke-width': '1.5' })]],
    ['rect', 'Rectangle', 'Draws a rectangle with the selected fill style.', [S('M2.5 4.5h11v8h-11z')]],
    ['poly', 'Polygon', 'Draws a polygon. Double-click or click the first point to close it.', [S('M2 13 4 3l6 4 4-3v9z')]],
    ['ellipse', 'Ellipse', 'Draws an ellipse with the selected fill style.', [['ellipse', { cx: '8', cy: '8.5', rx: '6', ry: '4.5', stroke: '#000', fill: 'none' }]]],
    ['rrect', 'Rounded Rectangle', 'Draws a rounded rectangle with the selected fill style.', [['rect', { x: '2.5', y: '4.5', width: '11', height: '8', rx: '3', stroke: '#000', fill: 'none' }]]],
  ];
  const CURSOR = { text: 'text', zoom: 'zoom-in', fill: 'cell', picker: 'copy' };
  const selectTool = id => {
    if (id === P.tool) return;
    endPending();
    if (id === 'picker') P.prev = P.tool;
    P.tool = id;
    toolsEl.querySelectorAll('.p-tool').forEach(t => t.setAttribute('aria-pressed', String(t.dataset.tool === id)));
    renderOpts();
    fontbar.hidden = !(id === 'text' && P.view.textbar);
    cv.style.cursor = CURSOR[id] || 'crosshair';
  };
  for (const [id, name, tip, shapes] of TOOLS) {
    toolsEl.append(h('button', {
      class: 'p-tool raised', type: 'button', title: name, 'aria-label': name, 'data-tool': id, 'aria-pressed': String(id === P.tool),
      onpointerenter: () => { helpEl.textContent = tip; }, onpointerleave: () => { helpEl.textContent = HELP; }, onclick: () => selectTool(id),
    }, svg('0 0 16 16', 'pix16', shapes)));
  }
  const OPTS = { select: 'opaque', free: 'opaque', text: 'opaque', eraser: 'eraser', line: 'width', curve: 'width', rect: 'fill', ellipse: 'fill', rrect: 'fill', poly: 'fill', brush: 'brush', spray: 'spray', zoom: 'zoom' };
  /* A small black mark of a given size, for the tool options. */
  const mark = (w, hh, round) => { const i = h('i'); i.style.width = `${w}px`; i.style.height = `${hh}px`; if (round) i.style.borderRadius = '50%'; return i; };
  const renderOpts = () => {
    const kind = OPTS[P.tool];
    optsEl.className = `p-opts sunken${kind ? ` k-${kind}` : ''}`;
    optsEl.replaceChildren();
    const add = (inner, on, fn, label) => optsEl.append(h('button', { class: 'p-opt', type: 'button', title: label, 'aria-label': label, 'aria-pressed': String(on), onclick: () => { fn(); renderOpts(); } }, inner));
    const shapes = bgFill => svg('0 0 30 18', 'p-shapes', [['rect', { x: '1', y: '1', width: '28', height: '16', fill: bgFill, stroke: 'currentColor' }], ['circle', { cx: '10', cy: '9', r: '5', fill: '#f00' }], ['rect', { x: '15', y: '4', width: '10', height: '10', fill: '#00f' }]]);
    if (kind === 'opaque') { add(shapes('#fff'), P.opaque, () => setOpaque(true), 'Draw opaque'); add(shapes('none'), !P.opaque, () => setOpaque(false), 'Draw transparent'); }
    if (kind === 'eraser') [4, 6, 8, 10].forEach(s => add(mark(s, s), P.eraser === s, () => { P.eraser = s; }, `Eraser size ${s}`));
    if (kind === 'width') [1, 2, 3, 4, 5].forEach(s => add(mark(28, s), P.lineW === s, () => { P.lineW = s; }, `Line width ${s}`));
    if (kind === 'fill') [0, 1, 2].forEach(m => add(h('i', { class: `f${m}` }), P.fillMode === m, () => { P.fillMode = m; }, ['Outline', 'Outline with back-color fill', 'Solid'][m]));
    if (kind === 'brush') BRUSHES.forEach(br => add(mark(br[0], br[0], br[1] === 'round'), P.brush === br, () => { P.brush = br; }, `${br[1]} brush, size ${br[0]}`));
    if (kind === 'spray') {
      [6, 10, 16].forEach(r => {
        const dots = [...Array(14)].map((_, i) => ['rect', { x: String(16 + Math.round(Math.cos(i * 2.4) * ((i % 4) + 1) * 3.5)), y: String(16 + Math.round(Math.sin(i * 2.4) * ((i % 4) + 1) * 3.5)), width: '2', height: '2', fill: 'currentColor' }]);
        const s = svg('0 0 32 32', null, dots);
        s.style.width = s.style.height = `${r + 6}px`;
        add(s, P.spray === r, () => { P.spray = r; }, `Airbrush size ${r}`);
      });
    }
    if (kind === 'zoom') [1, 2, 6, 8].forEach(z => add(`${z}x`, P.zoom === z, () => setZoom(z), `Zoom ${z}x`));
  };

  // ---------- Colors ----------
  const showColors = () => { fgEl.style.background = P.fg; bgEl.style.background = P.bg; if (tbox) placeText(); };
  const renderPal = () => {
    palEl.replaceChildren(...PAL.map((c, i) => {
      const b = h('button', {
        class: 'p-sw', type: 'button', 'aria-label': `Color ${c}`, title: 'Click: front color. Right-click: back color. Double-click: edit.',
        onclick: () => { P.fg = c; P.pal = i; showColors(); },
        oncontextmenu: e => { e.preventDefault(); P.bg = c; showColors(); if (sel?.float && !P.opaque) { refreshFloat(); drawSel(); } },
        ondblclick: () => { P.pal = i; editColors(); },
      });
      b.style.background = c;
      return b;
    }));
  };

  // ---------- Pointer input on the canvas ----------
  const pos = e => { const r = cv.getBoundingClientRect(); return [Math.floor(((e.clientX - r.left) * cv.width) / r.width), Math.floor(((e.clientY - r.top) * cv.height) / r.height)]; };
  const hover = p => {
    if (P.tool === 'eraser') {
      clr();
      const s = P.eraser, o = s >> 1;
      octx.fillStyle = '#fff';
      octx.fillRect(p[0] - o, p[1] - o, s, s);
      octx.strokeStyle = '#000';
      octx.lineWidth = 1;
      octx.strokeRect(p[0] - o + 0.5, p[1] - o + 0.5, s - 1, s - 1);
    } else if (poly) { clr(); drawPoly([...poly.pts, p], false, poly.col, poly.alt, false); }
    else if (P.tool === 'select' || P.tool === 'free') cv.style.cursor = inSel(p) ? 'move' : 'crosshair';
  };
  const setCtl = p => { if (curve.stage === 1) { curve.c1 = p; curve.c2 = p; } else curve.c2 = p; previewCurve(); };
  cv.addEventListener('contextmenu', e => e.preventDefault());
  cv.addEventListener('pointerdown', e => {
    if (e.button !== 0 && e.button !== 2) return;
    e.preventDefault();
    cv.setPointerCapture(e.pointerId);
    const p = pos(e), right = e.button === 2, col = right ? P.bg : P.fg, alt = right ? P.fg : P.bg, t = P.tool;
    if (t !== 'text' && tbox) commitText();
    if (t !== 'select' && t !== 'free') commitSel();
    act = { t, a: p, last: p, col, alt, right };
    switch (t) {
      case 'pencil': snap(); X.paint_line(p[0], p[1], p[0], p[1], 1, px(col)); draw(); break;
      case 'brush': snap(); brushAt(p, p, col); break;
      case 'eraser': snap(); eraseAt(p, p, right); break;
      case 'spray': snap(); spray(p, col); sprayT = setInterval(() => act && spray(act.last, col), 30); break;
      case 'fill': snap(); X.paint_flood_fill(p[0], p[1], px(col)); draw(); act = null; break;
      case 'picker': {
        const v = X.paint_get(p[0], p[1]) >>> 0;
        if (v >>> 24) { if (right) P.bg = toHex(v); else P.fg = toHex(v); showColors(); }
        act = null;
        selectTool(P.prev);
        break;
      }
      case 'zoom': setZoom(P.zoom === 1 ? 4 : 1, p); act = null; break;
      case 'text': if (tbox) { commitText(); act = null; } break;
      case 'select': case 'free':
        if (sel && inSel(p)) {
          if (!sel.float) lift(e.ctrlKey);
          // Ctrl-drag leaves a copy behind.
          else if (e.ctrlKey) { X.paint_sel_blit(sel.x, sel.y, P.opaque ? 0 : 1, px(P.bg)); draw(); }
          act.move = [p[0] - sel.x, p[1] - sel.y];
        } else { commitSel(); act.pts = [p]; }
        break;
      case 'curve': if (curve) { act.ctl = true; setCtl(p); } break;
      case 'poly': if (poly) act.next = true; break;
      default: break;
    }
  });
  cv.addEventListener('pointermove', e => {
    const p = pos(e);
    xyEl.textContent = `${p[0]},${p[1]}`;
    if (!act) { hover(p); return; }
    const { t, a, col } = act;
    switch (t) {
      case 'pencil': X.paint_line(act.last[0], act.last[1], p[0], p[1], 1, px(col)); draw(); break;
      case 'brush': brushAt(act.last, p, col); break;
      case 'eraser': eraseAt(act.last, p, act.right); hover(p); break;
      case 'select': case 'free':
        if (act.move) { sel.x = p[0] - act.move[0]; sel.y = p[1] - act.move[1]; drawSel(); }
        else if (t === 'select') { clr(); const [x, y, w, hh] = box(a, p); dash(x, y, w, hh); xyEl.textContent = `${w}×${hh}`; }
        else {
          act.pts.push(p);
          clr();
          octx.strokeStyle = '#000';
          octx.lineWidth = 1;
          octx.beginPath();
          act.pts.forEach(([x, y], i) => octx[i ? 'lineTo' : 'moveTo'](x + 0.5, y + 0.5));
          octx.stroke();
        }
        break;
      case 'line': case 'rect': case 'ellipse': case 'rrect': {
        clr();
        preview(t, a, constrain(a, p, e, t), col, act.alt);
        const [, , w, hh] = box(a, p);
        if (t !== 'line') xyEl.textContent = `${w}×${hh}`;
        break;
      }
      case 'text': { clr(); const [x, y, w, hh] = box(a, p); dash(x, y, w, hh); break; }
      case 'curve': if (act.ctl) setCtl(p); else { clr(); line2d(octx, a, p, P.lineW, col); } break;
      case 'poly': clr(); if (poly) drawPoly([...poly.pts, p], false, poly.col, poly.alt, false); else line2d(octx, a, p, P.lineW, col); break;
      default: break;
    }
    act.last = p;
  });
  const closePoly = () => { if (!poly) return; clr(); snap(); drawPoly(poly.pts, true, poly.col, poly.alt, true); poly = null; };
  const up = e => {
    clearInterval(sprayT);
    if (!act) return;
    const a0 = act;
    act = null;
    const p = e.type === 'pointercancel' ? a0.last : pos(e), { t, a, col, alt } = a0;
    const [cw, ch] = dims(0);
    switch (t) {
      case 'line': case 'rect': case 'ellipse': case 'rrect': clr(); commitShape(t, a, constrain(a, p, e, t), col, alt); break;
      case 'select':
        if (!a0.move) {
          clr();
          const [x, y, w, hh] = box(a, p), x0 = Math.max(0, x), y0 = Math.max(0, y), x1 = Math.min(cw, x + w), y1 = Math.min(ch, y + hh);
          if (x1 - x0 > 1 || y1 - y0 > 1) { sel = { x: x0, y: y0, w: x1 - x0, h: y1 - y0, float: false }; drawSel(); }
        }
        break;
      case 'free':
        if (!a0.move) {
          clr();
          if (a0.pts.length > 2) {
            const xs = a0.pts.map(q => q[0]), ys = a0.pts.map(q => q[1]), x = Math.min(...xs), y = Math.min(...ys);
            sel = { x, y, w: Math.max(...xs) - x + 1, h: Math.max(...ys) - y + 1, path: a0.pts, float: false };
            drawSel();
          }
        }
        break;
      case 'text': clr(); startText(a, p); break;
      case 'curve':
        if (!a0.ctl) { if (p[0] !== a[0] || p[1] !== a[1]) { curve = { a, b: p, col, stage: 1 }; previewCurve(); } else clr(); }
        else if (curve.stage === 1) { curve.stage = 2; previewCurve(); }
        else { clr(); commitCurve(); curve = null; }
        break;
      case 'poly':
        if (!poly) poly = { pts: [a, p], col, alt };
        else if (Math.hypot(p[0] - poly.pts[0][0], p[1] - poly.pts[0][1]) <= 4) { closePoly(); break; }
        else poly.pts.push(p);
        hover(p);
        break;
      default: break;
    }
  };
  cv.addEventListener('pointerup', up);
  cv.addEventListener('pointercancel', up);
  cv.addEventListener('dblclick', closePoly);
  cv.addEventListener('pointerleave', () => { xyEl.textContent = ''; if (!act && P.tool === 'eraser') clr(); });

  // Drag the blue handles to resize the picture.
  for (const hd of handles) {
    hd.addEventListener('pointerdown', e => {
      e.preventDefault();
      e.stopPropagation();
      endPending();
      hd.setPointerCapture(e.pointerId);
      const d = hd.dataset.rs, r = cv.getBoundingClientRect();
      let nw = cv.width, nh = cv.height;
      ghost.hidden = false;
      ghost.style.width = `${r.width}px`;
      ghost.style.height = `${r.height}px`;
      const move = ev => {
        if (d.includes('r')) nw = Math.min(MAX, Math.max(1, Math.round((ev.clientX - r.left) / P.zoom)));
        if (d.includes('b')) nh = Math.min(MAX, Math.max(1, Math.round((ev.clientY - r.top) / P.zoom)));
        ghost.style.width = `${nw * P.zoom}px`;
        ghost.style.height = `${nh * P.zoom}px`;
        sizeEl.textContent = `${nw} × ${nh}`;
      };
      const fin = () => {
        hd.removeEventListener('pointermove', move);
        hd.removeEventListener('pointerup', fin);
        hd.removeEventListener('pointercancel', fin);
        ghost.hidden = true;
        if (nw !== cv.width || nh !== cv.height) { snap(); X.paint_resize(nw, nh, px(P.bg)); draw(); } else applyZoom();
      };
      hd.addEventListener('pointermove', move);
      hd.addEventListener('pointerup', fin);
      hd.addEventListener('pointercancel', fin);
    });
  }

  // ---------- Clipboard ----------
  /* The selected pixels as ImageData. */
  const selPixels = () => {
    if (!sel.float) grab();
    const [w, hh] = dims(1);
    return new ImageData(view(1).slice(), w, hh);
  };
  const canvasOf = img => { const c = mkCanvas(img.width, img.height); c.getContext('2d').putImageData(img, 0, 0); return c; };
  const toSystem = img => {
    try {
      canvasOf(img).toBlob(b => { try { navigator.clipboard?.write?.([new ClipboardItem({ 'image/png': b })])?.catch(() => {}); } catch { /* Not allowed here. */ } });
    } catch { /* Not allowed here. */ }
  };
  const copySel = () => { if (!sel) return false; clip = selPixels(); toSystem(clip); return true; };
  const clearSel = () => {
    if (!sel) return;
    if (!sel.float) { snap(); clearRegion(sel); }
    sel = null;
    clr();
    P.dirty = true;
  };
  const cut = () => { if (copySel()) clearSel(); };
  const selectAll = () => {
    selectTool('select');
    endPending();
    const [w, hh] = dims(0);
    sel = { x: 0, y: 0, w, h: hh, float: false };
    drawSel();
  };
  /* The pixels of a picture (ImageBitmap or ImageData), at most MAX × MAX. */
  const pixelsOf = src => {
    if (src instanceof ImageData) return src;
    const c = mkCanvas(Math.min(MAX, src.width), Math.min(MAX, src.height)), x = c.getContext('2d');
    x.drawImage(src, 0, 0);
    return x.getImageData(0, 0, c.width, c.height);
  };
  const pasteImage = async src => {
    endPending();
    const img = pixelsOf(src), w = img.width, hh = img.height;
    const [cw, ch] = dims(0);
    let grown = false;
    if (w > cw || hh > ch) {
      const r = await dialog({ title: 'Paint', icon: 'q', body: 'The image in the clipboard is larger than the bitmap. Would you like the bitmap enlarged?', buttons: YNC });
      if (r.value === 'cancel') return;
      if (r.value === 'yes') { snap(); X.paint_resize(Math.max(w, cw), Math.max(hh, ch), px(P.bg)); draw(); grown = true; }
    }
    selectTool('select');
    if (!grown) snap();
    X.paint_blank(1, w, hh, 0);
    view(1).set(img.data);
    sel = { x: Math.max(0, Math.floor(well.scrollLeft / P.zoom)), y: Math.max(0, Math.floor(well.scrollTop / P.zoom)), w, h: hh, float: true };
    refreshFloat();
    drawSel();
  };
  const pasteMenu = async () => {
    if (clip) return pasteImage(clip);
    try {
      for (const it of await navigator.clipboard.read()) {
        const t = it.types.find(x => x.startsWith('image/'));
        if (t) return pasteImage(await createImageBitmap(await it.getType(t)));
      }
    } catch { /* Reading the clipboard is not allowed here. */ }
    return tell('Paint', ['The clipboard is empty.', 'To paste a picture from another program, press Ctrl+V.']);
  };
  document.addEventListener('paste', e => {
    if (!isActive('paint') || !modal.hidden || e.target.closest?.('input, textarea, select')) return;
    const item = [...(e.clipboardData?.items || [])].find(i => i.type.startsWith('image/'));
    e.preventDefault();
    if (item) createImageBitmap(item.getAsFile()).then(pasteImage).catch(() => {});
    else if (clip) pasteImage(clip);
  });

  // ---------- Image ----------
  /* With a selection, a change applies to the selection (target 1). Without one, to the picture. */
  const applyX = (kind, arg) => {
    commitText();
    finishShapes();
    let t = 0;
    if (sel) { if (!sel.float) lift(false); t = 1; } else snap();
    // New pixels of a selection are transparent; new pixels of the picture get the back color.
    const bg = t ? 0 : px(P.bg), [w, hh] = dims(t), skew = d => Math.round(Math.tan((d * Math.PI) / 180) * 1000);
    switch (kind) {
      case 'flipH': X.paint_flip_h(t); break;
      case 'flipV': X.paint_flip_v(t); break;
      case 'rot': for (let i = 0; i < arg / 90; i++) X.paint_rotate90(t); break;
      case 'invert': X.paint_invert(t); break;
      case 'bw': X.paint_black_white(t); break;
      case 'stretch': X.paint_stretch(t, Math.max(1, Math.round(w * arg[0])), Math.max(1, Math.round(hh * arg[1]))); break;
      case 'skewX': X.paint_skew_x(t, skew(arg), bg); break;
      case 'skewY': X.paint_skew_y(t, skew(arg), bg); break;
      default: break;
    }
    if (t) { refreshFloat(); drawSel(); } else draw();
    P.dirty = true;
  };
  const clearImage = () => {
    if (sel) { clearSel(); return; }
    endPending();
    snap();
    const [w, hh] = dims(0);
    X.paint_fill_rect(0, 0, w, hh, px(P.bg));
    draw();
  };
  const radio = (name, value, label, checked, disabled) => h('label', { class: 'rad' }, h('input', { type: 'radio', name, value, checked, disabled }), ` ${label}`);
  const flipRotate = async () => {
    const body = h('div', {}, h('fieldset', { class: 'grp' }, h('legend', { text: 'Flip or rotate' }),
      radio('fr', 'flipH', 'Flip horizontal', true), radio('fr', 'flipV', 'Flip vertical'), radio('fr', 'rot', 'Rotate by angle'),
      h('div', { class: 'indent' }, [90, 180, 270].map((d, i) => radio('deg', String(d), `${d}°`, !i, true)))));
    body.addEventListener('change', () => {
      const rot = body.querySelector('input[name=fr]:checked').value === 'rot';
      body.querySelectorAll('input[name=deg]').forEach(i => { i.disabled = !rot; });
    });
    const r = await dialog({ title: 'Flip and Rotate', body, width: 320, buttons: OKCANCEL });
    if (r.value === 'ok') applyX(body.querySelector('input[name=fr]:checked').value, Number(body.querySelector('input[name=deg]:checked').value));
  };
  const num = (id, value, min, max) => h('input', { class: 'in98', type: 'number', id, value: String(value), min: String(min), max: String(max) });
  /* Checks number fields. Each entry: [input, low, high]. */
  const intsOk = async list => {
    const bad = list.find(([i, lo, hi]) => { const v = Number(i.value); return !Number.isInteger(v) || v < lo || v > hi; });
    if (!bad) return true;
    await tell('Paint', `Please enter an integer between ${bad[1]} and ${bad[2]}.`, 'warn');
    bad[0].focus();
    return false;
  };
  const stretchSkew = async () => {
    const sh = num('st-h', 100, 1, 500), sv = num('st-v', 100, 1, 500), kh = num('sk-h', 0, -89, 89), kv = num('sk-v', 0, -89, 89);
    const pair = (id, label, input, unit) => [h('label', { for: id, text: label }), input, h('span', { text: unit })];
    const body = h('div', {},
      h('fieldset', { class: 'grp' }, h('legend', { text: 'Stretch' }), h('div', { class: 'pairs' }, pair('st-h', 'Horizontal:', sh, '%'), pair('st-v', 'Vertical:', sv, '%'))),
      h('fieldset', { class: 'grp' }, h('legend', { text: 'Skew' }), h('div', { class: 'pairs' }, pair('sk-h', 'Horizontal:', kh, 'Degrees'), pair('sk-v', 'Vertical:', kv, 'Degrees'))));
    const r = await dialog({ title: 'Stretch and Skew', body, width: 320, buttons: [
      { label: 'OK', value: 'ok', def: true, run: () => intsOk([[sh, 1, 500], [sv, 1, 500], [kh, -89, 89], [kv, -89, 89]]) },
      { label: 'Cancel', value: 'cancel' }] });
    if (r.value !== 'ok') return;
    const v = i => Number(i.value);
    if (v(sh) !== 100 || v(sv) !== 100) applyX('stretch', [v(sh) / 100, v(sv) / 100]);
    if (v(kh)) applyX('skewX', v(kh));
    if (v(kv)) applyX('skewY', v(kv));
  };
  const attributes = async () => {
    const [cw, ch] = dims(0);
    const aw = num('at-w', cw, 1, MAX), ah = num('at-h', ch, 1, MAX);
    const body = h('div', {},
      h('div', { class: 'pairs' }, h('label', { for: 'at-w', text: 'Width:' }), aw, h('span', { text: 'pels' }), h('label', { for: 'at-h', text: 'Height:' }), ah, h('span', { text: 'pels' })),
      h('fieldset', { class: 'grp' }, h('legend', { text: 'Colors' }), radio('at-c', 'bw', 'Black and white'), radio('at-c', 'c', 'Colors', true)));
    const r = await dialog({ title: 'Attributes', body, width: 300, buttons: [
      { label: 'OK', value: 'ok', def: true, run: () => intsOk([[aw, 1, MAX], [ah, 1, MAX]]) },
      { label: 'Cancel', value: 'cancel' },
      { label: 'Default', run: () => { aw.value = '560'; ah.value = '340'; return false; } }] });
    if (r.value !== 'ok') return;
    const w = Number(aw.value), hh = Number(ah.value);
    endPending();
    if (w !== cw || hh !== ch) { snap(); X.paint_resize(w, hh, px(P.bg)); draw(); }
    if (body.querySelector('input[name=at-c]:checked').value === 'bw') applyX('bw');
  };
  const customZoom = async () => {
    const body = h('div', {}, h('p', { text: `Current zoom: ${P.zoom * 100}%` }),
      h('fieldset', { class: 'grp' }, h('legend', { text: 'Zoom to' }), h('div', { class: 'indent flush' }, [1, 2, 4, 6, 8].map(z => radio('zm', String(z), `${z * 100}%`, z === P.zoom)))));
    const r = await dialog({ title: 'Custom Zoom', body, width: 300, buttons: OKCANCEL });
    const c = body.querySelector('input[name=zm]:checked');
    if (r.value === 'ok' && c) setZoom(Number(c.value));
  };
  const BASIC = ['#ff8080', '#ffff80', '#80ff80', '#00ff80', '#80ffff', '#0080ff', '#ff80c0', '#ff80ff', '#ff0000', '#ffff00', '#80ff00', '#00ff40', '#00ffff', '#0080c0', '#8080c0', '#ff00ff',
    '#804040', '#ff8040', '#00ff00', '#008080', '#004080', '#8080ff', '#800040', '#ff0080', '#800000', '#ff8000', '#008000', '#008040', '#0000ff', '#0000a0', '#800080', '#8000ff',
    '#400000', '#804000', '#004000', '#004040', '#000080', '#000040', '#400040', '#400080', '#000000', '#808000', '#808040', '#808080', '#408080', '#c0c0c0', '#400040', '#ffffff'];
  const editColors = async () => {
    let pick = PAL[P.pal] || P.fg;
    const custom = h('input', { type: 'color', id: 'cc-in', value: pick, oninput: e => { pick = e.target.value; } });
    const grid = h('div', { class: 'p-c48' }, BASIC.map(c => {
      const b = h('button', { type: 'button', class: 'p-sw', 'aria-label': `Color ${c}`, onclick: () => { pick = c; custom.value = c; grid.querySelectorAll('.p-sw').forEach(x => x.classList.toggle('on', x === b)); } });
      b.style.background = c;
      return b;
    }));
    const body = h('div', {}, h('div', { text: 'Basic colors:' }), grid, h('div', { class: 'frow' }, h('label', { for: 'cc-in', text: 'Custom color:' }), custom));
    const r = await dialog({ title: 'Edit Colors', body, width: 300, buttons: OKCANCEL });
    if (r.value !== 'ok') return;
    PAL[P.pal] = pick;
    P.fg = pick;
    renderPal();
    showColors();
  };
  const toggleView = k => {
    P.view[k] = !P.view[k];
    toolbox.hidden = !P.view.toolbox;
    colorbox.hidden = !P.view.colorbox;
    statusEl.hidden = !P.view.status;
    fontbar.hidden = !(P.tool === 'text' && P.view.textbar);
  };
  const toggleGrid = () => { if (P.zoom < 4) return; P.grid = !P.grid; applyZoom(); };
  const viewBitmap = () => {
    endPending();
    const v = h('div', { class: 'p-viewbmp', tabindex: '0', role: 'dialog', 'aria-label': 'View Bitmap. Click or press a key to go back.' }, h('img', { src: cv.toDataURL(), alt: 'Your picture' }));
    const out = () => { v.remove(); el.focus(); };
    v.addEventListener('click', out);
    v.addEventListener('keydown', e => { e.preventDefault(); out(); });
    layer.append(v);
    v.focus();
  };
  const wallpaper = tiled => {
    endPending();
    const w = layer.querySelector('[data-wallpaper]').style;
    w.backgroundImage = `url("${cv.toDataURL()}")`;
    w.backgroundRepeat = tiled ? 'repeat' : 'no-repeat';
    w.backgroundPosition = tiled ? '0 0' : 'center';
  };

  // ---------- Files ----------
  const MIME = { png: 'image/png', jpg: 'image/jpeg' };
  const TYPES = [['png', 'PNG (*.png)'], ['jpg', 'JPEG (*.jpg)']];
  const ACCEPT = 'image/png,image/jpeg,image/gif,image/bmp,image/webp';
  const blobOf = (c, type) => new Promise(r => c.toBlob(r, MIME[type], 0.92));
  const paintWrite = async () => {
    endPending();
    saveFile(`${P.name}.${P.type}`, await blobOf(cv, P.type));
    P.saved = true;
    P.dirty = false;
    setTitle();
    return true;
  };
  const paintSaveAs = async () => {
    const r = await saveAsDialog({ name: P.name, type: P.type, types: TYPES });
    if (!r) return false;
    P.name = r.name;
    P.type = r.type;
    return paintWrite();
  };
  const paintSave = () => (P.saved ? paintWrite() : paintSaveAs());
  /* Unsaved changes: Yes saves, No goes on, Cancel stops. */
  const paintAsk = async () => {
    if (!P.dirty) return true;
    const r = await dialog({ title: 'Paint', icon: 'warn', body: `Save changes to ${P.name}?`, buttons: YNC });
    if (r.value === 'yes') return paintSave();
    return r.value === 'no';
  };
  /* A new white picture, with no history. */
  const resetPaint = (w = 560, hh = 340) => {
    sel = null; act = null; curve = null; poly = null; tbox = null; txt.hidden = true;
    P.zoom = 1;
    P.grid = false;
    X.paint_init(w, hh, WHITE);
    clr();
    draw();
    applyZoom();
    Object.assign(P, { name: 'untitled', type: 'png', saved: false, dirty: false });
    setTitle();
  };
  const paintNew = async () => { if (await paintAsk()) resetPaint(); };
  const paintOpen = async () => {
    if (!(await paintAsk())) return;
    const f = await pickFile(ACCEPT);
    if (!f) return;
    let img;
    try { img = await createImageBitmap(f); } catch { tell('Paint', [`Paint cannot read ${f.name}.`, 'It is not a valid picture, or Paint does not support its format.'], 'warn'); return; }
    // A transparent part of the file becomes white.
    const c = mkCanvas(Math.min(MAX, img.width), Math.min(MAX, img.height)), x = c.getContext('2d');
    x.fillStyle = '#ffffff';
    x.fillRect(0, 0, c.width, c.height);
    x.drawImage(img, 0, 0);
    resetPaint(c.width, c.height);
    view(0).set(x.getImageData(0, 0, c.width, c.height).data);
    draw();
    const m = f.name.match(/^(.*?)(?:\.([^.]+))?$/);
    Object.assign(P, { name: m[1] || 'untitled', type: /^jpe?g$/i.test(m[2] || '') ? 'jpg' : 'png', saved: true, dirty: false });
    setTitle();
  };
  const copyTo = async () => {
    if (!sel) return;
    const c = canvasOf(selPixels());
    const r = await saveAsDialog({ name: 'selection', type: 'png', types: TYPES });
    if (r) saveFile(`${r.name}.${r.type}`, await blobOf(c, r.type));
  };
  const pasteFrom = async () => {
    const f = await pickFile(ACCEPT);
    if (!f) return;
    try { await pasteImage(await createImageBitmap(f)); } catch { tell('Paint', `Paint cannot read ${f.name}.`, 'warn'); }
  };

  menubar(menuEl, {
    File: () => [
      { label: 'New', key: 'Ctrl+N', run: paintNew },
      { label: 'Open…', key: 'Ctrl+O', run: paintOpen },
      { label: 'Save', key: 'Ctrl+S', run: paintSave },
      { label: 'Save As…', run: paintSaveAs },
      '-',
      { label: 'Set As Wallpaper (Tiled)', run: () => wallpaper(true) },
      { label: 'Set As Wallpaper (Centered)', run: () => wallpaper(false) },
      '-',
      { label: 'Exit', key: 'Alt+F4', run: () => closeWin('paint') },
    ],
    Edit: () => [
      { label: 'Undo', key: 'Ctrl+Z', run: undo, disabled: !X.paint_can_undo() && !tbox && !curve && !poly && !sel?.float },
      { label: 'Repeat', key: 'Ctrl+Y', run: redo, disabled: !X.paint_can_redo() },
      '-',
      { label: 'Cut', key: 'Ctrl+X', run: cut, disabled: !sel },
      { label: 'Copy', key: 'Ctrl+C', run: copySel, disabled: !sel },
      { label: 'Paste', key: 'Ctrl+V', run: pasteMenu },
      { label: 'Clear Selection', key: 'Del', run: clearSel, disabled: !sel },
      { label: 'Select All', key: 'Ctrl+A', run: selectAll },
      '-',
      { label: 'Copy To…', run: copyTo, disabled: !sel },
      { label: 'Paste From…', run: pasteFrom },
    ],
    View: () => [
      { label: 'Tool Box', key: 'Ctrl+T', check: P.view.toolbox, run: () => toggleView('toolbox') },
      { label: 'Color Box', key: 'Ctrl+L', check: P.view.colorbox, run: () => toggleView('colorbox') },
      { label: 'Status Bar', check: P.view.status, run: () => toggleView('status') },
      { label: 'Text Toolbar', check: P.view.textbar, run: () => toggleView('textbar') },
      '-',
      { label: 'Normal Size', key: 'Ctrl+PgUp', check: P.zoom === 1, run: () => setZoom(1) },
      { label: 'Large Size', key: 'Ctrl+PgDn', check: P.zoom === 4, run: () => setZoom(4) },
      { label: 'Custom…', run: customZoom },
      { label: 'Show Grid', key: 'Ctrl+G', check: P.grid, disabled: P.zoom < 4, run: toggleGrid },
      '-',
      { label: 'View Bitmap', key: 'Ctrl+F', run: viewBitmap },
    ],
    Image: () => [
      { label: 'Flip/Rotate…', key: 'Ctrl+R', run: flipRotate },
      { label: 'Stretch/Skew…', key: 'Ctrl+W', run: stretchSkew },
      { label: 'Invert Colors', key: 'Ctrl+I', run: () => applyX('invert') },
      { label: 'Attributes…', key: 'Ctrl+E', run: attributes },
      { label: 'Clear Image', key: 'Ctrl+Shft+N', run: clearImage },
      { label: 'Draw Opaque', check: P.opaque, run: () => { setOpaque(!P.opaque); renderOpts(); } },
    ],
    Colors: () => [{ label: 'Edit Colors…', run: editColors }],
    Help: () => [
      { label: 'Help Topics', run: () => dialog({ title: 'Paint Help', width: 420, buttons: OKBTN, body: [
        'Draw: pick a tool on the left. Left-click draws with the front color. Right-click draws with the back color.',
        'Colors: click a color for the front color. Right-click a color for the back color. Double-click a color to change it.',
        'Select: drag a box, then drag inside it to move it. Hold Ctrl to leave a copy behind.',
        'Files: File ▸ Save saves a PNG or JPEG file. File ▸ Open opens a picture.',
      ] }) },
      '-',
      { label: 'About Paint', run: () => tell('About Paint', ['Paint', 'Logbook 98']) },
    ],
  });

  const keys = e => {
    if (e.target.closest?.('input, textarea, select')) {
      if (e.key === 'Escape' && e.target === txt) { e.preventDefault(); commitText(); }
      return;
    }
    const k = e.key.toLowerCase(), c = e.ctrlKey || e.metaKey;
    const map = c ? {
      z: undo, y: redo, x: cut, c: copySel, a: selectAll, n: e.shiftKey ? clearImage : paintNew, o: paintOpen, s: paintSave,
      t: () => toggleView('toolbox'), l: () => toggleView('colorbox'), g: toggleGrid, f: viewBitmap, r: flipRotate, w: stretchSkew,
      i: () => applyX('invert'), e: attributes, pageup: () => setZoom(1), pagedown: () => setZoom(4),
    } : { delete: clearSel, escape: cancelPending };
    const fn = !e.altKey && map[k];
    if (fn) { e.preventDefault(); fn(); }
  };

  return {
    hooks: { canClose: paintAsk, onClose: () => resetPaint(), keys },
    /* Loads the WASM core and returns the Paint window. */
    async start(url) {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`paint: HTTP ${res.status}`);
      const { instance } = await WebAssembly.instantiateStreaming(res, {});
      X = instance.exports;
      renderPal();
      showColors();
      renderOpts();
      cv.style.cursor = 'crosshair';
      resetPaint();
      return el;
    },
  };
})();
