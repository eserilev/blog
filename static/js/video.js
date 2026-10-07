/* <video-embed> (spec 4.10): one YouTube video in a post.

   The server renders a neutral element with data-id (an 11-byte YouTube ID) and
   data-title. Without scripts, the element holds a link to YouTube.

   This component gives the element the look of the view that holds it. The view
   follows <html data-mode>:
   - general: the Win98 page (also the editor, in every mode): a Media Player window.
   - night: the MS-DOS screen: PLAY.EXE, with a text-mode look.
   - phone: the phone screen: the LCD player.
   A mode change shows another view, with its own copy of the post. An embed that
   the change hides pauses.

   Nothing loads from YouTube before the reader clicks play. The click adds the
   player frame (youtube-nocookie.com, enablejsapi=1, controls=0). The skin drives
   it with postMessage commands and reads its infoDelivery messages. No YouTube
   script runs in the page, so script-src does not change. The component accepts a
   message only from https://www.youtube-nocookie.com and only from its own frame.

   Rules: all text goes in with textContent. Sizes go through CSSOM (CSP). */
(() => {
  const ID_RE = /^[A-Za-z0-9_-]{11}$/;
  const YT_ORIGIN = 'https://www.youtube-nocookie.com';
  const SVG_NS = 'http://www.w3.org/2000/svg';
  const STEP = 10;
  /* The handshake: "listening" every 250 ms until the player answers, 10 s at most. */
  const HELLO_MS = 250, HELLO_MAX = 40;
  /* YouTube player states. */
  const ENDED = 0, PLAYING = 1, PAUSED = 2, BUFFERING = 3;

  const two = n => String(n).padStart(2, '0');
  /* "00:42", or "1:02:03" for an hour or more. "--:--" if unknown. */
  const clock = s => {
    if (!Number.isFinite(s) || s < 0) return '--:--';
    s = Math.floor(s);
    const h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60);
    return h ? `${h}:${two(m)}:${two(s % 60)}` : `${two(m)}:${two(s % 60)}`;
  };
  /* "3:15": the length, as people say it. Empty if unknown. */
  const length = s => {
    if (!Number.isFinite(s) || s <= 0) return '';
    return s >= 3600 ? clock(s) : `${Math.floor(s / 60)}:${two(Math.floor(s % 60))}`;
  };
  const num = v => (typeof v === 'number' && Number.isFinite(v) ? v : null);

  const el = (tag, cls, text) => {
    const e = document.createElement(tag);
    if (cls) e.className = cls;
    if (text != null) e.textContent = text;
    return e;
  };
  const btn = (cls, label, ...content) => {
    const b = el('button', cls);
    b.type = 'button';
    if (label) b.setAttribute('aria-label', label);
    b.append(...content);
    return b;
  };
  /* A pixel icon: paths on a small grid, one class per color. */
  const icon = (w, h, paths, cls) => {
    const s = document.createElementNS(SVG_NS, 'svg');
    s.setAttribute('viewBox', `0 0 ${w} ${h}`);
    s.setAttribute('aria-hidden', 'true');
    s.setAttribute('focusable', 'false');
    s.setAttribute('shape-rendering', 'crispEdges');
    s.setAttribute('class', cls);
    for (const [d, c] of paths) {
      const p = document.createElementNS(SVG_NS, 'path');
      p.setAttribute('d', d);
      if (c) p.setAttribute('class', c);
      s.append(p);
    }
    return s;
  };
  const ICONS = {
    play: [6, 6, [['M1 0h1v6H1zM2 1h1v4H2zM3 2h1v2H3z']]],
    pause: [6, 6, [['M1 0h1v6H1zM4 0h1v6H4z']]],
    stop: [6, 6, [['M1 1h4v4H1z']]],
    prev: [7, 6, [['M0 0h1v6H0zM3 0h1v6H3zM2 1h1v4H2zM1 2h1v2H1zM6 0h1v6H6zM5 1h1v4H5zM4 2h1v2H4z']]],
    rew: [7, 6, [['M2 0h1v6H2zM1 1h1v4H1zM0 2h1v2H0zM5 0h1v6H5zM4 1h1v4H4zM3 2h1v2H3z']]],
    ff: [7, 6, [['M0 0h1v6H0zM1 1h1v4H1zM2 2h1v2H2zM3 0h1v6H3zM4 1h1v4H4zM5 2h1v2H5z']]],
    next: [7, 6, [['M0 0h1v6H0zM1 1h1v4H1zM2 2h1v2H2zM3 0h1v6H3zM4 1h1v4H4zM5 2h1v2H5zM6 0h1v6H6z']]],
    volume: [8, 8, [['M0 3h2v2H0zM2 2h1v4H2zM3 1h1v6H3zM5 2h1v1H5zM5 5h1v1H5zM6 3h1v2H6z']]],
    reel: [7, 7, [['M0 0h7v7H0z', 've-c-white'], ['M2 1h1v5H2zM3 2h1v3H3zM4 3h1v1H4z', 've-c-navy']]],
    big: [24, 16, [['M0 2h24v12H0z', 've-c-blue'], ['M1 3h22v10H1z', 've-c-navy'],
      ['M9 5h1v6H9zM10 6h1v4h-1zM11 6h1v4h-1zM12 7h1v2h-1zM13 7h1v2h-1z', 've-c-white'],
      ['M3 0h2v2H3zM19 0h2v2h-2z', 've-c-gray']]],
    lcdPlay: [10, 10, [['M2 1h1v8H2zM3 2h1v6H3zM4 3h1v4H4zM5 4h1v2H5z']]],
  };
  const ico = (name, cls = 've-ico') => {
    const [w, h, p] = ICONS[name];
    return icon(w, h, p, w > h ? `${cls} ve-wide` : cls);
  };

  /* Every connected embed, for the page-wide events below. */
  const live = new Set();
  let uid = 0;
  document.addEventListener('modechange', () => live.forEach(v => v.modeChanged()));
  window.addEventListener('message', e => {
    // Only the YouTube player, and only the frame of one embed.
    if (e.origin !== YT_ORIGIN || !e.source) return;
    for (const v of live) {
      if (v.frame && e.source === v.frame.contentWindow) { v.onMessage(e.data); return; }
    }
  });

  customElements.define('video-embed', class extends HTMLElement {
    connectedCallback() {
      live.add(this);
      if (!this.built) this.build();
    }

    disconnectedCallback() {
      live.delete(this);
      this.pause();
      clearInterval(this.hello);
    }

    /* ---- Set up ---- */

    build() {
      const id = this.dataset.id || '';
      // The server checks the ID (theorem T18). The component checks it again.
      if (!ID_RE.test(id)) return;
      this.built = true;
      this.ytId = id;
      this.uid = ++uid;
      this.label = (this.dataset.title || '').trim();
      this.state = 'idle';
      this.ready = false;
      this.info = { t: 0, d: NaN, volume: 100, muted: false };
      this.textMode = true;
      this.tint = true;
      this.skin = this.closest('dos-shell') ? 'night' : this.closest('phone-shell') ? 'phone' : 'general';
      this.ui = {};
      const figure = { general: () => this.buildGeneral(), night: () => this.buildNight(), phone: () => this.buildPhone() }[this.skin]();
      figure.classList.add('ve', `ve-${this.skin}`);
      this.replaceChildren(figure);
      this.addEventListener('keydown', e => this.onKey(e));
      this.update();
    }

    /* The screen: the pre-play button, later the player frame, and an effect layer
       (the LCD tint, or the text-mode grid) that never takes a click. */
    screen(cls, start) {
      const s = el('div', `ve-screen ${cls || ''}`.trim());
      // MS-DOS: the screen takes the focus, so its keys work after the start
      // button goes. Other skins: the focus moves here from the start button.
      s.tabIndex = this.skin === 'night' ? 0 : -1;
      if (this.skin === 'night') {
        s.setAttribute('role', 'group');
        s.setAttribute('aria-label', `Video player${this.label ? `: ${this.label}` : ''}. Keys: Enter play or pause, Escape stop, A text mode, F full screen.`);
      }
      const fx = el('div', 've-fx');
      fx.setAttribute('aria-hidden', 'true');
      start.classList.add('ve-start');
      start.addEventListener('click', () => this.play());
      s.append(fx, start);
      this.ui.screen = s;
      this.ui.fx = fx;
      this.ui.start = start;
      return s;
    }

    startLabel() {
      return `Play YouTube video${this.label ? `: ${this.label}` : ''}. It loads from YouTube.`;
    }

    /* ---- Win98: a Media Player window ---- */

    buildGeneral() {
      const fig = el('figure');
      const win = el('div', 've-win raised');
      const bar = el('div', 'titlebar ve-titlebar');
      const name = el('span', 'ttl', `Media Player - ${this.label || 'YouTube'}`);
      const ctl = el('span', 've-ctl');
      ctl.setAttribute('aria-hidden', 'true');
      ['_', '□', '×'].forEach(c => ctl.append(el('span', 'ctl raised', c)));
      bar.append(ico('reel', 've-ico ve-reel'), name, ctl);
      const menu = el('div', 'menubar ve-menu');
      menu.setAttribute('aria-hidden', 'true');
      ['File', 'View', 'Play', 'Favorites', 'Go', 'Help'].forEach(m => menu.append(el('span', null, m)));
      this.ui.hint = el('span', 've-hint');
      const screen = this.screen('sunken', btn('', this.startLabel(), ico('big', 've-big'), this.ui.hint));

      const seek = el('input', 've-seek');
      seek.type = 'range';
      seek.min = '0';
      seek.max = '1000';
      seek.step = '1';
      seek.value = '0';
      seek.setAttribute('aria-label', 'Seek');
      seek.addEventListener('change', () => this.seekTo((seek.value / 1000) * this.info.d));
      const track = el('div', 've-track');
      track.append(seek);
      this.ui.seek = seek;

      const row = el('div', 've-buttons');
      const b = (key, label, fn) => { const x = btn('ve-btn', label, ico(key)); x.addEventListener('click', fn); return x; };
      this.ui.play = b('play', 'Play', () => this.play());
      this.ui.pause = b('pause', 'Pause', () => this.pause());
      const sep = el('span', 've-sep');
      sep.setAttribute('aria-hidden', 'true');
      const vol = el('input', 've-volume');
      vol.type = 'range';
      vol.min = '0';
      vol.max = '100';
      vol.step = '5';
      vol.value = '100';
      vol.setAttribute('aria-label', 'Volume');
      vol.addEventListener('input', () => this.setVolume(Number(vol.value)));
      this.ui.volume = vol;
      row.append(
        this.ui.play, this.ui.pause, b('stop', 'Stop', () => this.stop()), sep,
        // One video has no previous or next track: these go to the start and the end.
        b('prev', 'Go to the start', () => this.seekTo(0)),
        b('rew', `Back ${STEP} seconds`, () => this.seekBy(-STEP)),
        b('ff', `Forward ${STEP} seconds`, () => this.seekBy(STEP)),
        b('next', 'Go to the end', () => this.seekTo(this.info.d)),
        el('span', 've-grow'), ico('volume', 've-ico ve-vol-ico'), vol,
      );
      const status = el('div', 'status ve-status');
      this.ui.state = el('span', 'sunken ve-state');
      this.ui.state.setAttribute('aria-live', 'polite');
      this.ui.clock = el('span', 'sunken ve-clock');
      this.ui.sound = el('span', 'sunken ve-sound');
      status.append(this.ui.state, this.ui.clock, this.ui.sound);
      win.append(bar, menu, screen, track, row, status);
      this.ui.caption = el('figcaption', 've-caption');
      fig.append(win, this.ui.caption);
      return fig;
    }

    updateGeneral() {
      const ui = this.ui, i = this.info;
      const len = length(i.d);
      ui.hint.textContent = len ? `Click ▶ to play · ${len}` : 'Click ▶ to play';
      ui.state.textContent = { play: 'Playing', pause: 'Paused', idle: 'Stopped' }[this.state];
      ui.clock.textContent = `${clock(i.t)} / ${clock(i.d)}`;
      ui.sound.textContent = i.muted || i.volume === 0 ? 'Muted' : 'Stereo';
      ui.caption.textContent = [this.label || 'YouTube video', len].filter(Boolean).join(' · ');
      const p = this.progress();
      if (document.activeElement !== ui.seek) ui.seek.value = String(Math.round(p * 1000));
      ui.seek.style.setProperty('--p', `${(p * 100).toFixed(2)}%`);
      ui.play.classList.toggle('on', this.state === 'play');
      ui.pause.classList.toggle('on', this.state === 'pause');
      if (document.activeElement !== ui.volume) ui.volume.value = String(i.muted ? 0 : i.volume);
    }

    /* ---- MS-DOS: PLAY.EXE ---- */

    buildNight() {
      const fig = el('figure');
      const box = el('div', 've-box');
      const name = this.label.toUpperCase().replace(/[^A-Z0-9]/g, '').slice(0, 8) || 'YOUTUBE';
      const label = el('div', 've-label', `PLAY.EXE ─ ${name}`);
      const frame = el('span', 've-frame');
      frame.setAttribute('aria-hidden', 'true');
      frame.textContent = '┌────────────────┐\n│  ► PRESS ENTER │\n└────────────────┘';
      this.ui.info = el('span', 've-info');
      const screen = this.screen('', btn('', this.startLabel(), frame, this.ui.info));
      const status = el('div', 've-statusline');
      status.setAttribute('aria-hidden', 'true');
      this.ui.statusText = el('span');
      this.ui.bar = el('span', 've-bar');
      status.append(this.ui.statusText, this.ui.bar);
      const keys = el('div', 've-keys');
      const key = (k, text, fn) => {
        const b = btn('ve-key', `${k} ${text}`, el('b', null, k), el('span', null, text));
        b.addEventListener('click', fn);
        return b;
      };
      this.ui.toggle = key('ENTER', 'Play', () => this.toggle());
      this.ui.toggleText = this.ui.toggle.lastChild;
      this.ui.textKey = key('A', 'Text mode', () => this.setTextMode(!this.textMode));
      keys.append(this.ui.toggle, key('ESC', 'Stop', () => this.stop()), this.ui.textKey,
        key('F', 'Full screen', () => this.fullscreen(this.ui.screen)));
      box.append(label, screen, status, keys);
      fig.append(box);
      if (this.label) fig.append(el('figcaption', 've-caption', this.label));
      return fig;
    }

    updateNight() {
      const ui = this.ui, i = this.info;
      ui.info.textContent = `YOUTUBE · ${length(i.d) || '--:--'}`;
      const state = { play: '► PLAY ', pause: '‖ PAUSE', idle: '■ STOP ' }[this.state];
      ui.statusText.textContent = `${state}  ${clock(i.t)} / ${clock(i.d)}  `;
      const n = Math.round(this.progress() * 20);
      ui.bar.textContent = `[${'█'.repeat(n)}${'░'.repeat(20 - n)}]`;
      ui.toggleText.textContent = this.state === 'play' ? 'Pause' : 'Play';
      ui.toggle.setAttribute('aria-label', `ENTER ${ui.toggleText.textContent}`);
      ui.toggle.classList.toggle('on', this.state === 'play');
      ui.textKey.classList.toggle('on', this.textMode);
      ui.textKey.setAttribute('aria-pressed', String(this.textMode));
      ui.screen.classList.toggle('text', this.textMode);
    }

    setTextMode(on) {
      this.textMode = on;
      this.update();
    }

    /* Keys work while the focus is in the player. Nothing listens on the page, so
       the MS-DOS prompt keeps its keys. */
    onKey(e) {
      if (this.skin !== 'night' || e.altKey || e.ctrlKey || e.metaKey) return;
      const k = e.key.toLowerCase();
      // A focused key does its own job on Enter and Space.
      if ((k === 'enter' || k === ' ') && e.target.closest('button')) return;
      const run = fn => { e.preventDefault(); e.stopPropagation(); fn(); };
      if (k === 'enter') return run(() => this.toggle());
      if (k === 'escape') return run(() => this.stop());
      if (k === 'f') return run(() => this.fullscreen(this.ui.screen));
      if (k === 'a') return run(() => this.setTextMode(!this.textMode));
    }

    /* ---- Phone: the LCD player ---- */

    buildPhone() {
      const fig = el('figure');
      const head = el('div', 've-head');
      this.ui.mark = el('span');
      this.ui.len = el('span');
      head.append(this.ui.mark, this.ui.len);
      const box = el('span', 've-playbox');
      box.append(ico('lcdPlay'));
      const screen = this.screen('', btn('', this.startLabel(), box, el('span', 've-name', this.label || 'YouTube')));
      const blocks = el('div', 've-blocks');
      blocks.setAttribute('role', 'progressbar');
      blocks.setAttribute('aria-label', 'Progress');
      blocks.setAttribute('aria-valuemin', '0');
      blocks.setAttribute('aria-valuemax', '100');
      this.ui.blocks = Array.from({ length: 12 }, () => el('i'));
      blocks.append(...this.ui.blocks);
      this.ui.blockBar = blocks;
      const line = el('div', 've-line');
      this.ui.clock = el('span');
      this.ui.mode = el('span');
      line.append(this.ui.clock, this.ui.mode);
      const keys = el('div', 've-keys');
      const rew = btn('ve-key', `Back ${STEP} seconds`, ico('rew'), document.createTextNode(`${STEP}s`));
      rew.addEventListener('click', () => this.seekBy(-STEP));
      this.ui.toggle = btn('ve-key', null);
      this.ui.toggle.addEventListener('click', () => this.toggle());
      this.ui.lcd = btn('ve-key', 'LCD tint', document.createTextNode('LCD'));
      this.ui.lcd.addEventListener('click', () => { this.tint = !this.tint; this.update(); });
      // Full screen: the player frame, in full color.
      const full = btn('ve-key', 'Full screen', document.createTextNode('FULL'));
      full.addEventListener('click', () => this.fullscreen(this.frame));
      keys.append(rew, this.ui.toggle, this.ui.lcd, full);
      fig.append(head, screen, blocks, line, keys);
      return fig;
    }

    updatePhone() {
      const ui = this.ui, i = this.info;
      ui.mark.textContent = { play: '► VIDEO', pause: '❚❚ VIDEO', idle: '■ VIDEO' }[this.state];
      ui.len.textContent = length(i.d) || '--:--';
      const p = this.progress();
      const n = Math.round(p * 12);
      ui.blocks.forEach((b, k) => b.classList.toggle('on', k < n));
      ui.blockBar.setAttribute('aria-valuenow', String(Math.round(p * 100)));
      ui.clock.textContent = `${clock(i.t)} / ${clock(i.d)}`;
      ui.mode.textContent = { play: 'PLAY', pause: 'PAUSE', idle: 'STOP' }[this.state];
      ui.toggle.textContent = this.state === 'play' ? 'Pause' : 'Play';
      ui.toggle.classList.toggle('on', this.state === 'play');
      ui.lcd.classList.toggle('on', this.tint);
      ui.lcd.setAttribute('aria-pressed', String(this.tint));
      ui.screen.classList.toggle('tint', this.tint);
    }

    /* ---- The player frame ---- */

    progress() {
      const { t, d } = this.info;
      return d > 0 ? Math.min(1, Math.max(0, t / d)) : 0;
    }

    update() {
      if (!this.built) return;
      this.classList.toggle('ve-on', this.state === 'play');
      this.ui.start.hidden = this.state !== 'idle';
      if (this.skin === 'general') this.updateGeneral();
      else if (this.skin === 'night') this.updateNight();
      else this.updatePhone();
    }

    /* Adds the player frame. Its URL asks it to play at once, and the first
       "playVideo" after the handshake asks again. */
    load() {
      const f = document.createElement('iframe');
      f.className = 've-player';
      f.title = this.label || 'YouTube video';
      const params = new URLSearchParams({
        enablejsapi: '1', controls: '0', playsinline: '1', autoplay: '1', rel: '0', origin: location.origin,
      });
      f.src = `${YT_ORIGIN}/embed/${this.ytId}?${params}`;
      f.allow = 'autoplay; encrypted-media; fullscreen; picture-in-picture';
      f.allowFullscreen = true;
      f.referrerPolicy = 'strict-origin-when-cross-origin';
      f.setAttribute('sandbox', 'allow-scripts allow-same-origin allow-presentation allow-popups');
      f.addEventListener('load', () => this.handshake());
      this.frame = f;
      this.ui.screen.prepend(f);
    }

    /* Asks the player to send its events, until it answers. */
    handshake() {
      clearInterval(this.hello);
      let n = 0;
      const hello = () => {
        if (this.ready || ++n > HELLO_MAX) { clearInterval(this.hello); return; }
        this.post({ event: 'listening', id: this.uid, channel: 'widget' });
      };
      hello();
      this.hello = setInterval(hello, HELLO_MS);
    }

    post(msg) {
      this.frame?.contentWindow?.postMessage(JSON.stringify(msg), YT_ORIGIN);
    }

    command(func, args = []) {
      if (this.ready) this.post({ event: 'command', func, args, id: this.uid, channel: 'widget' });
    }

    /* A message from the player: onReady, infoDelivery, initialDelivery, or
       onStateChange. Only numbers are read from it. */
    onMessage(data) {
      let m = data;
      if (typeof m === 'string') {
        try { m = JSON.parse(m); } catch { return; }
      }
      if (!m || typeof m !== 'object') return;
      const first = !this.ready;
      if (['onReady', 'infoDelivery', 'initialDelivery'].includes(m.event)) this.ready = true;
      if (first && this.ready) {
        clearInterval(this.hello);
        if (this.wantPlay) this.command('playVideo');
      }
      let ps = null;
      if (m.event === 'onStateChange') ps = num(m.info);
      if ((m.event === 'infoDelivery' || m.event === 'initialDelivery') && m.info && typeof m.info === 'object') {
        const i = m.info;
        const t = num(i.currentTime), d = num(i.duration), v = num(i.volume);
        if (t !== null && this.state !== 'idle') this.info.t = Math.max(0, t);
        if (d !== null && d > 0) this.info.d = d;
        if (v !== null) this.info.volume = Math.max(0, Math.min(100, v));
        if (typeof i.muted === 'boolean') this.info.muted = i.muted;
        ps = num(i.playerState) ?? ps;
      }
      if (ps !== null) this.playerState(ps);
      this.update();
    }

    playerState(ps) {
      if (ps === PLAYING || ps === BUFFERING) {
        this.wantPlay = false;
        if (this.state !== 'idle') this.state = 'play';
      } else if (ps === PAUSED) {
        if (this.state !== 'idle') this.state = 'pause';
      } else if (ps === ENDED) {
        this.stop();
      }
    }

    /* ---- Controls ---- */

    /* The first click loads the frame and plays. */
    play() {
      this.state = this.state === 'play' ? 'play' : 'pause';
      this.wantPlay = true;
      if (!this.frame) this.load();
      else this.command('playVideo');
      this.update();
      if (this.ui.start.contains(document.activeElement)) this.ui.screen.focus({ preventScroll: true });
    }

    pause() { this.command('pauseVideo'); }

    toggle() {
      if (this.state === 'play') this.pause();
      else this.play();
    }

    stop() {
      this.wantPlay = false;
      this.command('pauseVideo');
      this.command('seekTo', [0, true]);
      this.state = 'idle';
      this.info.t = 0;
      this.update();
    }

    seekTo(t) {
      const d = this.info.d;
      if (!this.ready || !(d > 0)) return;
      const to = Math.max(0, Math.min(d, t));
      this.command('seekTo', [to, true]);
      this.info.t = to;
      if (this.state === 'idle') this.state = 'pause';
      this.update();
    }

    seekBy(dt) { this.seekTo(this.info.t + dt); }

    setVolume(v) {
      this.command('setVolume', [v]);
      this.command(v === 0 ? 'mute' : 'unMute');
      this.info.volume = v;
      this.info.muted = v === 0;
      this.update();
    }

    fullscreen(target) {
      if (document.fullscreenElement) { document.exitFullscreen().catch(() => {}); return; }
      target?.requestFullscreen?.().catch(() => {});
    }

    /* A mode change hides some views. An embed in a hidden view pauses. */
    modeChanged() {
      if (!this.getClientRects().length) this.pause();
    }
  });
})();
