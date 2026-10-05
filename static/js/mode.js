/* The view mode of the site (spec 11.1, 11.8). One localStorage key, logbook.mode:
   "general" is the Win98 site (light mode), "night" is MS-DOS mode (dark mode),
   and "phone" is phone mode. Only a phone can show phone mode.
   This script blocks the first paint and sets <html data-mode>, so a visitor never
   sees the wrong mode first. On a phone it also sets <html data-device="phone">,
   which shows the switches to phone mode. The mode lives in the browser, never in
   a cookie: public responses are the same for everyone. Storage can throw (private
   windows, blocked site data). Then a change lasts for this page only. */
const Mode = {
  KEY: 'logbook.mode',
  /* A phone: a touch screen 600 px wide or less, when the page loads. */
  phone: matchMedia('(pointer: coarse) and (max-width: 600px)').matches,
  /* The modes that this device can show. */
  allowed() { return this.phone ? ['phone', 'general', 'night'] : ['general', 'night']; },
  saved() {
    try { return localStorage.getItem(this.KEY); } catch { return null; }
  },
  /* The saved mode, if this device can show it. Else the default for the device. */
  initial() {
    const saved = this.saved();
    if (this.allowed().includes(saved)) return saved;
    return this.phone ? 'phone' : 'general';
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
if (Mode.phone) document.documentElement.dataset.device = 'phone';
