// The loading bar and the graph's height, in the built app against a mocked Worker.

import { Page, expect, test } from '@playwright/test';
import { ARCHITECTURE, CAPABILITIES, REVIEW, mockApi } from './mock';
import { capabilities, cost } from '../src/views/fixtures';

const SNAPSHOT = 'GET /api/repos/o/a/snapshot';

/** The requests made for `key` so far. */
const count = (requests: string[], key: string) => requests.filter((r) => r === key).length;

test.describe('loading bar', () => {
  // req:web/loading-indicator@bbcae7bd
  test('slow view: the bar shows from the first load while the answer is pending, and goes once the view is drawn', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.hold(SNAPSHOT);
    await page.goto('/app/#/o/a/graph');
    await expect.poll(() => count(mock.requests, SNAPSHOT)).toBe(1);
    await expect(page.locator('#loadbar')).toBeVisible();
    await expect(page.locator('.n[data-node=b]')).toHaveCount(0);
    mock.release(SNAPSHOT);
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect(page.locator('#loadbar')).toBeHidden();
  });

  // req:web/loading-indicator@bbcae7bd
  test('a navigation to Capabilities shows the bar until its report is drawn', async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect(page.locator('#loadbar')).toBeHidden();
    mock.hold(CAPABILITIES);
    await page.locator('.sidebar .nv[data-view=capabilities]').click();
    await expect.poll(() => count(mock.requests, CAPABILITIES)).toBe(1);
    await expect(page.locator('#loadbar')).toBeVisible();
    mock.release(CAPABILITIES);
    await expect(page.locator('table.caps tr.cap')).toHaveCount(2);
    await expect(page.locator('#loadbar')).toBeHidden();
  });

  // req:web/loading-indicator@bbcae7bd
  test('a navigation to Architecture shows the bar until its report is drawn', async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect(page.locator('#loadbar')).toBeHidden();
    mock.hold(ARCHITECTURE);
    await page.locator('.sidebar .nv[data-view=architecture]').click();
    await expect.poll(() => count(mock.requests, ARCHITECTURE)).toBe(1);
    await expect(page.locator('#loadbar')).toBeVisible();
    mock.release(ARCHITECTURE);
    await expect(page.locator('table.caps tr.cap')).toHaveCount(2);
    await expect(page.locator('#loadbar')).toBeHidden();
  });

  // req:web/loading-indicator@bbcae7bd
  test("a navigation to a PR shows the bar until its review is drawn", async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/prs');
    await expect(page.locator('.prl[data-branch]').first()).toBeVisible();
    await expect(page.locator('#loadbar')).toBeHidden();
    mock.hold(REVIEW);
    await page.evaluate(() => (location.hash = '#/o/a/prs/spec%2Fc'));
    await expect.poll(() => count(mock.requests, REVIEW)).toBe(1);
    await expect(page.locator('#loadbar')).toBeVisible();
    await expect(page.locator('button.approve')).toHaveCount(0);
    mock.release(REVIEW);
    await expect(page.locator('button.approve')).toBeVisible();
    await expect(page.locator('#loadbar')).toBeHidden();
  });

  // req:web/loading-indicator@bbcae7bd
  test('a redirect to the first repo keeps the bar until its view is drawn', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.hold(SNAPSHOT);
    await page.goto('/app/');
    await expect(page).toHaveURL(/#\/o\/a\/graph$/);
    await expect.poll(() => count(mock.requests, SNAPSHOT)).toBe(1);
    await expect(page.locator('#loadbar')).toBeVisible();
    mock.release(SNAPSHOT);
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect(page.locator('#loadbar')).toBeHidden();
  });

  // req:web/loading-indicator@bbcae7bd
  test('going online or offline is no navigation: it shows no bar', async ({ page, context }) => {
    await mockApi(context);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect(page.locator('#loadbar')).toBeHidden();
    // Read at once, before the redraw it starts could hide the bar again.
    for (const event of ['offline', 'online']) {
      await page.locator('.canvas').evaluate((e) => ((e as HTMLElement).dataset.keep = '1'));
      const shown = await page.evaluate((e) => {
        dispatchEvent(new Event(e));
        return !document.getElementById('loadbar')!.hidden;
      }, event);
      expect(shown).toBe(false);
      // The view is drawn again each time.
      await expect(page.locator('.canvas[data-keep]')).toHaveCount(0);
      await expect(page.locator('.n[data-node=b]')).toBeVisible();
      await expect(page.locator('#loadbar')).toBeHidden();
    }
  });

  // req:web/loading-indicator@bbcae7bd
  test('a background refresh shows no bar', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect(page.locator('#loadbar')).toBeHidden();
    mock.hold(SNAPSHOT);
    await page.clock.runFor(61_000);
    await expect.poll(() => count(mock.requests, SNAPSHOT)).toBe(2);
    await expect(page.locator('#loadbar')).toBeHidden();
    mock.release(SNAPSHOT);
  });
});

