// The loading bar and the graph's height, in the built app against a mocked Worker.

import { Page, expect, test } from '@playwright/test';
import { ARCHITECTURE, CAPABILITIES, mockApi } from './mock';

const SNAPSHOT = 'GET /api/repos/o/a/snapshot';

/** The requests made for `key` so far. */
const count = (requests: string[], key: string) => requests.filter((r) => r === key).length;

// req:web/loading-indicator@bc3e841a
test.describe('loading bar', () => {
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

  test('going online or offline is no navigation: it shows no bar', async ({ page, context }) => {
    await mockApi(context);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect(page.locator('#loadbar')).toBeHidden();
    // Read at once, before the redraw it starts could hide the bar again.
    const shown = await page.evaluate(() =>
      ['offline', 'online'].map((e) => {
        dispatchEvent(new Event(e));
        return !document.getElementById('loadbar')!.hidden;
      }),
    );
    expect(shown).toEqual([false, false]);
  });

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

// req:web/graph-fills-height@7acb0b89
test.describe('the graph fills the height', () => {
  test('desktop: at 1440 × 1000 the canvas ends within 32 px of the bottom, and the page does not scroll', async ({ page, context }) => {
    await mockApi(context);
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect.poll(() => edge(page, '.canvas')).toBeGreaterThanOrEqual(1000 - 32);
    expect(await edge(page, '.canvas')).toBeLessThanOrEqual(1000);
    expect(await page.evaluate(() => document.documentElement.scrollHeight <= window.innerHeight)).toBe(true);
    // The inspector scrolls within the canvas's height.
    const height = await page.locator('.canvas').evaluate((e) => e.getBoundingClientRect().height);
    expect(await page.locator('.insp').evaluate((e) => parseFloat(getComputedStyle(e).maxHeight))).toBe(height);
  });

  test('a short window: the canvas stays at least 320 px tall', async ({ page, context }) => {
    await mockApi(context);
    await page.setViewportSize({ width: 1440, height: 300 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect.poll(() => page.locator('.canvas').evaluate((e) => e.getBoundingClientRect().height)).toBe(320);
  });

  test.describe('phone', () => {
    test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

    test('the spec graph ends within 16 px of the tab bar', async ({ page, context }) => {
      await mockApi(context);
      await page.goto('/app/#/o/a/graph');
      await expect(page.locator('.n[data-node=b]')).toBeVisible();
      const tabbar = await edge(page, '.tabbar', true);
      await expect.poll(async () => tabbar - (await edge(page, '.canvas'))).toBeLessThanOrEqual(16);
      expect(await edge(page, '.canvas')).toBeLessThanOrEqual(tabbar);
    });

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
