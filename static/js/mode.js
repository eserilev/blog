/* The view mode of the site (spec 11.1). One localStorage key, logbook.mode:
   "general" is the Win98 site (light mode), "night" is MS-DOS mode (dark mode).
   This script blocks the first paint and sets <html data-mode>, so a visitor never
   sees the wrong mode first. The mode lives in the browser, never in a cookie:
   public responses are the same for everyone. Storage can throw (private windows,
   blocked site data). Then a change lasts for this page only. */
const Mode = {
  KEY: 'logbook.mode',
  /* The modes that this device can show. */
  allowed() { return ['general', 'night']; },
  saved() {
    try { return localStorage.getItem(this.KEY); } catch { return null; }
  },
  /* The saved mode, if this device can show it. Else the default. */
  initial() {
    const saved = this.saved();
    return this.allowed().includes(saved) ? saved : 'general';
  },
  get current() { return document.documentElement.dataset.mode; },
  set(mode) {
    if (!this.allowed().includes(mode)) return;
    try { localStorage.setItem(this.KEY, mode); } catch { /* This page only. */ }
    document.documentElement.dataset.mode = mode;
    document.dispatchEvent(new CustomEvent('modechange', { detail: { mode } }));
  },
};
document.documentElement.dataset.mode = Mode.initial();