/** The bottom edge of `selector`'s first element, or its top with `top`. */
const edge = (page: Page, selector: string, top = false) =>
  page
    .locator(selector)
    .first()
    .evaluate((e, t) => {
      const r = e.getBoundingClientRect();
      return t ? r.top : r.bottom;
    }, top);

test.describe('the graph fills the height', () => {
  // req:web/graph-fills-height@f8bc8a07
  test('desktop: at 1440 × 1000 the canvas ends within 32 px of the bottom, and the page does not scroll', async ({ page, context }) => {
    await mockApi(context);
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect.poll(() => edge(page, '.canvas')).toBeGreaterThanOrEqual(1000 - 32);
    expect(await edge(page, '.canvas')).toBeLessThanOrEqual(1000);
    expect(await page.evaluate(() => document.documentElement.scrollHeight <= window.innerHeight)).toBe(true);
  });

  // req:web/graph-fills-height@f8bc8a07
  test('a short window: the canvas stays at least 320 px tall', async ({ page, context }) => {
    await mockApi(context);
    await page.setViewportSize({ width: 1440, height: 400 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect.poll(() => page.locator('.canvas').evaluate((e) => e.getBoundingClientRect().height)).toBe(320);
  });

  test.describe('phone', () => {
    test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

    // req:web/graph-fills-height@f8bc8a07
    test('the spec graph ends within 16 px of the tab bar', async ({ page, context }) => {
      await mockApi(context);
      await page.goto('/app/#/o/a/graph');
      await expect(page.locator('.n[data-node=b]')).toBeVisible();
      const tabbar = await edge(page, '.tabbar', true);
      await expect.poll(async () => tabbar - (await edge(page, '.canvas'))).toBeLessThanOrEqual(16);
      expect(await edge(page, '.canvas')).toBeLessThanOrEqual(tabbar);
    });

    // req:web/graph-fills-height@f8bc8a07
    test("a PR's Graph tab ends within 16 px of the Approve bar, as drawn with what it holds", async ({ page, context }) => {
      await mockApi(context);
      await page.goto('/app/#/o/a/prs/spec%2Fc/graph');
      await expect(page.locator('.n[data-node=b]')).toBeVisible();
      const fits = async () => {
        const bar = await edge(page, '.approve-bar', true);
        const bottom = await edge(page, '.canvas');
        return bottom <= bar && bar - bottom <= 16;
      };
      await expect.poll(fits).toBe(true);
      // Opening Request changes grows the Approve bar: the canvas still ends above it.
      await page.locator('.approve-bar button.request').tap();
      await expect(page.locator('.approve-bar textarea')).toBeVisible();
      await expect.poll(fits).toBe(true);
    });

    // req:web/graph-fills-height@f8bc8a07
    test("a PR's Graph tab drawn again after Approve is fitted again", async ({ page, context }) => {
      await mockApi(context);
      await page.goto('/app/#/o/a/prs/spec%2Fc/graph');
      await expect(page.locator('.n[data-node=b]')).toBeVisible();
      const gap = async () => (await edge(page, '.approve-bar', true)) - (await edge(page, '.canvas'));
      await expect.poll(gap).toBeLessThanOrEqual(16);
      const first = await page.locator('.canvas').elementHandle();
      await page.locator('.approve-bar button.approve').tap();
      // The graph is drawn anew with the review loaded again.
      await expect.poll(() => first!.evaluate((e) => e.isConnected)).toBe(false);
      await expect(page.locator('.n[data-node=b]')).toBeVisible();
      await expect.poll(gap).toBeLessThanOrEqual(16);
      expect(await gap()).toBeGreaterThanOrEqual(0);
    });
  });
});

test.describe('the inspector is an overlay', () => {
  // req:web/graph-in-the-browser@3a6ebaea
  test('desktop: the canvas keeps its width, the drawer overlaps it from the right edge, at most 640 px wide, and Escape closes it', async ({ page, context }) => {
    await mockApi(context);
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    const before = (await page.locator('.canvas').boundingBox())!;
    await page.locator('.n[data-node=b]').click();
    const drawer = page.locator('.insp:not(.closed)');
    await expect(drawer).toContainText('The b contract.');
    const after = (await page.locator('.canvas').boundingBox())!;
    expect(after.width).toBe(before.width);
    const box = (await drawer.boundingBox())!;
    expect(box.width).toBeLessThanOrEqual(Math.min(640, 0.9 * 1280));
    expect(Math.round(box.x + box.width)).toBe(1280);
    expect(box.x).toBeLessThan(after.x + after.width);
    expect(box.height).toBeLessThanOrEqual(900);
    await page.keyboard.press('Escape');
    await expect(page.locator('.insp.closed')).toHaveCount(1);
  });

  // req:web/graph-in-the-browser@3a6ebaea
  test('selecting the selected component again clears the selection, and the × button closes', async ({ page, context }) => {
    await mockApi(context);
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto('/app/#/o/a/graph');
    await page.locator('.n[data-node=b]').click();
    await expect(page.locator('.insp:not(.closed)')).toBeVisible();
    await expect(page.locator('.e.ein, .e.eout')).not.toHaveCount(0);
    await page.locator('.insp .close').click();
    await expect(page.locator('.insp.closed')).toHaveCount(1);
    await expect(page.locator('.n.sel')).toHaveCount(0);
    await expect(page.locator('.e.ein, .e.eout')).toHaveCount(0);
    await expect(page.locator('.insp.closed')).toHaveText('Select a component to see its contract, changes, work and backlog.');
    // With the drawer closed the card is tappable again; with it open, a click on the card still reaches it.
    await page.locator('.n[data-node=b]').click();
    await expect(page.locator('.insp:not(.closed)')).toBeVisible();
    await page.locator('.n[data-node=b]').dispatchEvent('click');
    await expect(page.locator('.insp.closed')).toHaveCount(1);
  });
});

test.describe('Markdown from the repository', () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

  test('a long code line and a wide table scroll within their boxes, in JetBrains Mono, and the page does not scroll sideways', async ({ page, context }) => {
    // req:web/markdown-styles@ef139cc3
    const mock = await mockApi(context);
    const row = (cells: string[]) => `| ${cells.join(' | ')} |`;
    const wide = ['alphabetical', 'bravo-column', 'charlie-column', 'delta-column', 'echo-column'];
    const text = ['### Capability: a', '', 'A person can a, with `inline` code.', '', '```', 'x'.repeat(300), '```', '', row(wide), row(wide.map(() => '---')), row(wide.map((w) => w.repeat(2))), '', '> quoted', '', '- one', '- two', '', '[a link](https://example.com/x)'].join('\n');
    const c = capabilities();
    c.report.capabilities[1] = cost('a', 2, { text });
    mock.routes[CAPABILITIES] = c;
    await page.goto('/app/#/o/a/capabilities/a');
    const pre = page.locator('.md pre').first();
    await expect(pre).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    // The code block is wider than its box and scrolls inside it.
    const box = await pre.evaluate((e) => ({ scroll: e.scrollWidth, client: e.clientWidth, overflowX: getComputedStyle(e).overflowX }));
    expect(box.scroll).toBeGreaterThan(box.client);
    expect(['auto', 'scroll']).toContain(box.overflowX);
    // The table is wider than its box and scrolls inside it.
    const table = await page.locator('.md table').first().evaluate((e) => ({ scroll: e.scrollWidth, client: e.clientWidth, overflowX: getComputedStyle(e).overflowX }));
    expect(table.scroll).toBeGreaterThan(table.client);
    expect(['auto', 'scroll']).toContain(table.overflowX);
    // Headings, lists, quotes and links carry their own styles.
    const style = (sel: string, prop: string) => page.locator(sel).first().evaluate((e, p) => getComputedStyle(e).getPropertyValue(p), prop);
    expect(await style('.md h3', 'font-weight')).toBe('600');
    // The values the app sets, not the browser's defaults (a list's 40 px, an unstyled link).
    expect(await style('.md ul', 'padding-left')).toBe('22px');
    expect(parseFloat(await style('.md blockquote', 'border-left-width'))).toBeGreaterThan(0);
    expect(await style('.md a', 'text-decoration-line')).toBe('underline');
    expect(await style('.md a', 'color')).not.toBe(await style('.md p', 'color'));
    // Table cells are bordered and its header shaded.
    expect(parseFloat(await style('.md td', 'border-left-width'))).toBeGreaterThan(0);
    expect(await style('.md th', 'background-color')).not.toBe('rgba(0, 0, 0, 0)');
    // Code is set in JetBrains Mono.
    expect(await page.locator('.md code').first().evaluate((e) => getComputedStyle(e).fontFamily)).toContain('JetBrains Mono');
    expect(await pre.evaluate((e) => getComputedStyle(e).fontFamily)).toContain('JetBrains Mono');
    expect(await pre.locator('code').evaluate((e) => getComputedStyle(e).fontFamily)).toContain('JetBrains Mono');
  });
});
