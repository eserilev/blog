/* <video-embed> (spec 4.10): one video in a post.

   The server renders a neutral element: data-kind="file" with data-src (an
   uploaded video), or data-kind="youtube" with data-id (an 11-byte YouTube ID),
   plus data-title. Without scripts, the element holds a plain <video controls>
   or a link to YouTube.

   This component gives the element the look of the view that holds it. The view
   follows <html data-mode>:
   - general: the Win98 page (also the editor, in every mode): a Media Player window.
   - night: the MS-DOS screen: PLAY.EXE, with live ASCII art of the frames.
   - phone: the phone screen: the LCD player.
   A mode change shows another view, with its own copy of the post. An embed that
   the change hides stops.

   Rules: all text goes in with textContent. Sizes go through CSSOM (CSP). A YouTube
   embed loads nothing from YouTube before the reader clicks play. The ASCII loop
   runs only while the video plays and the embed shows on screen. */
(() => {
  const SRC_RE = /^\/media\/[0-9a-f]{64}\.(mp4|webm)$/;
  const ID_RE = /^[A-Za-z0-9_-]{11}$/;
  const RAMP = ' .:-=+*#%@';
  const ASCII_FPS = 12;
  /* IBM Plex Mono: each character is 0.6 em wide. */
  const CHAR_W = 0.6;
  const SVG_NS = 'http://www.w3.org/2000/svg';
  const YT_ORIGIN = 'https://www.youtube-nocookie.com';
  const STEP = 10;

  const two = n => String(n).padStart(2, '0');
  /* "00:42", or "1:02:03" for an hour or more. */
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
  const bytes = n => (n >= 1048576 ? `${(n / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1024))} KB`);

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
  const icon = (w, h, paths, cls = 've-ico') => {
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
  const ico = (name, cls = 've-ico') => { const [w, h, p] = ICONS[name]; return icon(w, h, p, w > h ? `${cls} ve-wide` : cls); };

  /* Every connected embed, for the page-wide events below. */
  const live = new Set();
  document.addEventListener('visibilitychange', () => live.forEach(v => v.sync()));
  document.addEventListener('modechange', () => live.forEach(v => v.modeChanged()));
  document.addEventListener('fullscreenchange', () => live.forEach(v => v.layout()));

  customElements.define('video-embed', class extends HTMLElement {
    connectedCallback() {
      live.add(this);
      if (!this.built) this.build();
      this.observe();
    }

    disconnectedCallback() {
      live.delete(this);
      this.io?.disconnect();
      this.ro?.disconnect();
      this.io = this.ro = null;
      this.visible = false;
      this.video?.pause();
      this.unloadYouTube();
      this.sync();
    }

    /* ---- Set up ---- */

    build() {
      const kind = this.dataset.kind;
      const src = this.dataset.src || '';
      const id = this.dataset.id || '';
      // The server checks these values (theorem T18). The component checks them again.
      if (kind === 'file' && SRC_RE.test(src)) this.kind = 'file';
      else if (kind === 'youtube' && ID_RE.test(id)) this.kind = 'youtube';
      else return;
      this.built = true;
      this.src = src;
      this.ytId = id;
      this.label = (this.dataset.title || '').trim();
      this.ext = this.kind === 'file' ? src.split('.').pop() : '';
      this.file = this.kind === 'file' ? `${src.slice(7, 15)}.${this.ext}` : 'YouTube';
      this.state = 'idle';
      this.ascii = true;
      this.tint = true;
      this.visible = false;
      this.raf = 0;
      this.lastDraw = 0;
      if (this.kind === 'file') {
        this.video = this.querySelector('video') || el('video');
        this.video.removeAttribute('controls');
        this.video.preload = 'metadata';
        this.video.playsInline = true;
        this.video.src = src;
        if (this.label) this.video.setAttribute('aria-label', this.label);
        this.listen();
      }
      this.skin = this.closest('dos-shell') ? 'night' : this.closest('phone-shell') ? 'phone' : 'general';
      this.ui = {};
      const figure = { general: () => this.buildGeneral(), night: () => this.buildNight(), phone: () => this.buildPhone() }[this.skin]();
      figure.classList.add('ve', `ve-${this.skin}`, `ve-${this.kind}`);
      this.replaceChildren(figure);
      this.addEventListener('keydown', e => this.onKey(e));
      this.update();
    }

    listen() {
      const v = this.video;
      v.addEventListener('play', () => { this.state = 'play'; this.update(); this.sync(); });
      v.addEventListener('pause', () => {
        if (this.state !== 'idle') this.state = 'pause';
        this.update();
        this.sync();
        this.drawAscii();
      });
      v.addEventListener('ended', () => this.stop());
      v.addEventListener('timeupdate', () => this.update());
      v.addEventListener('durationchange', () => this.update());
      v.addEventListener('loadedmetadata', () => { this.update(); this.layout(); });
      v.addEventListener('volumechange', () => this.update());
      v.addEventListener('seeked', () => { if (v.paused) this.drawAscii(); });
      v.addEventListener('error', () => { this.state = 'error'; this.update(); });
    }

    /* Watches the size of the screen and whether the embed shows. */
    observe() {
      if (!this.built || this.io) return;
      this.io = new IntersectionObserver(entries => {
        this.visible = entries.some(e => e.isIntersecting);
        this.sync();
      }, { rootMargin: '100px' });
      this.io.observe(this);
      if (this.ui.screen && 'ResizeObserver' in window) {
        this.ro = new ResizeObserver(() => this.layout());
        this.ro.observe(this.ui.screen);
      }
    }

    /* The screen: the video, the pre-play button, and (MS-DOS) the ASCII art. */
    screen(cls, start) {
      const s = el('div', `ve-screen ${cls || ''}`.trim());
      // MS-DOS: the screen takes the focus, so its keys work after the start
      // button goes. Other skins: the focus moves here from the start button.
      s.tabIndex = this.skin === 'night' ? 0 : -1;
      if (this.skin === 'night') {
        s.setAttribute('role', 'group');
        s.setAttribute('aria-label', `Video player${this.label ? `: ${this.label}` : ''}. Keys: Enter play or pause, Escape stop, F full screen${this.kind === 'file' ? ', A ASCII' : ''}.`);
      }
      if (this.video) s.append(this.video);
      start.classList.add('ve-start');
      start.addEventListener('click', () => this.play());
      s.append(start);
      this.ui.screen = s;
      this.ui.start = start;
      return s;
    }

    startLabel() {
      return this.kind === 'youtube'
        ? `Play YouTube video${this.label ? `: ${this.label}` : ''}. It loads from YouTube.`
        : `Play video${this.label ? `: ${this.label}` : ''}`;
    }

    /* ---- Win98: a Media Player window ---- */

    buildGeneral() {
      const fig = el('figure');
      const win = el('div', 've-win raised');
      const bar = el('div', 'titlebar ve-titlebar');
      const name = el('span', 'ttl', `Media Player - ${this.label || this.file}`);
      const ctl = el('span', 've-ctl');
      ctl.setAttribute('aria-hidden', 'true');
      ['_', '□', '×'].forEach(c => ctl.append(el('span', 'ctl raised', c)));
      bar.append(ico('reel', 've-ico ve-reel'), name, ctl);
      const menu = el('div', 'menubar ve-menu');
      menu.setAttribute('aria-hidden', 'true');
      ['File', 'View', 'Play', 'Favorites', 'Go', 'Help'].forEach(m => menu.append(el('span', null, m)));
      const start = btn('', this.startLabel(), ico('big', 've-big'), this.ui.hint = el('span', 've-hint'));
      const screen = this.screen('sunken', start);
      screen.addEventListener('dblclick', e => { if (e.target === this.video) this.fullscreen(this.video); });
      win.append(bar, menu, screen);

      if (this.kind === 'file') {
        const seek = el('input', 've-seek');
        seek.type = 'range';
        seek.min = '0';
        seek.max = '1000';
        seek.step = '1';
        seek.value = '0';
        seek.setAttribute('aria-label', 'Seek');
        seek.addEventListener('input', () => this.seekTo((seek.value / 1000) * (this.video.duration || 0)));
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
        vol.max = '1';
        vol.step = '0.05';
        vol.value = String(this.video.volume);
        vol.setAttribute('aria-label', 'Volume');
        vol.addEventListener('input', () => { this.video.volume = Number(vol.value); this.video.muted = vol.value === '0'; });
        this.ui.volume = vol;
        row.append(
          this.ui.play, this.ui.pause, b('stop', 'Stop', () => this.stop()), sep,
          // One video has no previous or next track: these go to the start and the end.
          b('prev', 'Go to the start', () => this.seekTo(0)),
          b('rew', `Back ${STEP} seconds`, () => this.seekBy(-STEP)),
          b('ff', `Forward ${STEP} seconds`, () => this.seekBy(STEP)),
          b('next', 'Go to the end', () => this.seekTo(this.video.duration)),
          el('span', 've-grow'), ico('volume', 've-ico ve-vol-ico'), vol,
        );
        win.append(track, row);
      }
      const status = el('div', 'status ve-status');
      this.ui.state = el('span', 'sunken ve-state');
      this.ui.state.setAttribute('aria-live', 'polite');
      this.ui.clock = el('span', 'sunken ve-clock');
      this.ui.sound = el('span', 'sunken ve-sound');
      status.append(this.ui.state, this.ui.clock, this.ui.sound);
      win.append(status);
      this.ui.caption = el('figcaption', 've-caption');
      fig.append(win, this.ui.caption);
      return fig;
    }

    updateGeneral() {
      const ui = this.ui, v = this.video;
      const len = length(v?.duration);
      if (this.kind === 'youtube') {
        ui.hint.textContent = 'Click ▶ to play · YouTube';
        ui.state.textContent = this.state === 'youtube' ? 'Playing' : 'Stopped';
        ui.clock.textContent = 'YouTube';
        ui.sound.textContent = 'Stereo';
        ui.caption.textContent = this.label || 'YouTube video';
        return;
      }
      ui.hint.textContent = len ? `Click ▶ to play · ${len}` : 'Click ▶ to play';
      ui.state.textContent = { play: 'Playing', pause: 'Paused', idle: 'Stopped', error: 'Cannot play this file' }[this.state];
      ui.clock.textContent = `${clock(v.currentTime)} / ${clock(v.duration)}`;
      ui.sound.textContent = v.muted || v.volume === 0 ? 'Muted' : 'Stereo';
      ui.caption.textContent = [this.label || this.file, len].filter(Boolean).join(' · ');
      const p = this.progress();
      if (document.activeElement !== ui.seek) ui.seek.value = String(Math.round(p * 1000));
      ui.seek.style.setProperty('--p', `${(p * 100).toFixed(2)}%`);
      ui.play.classList.toggle('on', this.state === 'play');
      ui.pause.classList.toggle('on', this.state === 'pause');
      if (document.activeElement !== ui.volume) ui.volume.value = String(v.muted ? 0 : v.volume);
    }

    /* ---- MS-DOS: PLAY.EXE ---- */

    dosName() {
      const base = (this.label || this.src.slice(7, 15)).toUpperCase().replace(/[^A-Z0-9]/g, '').slice(0, 8) || 'VIDEO';
      return this.kind === 'youtube' ? 'YOUTUBE' : `${base}.${this.ext.toUpperCase()}`;
    }

    buildNight() {
      const fig = el('figure');
      const box = el('div', 've-box');
      const label = el('div', 've-label', `PLAY.EXE ─ ${this.dosName()}`);
      const start = btn('', this.startLabel());
      const frame = el('span', 've-frame');
      frame.setAttribute('aria-hidden', 'true');
      frame.textContent = '┌────────────────┐\n│  ► PRESS ENTER │\n└────────────────┘';
      this.ui.info = el('span', 've-info');
      start.append(frame, this.ui.info);
      const screen = this.screen('', start);
      if (this.kind === 'file') {
        this.ui.ascii = el('pre', 've-ascii');
        this.ui.ascii.setAttribute('aria-hidden', 'true');
        screen.insertBefore(this.ui.ascii, start);
      }
      box.append(label, screen);

      const keys = el('div', 've-keys');
      const key = (k, text, fn) => {
        const b = btn('ve-key', `${k} ${text}`, el('b', null, k), el('span', null, text));
        b.addEventListener('click', fn);
        return b;
      };
      this.ui.toggle = key('ENTER', 'Play', () => this.toggle());
      this.ui.toggleText = this.ui.toggle.lastChild;
      const stop = key('ESC', 'Stop', () => this.stop());
      const full = key('F', 'Full screen', () => this.fullscreen(this.ui.screen));
      if (this.kind === 'file') {
        this.ui.status = el('div', 've-statusline');
        this.ui.status.setAttribute('aria-hidden', 'true');
        this.ui.bar = el('span', 've-bar');
        this.ui.statusText = el('span');
        this.ui.status.append(this.ui.statusText, this.ui.bar);
        this.ui.asciiKey = key('A', 'ASCII', () => this.setAscii(!this.ascii));
        keys.append(this.ui.toggle, stop, this.ui.asciiKey, full);
        box.append(this.ui.status, keys);
      } else {
        keys.append(this.ui.toggle, stop, full);
        box.append(el('div', 've-note', 'ASCII needs a local file'), keys);
      }
      fig.append(box);
      if (this.label) fig.append(el('figcaption', 've-caption', this.label));
      // The size of the file, for the PRESS ENTER box. Same origin, no body.
      if (this.kind === 'file') {
        fetch(this.src, { method: 'HEAD' }).then(r => {
          const n = Number(r.headers.get('content-length'));
          if (r.ok && n > 0) { this.size = n; this.update(); }
        }).catch(() => {});
      }
      return fig;
    }

    updateNight() {
      const ui = this.ui, v = this.video;
      if (this.kind === 'youtube') {
        ui.info.textContent = 'YOUTUBE · LOADS ON PLAY';
        ui.toggleText.textContent = this.state === 'youtube' ? 'Stop' : 'Play';
        ui.toggle.setAttribute('aria-label', `ENTER ${ui.toggleText.textContent}`);
        return;
      }
      const res = v.videoWidth ? `${v.videoWidth}x${v.videoHeight}` : '';
      ui.info.textContent = [length(v.duration), res, this.size ? bytes(this.size) : ''].filter(Boolean).join(' · ');
      const state = { play: '► PLAY ', pause: '‖ PAUSE', idle: '■ STOP ', error: 'X ERROR' }[this.state];
      ui.statusText.textContent = `${state}  ${clock(v.currentTime)} / ${clock(v.duration)}  `;
      const n = Math.round(this.progress() * 20);
      ui.bar.textContent = `[${'█'.repeat(n)}${'░'.repeat(20 - n)}]`;
      ui.toggleText.textContent = this.state === 'play' ? 'Pause' : 'Play';
      ui.toggle.setAttribute('aria-label', `ENTER ${ui.toggleText.textContent}`);
      ui.toggle.classList.toggle('on', this.state === 'play');
      ui.asciiKey.classList.toggle('on', this.ascii);
      ui.asciiKey.setAttribute('aria-pressed', String(this.ascii));
      ui.screen.classList.toggle('ascii', this.ascii);
    }

    setAscii(on) {
      this.ascii = on;
      if (!on) this.ui.ascii.textContent = '';
      this.update();
      this.sync();
      if (on && this.video.paused) this.drawAscii();
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
      if (k === 'a' && this.kind === 'file') return run(() => this.setAscii(!this.ascii));
    }

    /* ---- Phone: the LCD player ---- */

    buildPhone() {
      const fig = el('figure');
      const head = el('div', 've-head');
      this.ui.mark = el('span');
      this.ui.len = el('span');
      head.append(this.ui.mark, this.ui.len);
      const box = el('span', 've-playbox');
      box.append(ico('lcdPlay', 've-ico'));
      const start = btn('', this.startLabel(), box, el('span', 've-name', this.label || this.file));
      const screen = this.screen('', start);
      fig.append(head, screen);
      if (this.kind === 'file') {
        // A tap on the picture: the native full screen, in full color.
        screen.addEventListener('click', e => { if (e.target === this.video) this.fullscreen(this.video); });
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
        const rew = btn('ve-key', `Back ${STEP} seconds`, ico('rew', 've-ico'), document.createTextNode(`${STEP}s`));
        rew.addEventListener('click', () => this.seekBy(-STEP));
        this.ui.toggle = btn('ve-key', null);
        this.ui.toggle.addEventListener('click', () => this.toggle());
        this.ui.lcd = btn('ve-key', 'LCD tint', document.createTextNode('LCD'));
        this.ui.lcd.addEventListener('click', () => { this.tint = !this.tint; this.update(); });
        keys.append(rew, this.ui.toggle, this.ui.lcd);
        fig.append(blocks, line, keys);
      }
      return fig;
    }

    updatePhone() {
      const ui = this.ui, v = this.video;
      if (this.kind === 'youtube') {
        ui.mark.textContent = this.state === 'youtube' ? '► VIDEO' : '■ VIDEO';
        ui.len.textContent = 'YOUTUBE';
        return;
      }
      ui.mark.textContent = { play: '► VIDEO', pause: '❚❚ VIDEO', idle: '■ VIDEO', error: 'X VIDEO' }[this.state];
      ui.len.textContent = length(v.duration);
      const p = this.progress();
      const n = Math.round(p * 12);
      ui.blocks.forEach((b, i) => b.classList.toggle('on', i < n));
      ui.blockBar.setAttribute('aria-valuenow', String(Math.round(p * 100)));
      ui.clock.textContent = `${clock(v.currentTime)} / ${clock(v.duration)}`;
      ui.mode.textContent = { play: 'PLAY', pause: 'PAUSE', idle: 'STOP', error: 'ERROR' }[this.state];
      ui.toggle.textContent = this.state === 'play' ? 'Pause' : 'Play';
      ui.toggle.classList.toggle('on', this.state === 'play');
      ui.lcd.classList.toggle('on', this.tint);
      ui.lcd.setAttribute('aria-pressed', String(this.tint));
      ui.screen.classList.toggle('tint', this.tint);
    }

    /* ---- Playback ---- */

    progress() {
      const v = this.video;
      return v && v.duration > 0 ? Math.min(1, v.currentTime / v.duration) : 0;
    }

    update() {
      if (!this.built) return;
      const playing = this.state !== 'idle' && this.state !== 'error';
      this.classList.toggle('ve-on', playing);
      this.ui.start.hidden = playing || this.state === 'error';
      if (this.skin === 'general') this.updateGeneral();
      else if (this.skin === 'night') this.updateNight();
      else this.updatePhone();
    }

    play() {
      if (this.kind === 'youtube') { this.loadYouTube(); return; }
      if (this.state === 'idle') this.state = 'pause';
      this.update();
      this.video.play().catch(() => { if (this.video.paused) { this.state = 'pause'; this.update(); } });
      if (this.ui.start.contains(document.activeElement)) this.ui.screen.focus({ preventScroll: true });
    }

    pause() { this.video?.pause(); }

    toggle() {
      if (this.kind === 'youtube') { if (this.state === 'youtube') this.stop(); else this.play(); return; }
      if (this.video.paused) this.play(); else this.pause();
    }

    stop() {
      if (this.kind === 'youtube') {
        this.unloadYouTube();
      } else {
        this.state = 'idle';
        this.video.pause();
        if (this.video.readyState > 0) this.video.currentTime = 0;
        if (this.ui.ascii) this.ui.ascii.textContent = '';
      }
      this.update();
      this.sync();
    }

    seekTo(t) {
      const v = this.video;
      if (!v || !Number.isFinite(v.duration)) return;
      v.currentTime = Math.max(0, Math.min(v.duration, t));
      if (this.state === 'idle') this.state = 'pause';
      this.update();
    }

    seekBy(dt) { if (this.video) this.seekTo(this.video.currentTime + dt); }

    fullscreen(target) {
      if (document.fullscreenElement) { document.exitFullscreen().catch(() => {}); return; }
      const native = () => this.video?.webkitEnterFullscreen?.();
      if (target?.requestFullscreen) target.requestFullscreen().catch(native);
      else native();
    }

    /* ---- YouTube: a facade until the click ---- */

    loadYouTube() {
      if (this.ui.frame) return;
      const f = document.createElement('iframe');
      f.className = 've-frame-yt';
      f.title = this.label || 'YouTube video';
      f.src = `${YT_ORIGIN}/embed/${this.ytId}?autoplay=1`;
      f.allow = 'autoplay; encrypted-media; fullscreen; picture-in-picture';
      f.allowFullscreen = true;
      f.referrerPolicy = 'strict-origin-when-cross-origin';
      f.setAttribute('sandbox', 'allow-scripts allow-same-origin allow-presentation allow-popups');
      this.ui.frame = f;
      this.ui.screen.append(f);
      this.state = 'youtube';
      this.update();
      f.focus({ preventScroll: true });
    }

    unloadYouTube() {
      if (!this.ui?.frame) return;
      this.ui.frame.remove();
      this.ui.frame = null;
      this.state = 'idle';
      this.update();
    }

    /* A mode change hides some views. An embed in a hidden view stops. */
    modeChanged() {
      if (this.getClientRects().length) return;
      this.video?.pause();
      this.unloadYouTube();
    }

    /* ---- ASCII art (MS-DOS) ---- */

    /* The character grid for the size of the screen. */
    layout() {
      const pre = this.ui?.ascii, s = this.ui?.screen;
      if (!pre || !s) return;
      const w = s.clientWidth, h = s.clientHeight;
      if (!w || !h) return;
      const cols = Math.max(24, Math.min(160, Math.round(w / 6.6)));
      const cellW = w / cols, font = cellW / CHAR_W;
      const rows = Math.max(8, Math.floor(h / font));
      this.grid = { cols, rows, w, h, cellW, cellH: font };
      pre.style.fontSize = `${font}px`;
      pre.style.lineHeight = `${font}px`;
      if (this.video?.paused) this.drawAscii();
    }

    asciiRuns() {
      return this.skin === 'night' && this.kind === 'file' && this.ascii && this.visible &&
        !document.hidden && this.isConnected && this.video && !this.video.paused;
    }

    /* Starts or stops the ASCII loop to match the state. */
    sync() {
      if (!this.asciiRuns()) {
        if (this.raf) cancelAnimationFrame(this.raf);
        this.raf = 0;
        return;
      }
      if (this.raf) return;
      const tick = ts => {
        this.raf = 0;
        if (!this.asciiRuns()) return;
        if (ts - this.lastDraw >= 1000 / ASCII_FPS) {
          this.lastDraw = ts;
          this.drawAscii();
        }
        this.raf = requestAnimationFrame(tick);
      };
      this.raf = requestAnimationFrame(tick);
    }

    /* One frame as characters: the light of each cell picks a character of the ramp.
       The picture keeps its shape: the cells around it stay blank. */
    drawAscii() {
      const v = this.video, pre = this.ui?.ascii;
      if (!pre || !this.ascii || this.state === 'idle' || !v || v.readyState < 2 || !v.videoWidth) return;
      if (!this.grid) this.layout();
      const g = this.grid;
      if (!g) return;
      if (!this.canvas) {
        this.canvas = document.createElement('canvas');
        this.ctx = this.canvas.getContext('2d', { willReadFrequently: true });
      }
      const c = this.canvas, ctx = this.ctx;
      if (c.width !== g.cols || c.height !== g.rows) { c.width = g.cols; c.height = g.rows; }
      const scale = Math.min(g.w / v.videoWidth, g.h / v.videoHeight);
      const dw = (v.videoWidth * scale) / g.cellW, dh = (v.videoHeight * scale) / g.cellH;
      ctx.fillStyle = '#000';
      ctx.fillRect(0, 0, g.cols, g.rows);
      ctx.drawImage(v, (g.cols - dw) / 2, (g.rows - dh) / 2, dw, dh);
      const d = ctx.getImageData(0, 0, g.cols, g.rows).data;
      const lines = new Array(g.rows);
      for (let y = 0; y < g.rows; y++) {
        let line = '';
        for (let x = 0; x < g.cols; x++) {
          const i = (y * g.cols + x) * 4;
          const l = (d[i] * 299 + d[i + 1] * 587 + d[i + 2] * 114) / 255000;
          line += RAMP[Math.min(9, Math.floor(l * 10))];
        }
        lines[y] = line;
      }
      pre.textContent = lines.join('\n');
    }
  });
})();
