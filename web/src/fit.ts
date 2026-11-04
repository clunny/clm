// The graph's canvas runs from its top to the bottom of the window, or below 900 px to the
// top of the Approve bar or the tab bar, at least 320 px tall; on desktop the inspector
// scrolls within that height (web/graph-fills-height).

/** The shortest the canvas gets. */
const MIN = 320;
/** What is left below the canvas: `.main`'s bottom padding on desktop, so the page doesn't
 * scroll, and a small gap above the bar on phones. */
const DESKTOP_GAP = 22;
const PHONE_GAP = 8;

/** The canvas's height when its top is at `top` and it may run down to `limit`. */
export function canvasHeight(top: number, limit: number): number {
  return Math.max(MIN, Math.floor(limit - top));
}

/** Keeps whatever graph is drawn in `root` fitted, from now on: after every change to what
 * `root` holds (a view drawn, a graph redrawn in place, the Approve bar's form opened, a note
 * above the canvas), on a resize and once the fonts have loaded. The fit only sets a style, which
 * the observer doesn't watch, so it never feeds itself. */
export function fitGraphs(root: HTMLElement) {
  const narrow = matchMedia('(max-width: 899.98px)');
  const fit = () => {
    const wrap = root.querySelector<HTMLElement>('.gwrap');
    const canvas = wrap?.querySelector<HTMLElement>('.canvas');
    if (!wrap || !canvas) return;
    // The Approve bar is fixed above the tab bar on phones, and drawn inline on desktop.
    const bar = narrow.matches ? (root.querySelector('.approve-bar') ?? root.querySelector('.tabbar')) : null;
    const limit = bar ? bar.getBoundingClientRect().top - PHONE_GAP : innerHeight - DESKTOP_GAP;
    // Measured from the page's top, so the fit doesn't depend on how far it is scrolled.
    const h = canvasHeight(canvas.getBoundingClientRect().top + scrollY, limit);
    wrap.style.setProperty('--graph-h', `${h}px`);
  };
  let queued = false;
  const later = () => {
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      fit();
    });
  };
  new MutationObserver(later).observe(root, { childList: true, subtree: true, characterData: true });
  addEventListener('resize', later);
  void document.fonts?.ready.then(later);
  return fit;
}
