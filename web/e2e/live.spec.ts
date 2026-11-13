// The data the app holds and the refresh that keeps the view current, in the built app
// against a mocked Worker.

import { Page, WebSocketRoute, expect, test } from '@playwright/test';
import { APPROVE, REVIEW, mockApi } from './mock';
import { NOW, flight, implementation, review, snapshot } from '../src/views/fixtures';

const SNAPSHOT = 'GET /api/repos/o/a/snapshot';
const BRANCHES = 'GET /api/repos/o/a/branches';
const IMPL = 'GET /api/repos/o/a/implementation';

/** The requests made for `key` so far. */
const count = (requests: string[], key: string) => requests.filter((r) => r === key).length;

/** Makes the page hidden or visible, as a background tab is. */
async function visibility(page: Page, state: 'hidden' | 'visible') {
  await page.evaluate((s) => {
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => s });
    Object.defineProperty(document, 'hidden', { configurable: true, get: () => s === 'hidden' });
    document.dispatchEvent(new Event('visibilitychange'));
  }, state);
}

/** The repo's live socket, as the Worker's `Live` object answers it: `pong` to each `ping`
 * unless `silent`. */
async function liveSocket(page: Page, o: { silent?: boolean } = {}) {
  const sockets: WebSocketRoute[] = [];
  const closed: number[] = [];
  await page.routeWebSocket('**/api/repos/o/a/live', (ws) => {
    const n = sockets.push(ws) - 1;
    ws.onMessage((m) => {
      if (m === 'ping' && !o.silent) ws.send('pong');
    });
    ws.onClose(() => closed.push(n));
  });
  return { sockets, closed };
}

// req:web/notice-refresh@4f65ff27
test.describe('live updates', () => {
  test('a claim shows within seconds: a notice refreshes Build at once, with fresh', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    const live = await liveSocket(page);
    mock.routes[IMPL] = implementation({ flight: flight({ nodes: [{ ...flight().nodes[0], state: 'ready', claim: null }, flight().nodes[1]] }) });
    await page.goto('/app/#/o/a/build');
    await expect(page.locator('.fn[data-node=b]')).toContainText(/ready/i);
    await expect.poll(() => live.sockets.length).toBe(1);
    mock.routes[IMPL] = implementation();
    live.sockets[0].send(JSON.stringify({ delivery: 'd1' }));
    await expect(page.locator('.fn[data-node=b]')).toContainText(/claimed/i, { timeout: 2_000 });
    expect(mock.urls).toContain(`${IMPL}?fresh=d1`);
    expect(mock.urls).toContain(`${SNAPSHOT}?fresh=d1`);
  });

  test('hidden: the socket is closed while hidden, and a new one opened when shown', async ({ page, context }) => {
    await page.clock.install();
    await mockApi(context);
    const live = await liveSocket(page);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect.poll(() => live.sockets.length).toBe(1);
    await visibility(page, 'hidden');
    await expect.poll(() => live.closed).toEqual([0]);
    await page.clock.runFor(180_000);
    expect(live.sockets).toHaveLength(1);
    await visibility(page, 'visible');
    await expect.poll(() => live.sockets.length).toBe(2);
  });

  test('a silent drop: a socket that stops answering ping is closed and another opened', async ({ page, context }) => {
    await page.clock.install();
    await mockApi(context);
    const live = await liveSocket(page, { silent: true });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect.poll(() => live.sockets.length).toBe(1);
    await page.clock.runFor(61_000);
    await expect.poll(() => live.closed).toContain(0);
    await expect.poll(() => live.sockets.length).toBe(2);
  });

  test('a notice during an approval waits until it ends', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    const live = await liveSocket(page);
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await expect(page.locator('button.approve')).toBeEnabled();
    await expect.poll(() => live.sockets.length).toBe(1);
    mock.hold(APPROVE);
    await page.locator('button.approve').click();
    live.sockets[0].send(JSON.stringify({ delivery: 'd2' }));
    await page.waitForTimeout(300);
    expect(mock.urls.filter((u) => u.endsWith('fresh=d2'))).toEqual([]);
    mock.release(APPROVE);
    await expect(page.locator('button.approve')).toHaveText('Approved');
    await expect.poll(() => mock.urls.filter((u) => u.endsWith('fresh=d2')).length).toBeGreaterThan(0);
  });
});

// What a navigation draws from what the app already holds; the bar itself is in layout.spec.ts.
// req:web/loading-indicator@bc3e841a
test.describe('held data', () => {
  test("between views of a repo: the backlog is drawn from the snapshot already held when the Worker stops answering", async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    mock.hold();
    await page.locator('.sidebar .nv[data-view=backlog]').click();
    await expect(page.locator('.backlog .bi')).toHaveCount(3);
    await expect(page.locator('.sidebar .repo-meta')).toContainText('3 components');
    await expect(page.locator('#loadbar')).toBeHidden();
    mock.release();
  });

  test('the capabilities and architecture reports already held draw their views when the Worker stops answering', async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/architecture');
    await expect(page.locator('tr.cap[data-slug=e]')).toBeVisible();
    await page.locator('.sidebar .nv[data-view=capabilities]').click();
    await expect(page.locator('tr.cap[data-slug=b]')).toBeVisible();
    mock.hold();
    await page.locator('.sidebar .nv[data-view=architecture]').click();
    await expect(page.locator('tr.cap[data-slug=e]')).toBeVisible();
    await page.locator('.sidebar .nv[data-view=capabilities]').click();
    await expect(page.locator('tr.cap[data-slug=b]')).toBeVisible();
    await expect(page.locator('#loadbar')).toBeHidden();
    mock.release();
  });

  test('graph before implementation: the graph is drawn while the implementation state is pending', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.hold(IMPL);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect(page.locator('#loadbar')).toBeHidden();
    await page.locator('.n[data-node=b]').click();
    await expect(page.locator('.insp .work')).toContainText('Loading');
    mock.release(IMPL);
    await expect(page.locator('.insp .work')).toContainText('claimed');
  });

  test('a navigation asks for the repo list again while drawing from the held one', async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    const before = count(mock.requests, 'GET /api/repos');
    mock.routes['GET /api/repos'] = [
      { owner: 'o', name: 'a', default_branch: 'main' },
      { owner: 'p', name: 'q', default_branch: 'main' },
    ];
    await page.locator('.sidebar .nv[data-view=backlog]').click();
    await expect(page.locator('.backlog')).toBeVisible();
    await expect(page.locator('.sidebar select.repo option')).toHaveCount(2);
    expect(count(mock.requests, 'GET /api/repos')).toBe(before + 1);
  });
});

// req:web/live-refresh@28e7b29d
test.describe('live refresh', () => {
  test('new PR appears: listed without a reload, with the scroll position unchanged', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    const many = Array.from({ length: 40 }, (_, i) => ({ branch: `spec/c${i}`, head_sha: 'x', page: null }));
    mock.routes[BRANCHES] = many;
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/prs');
    await expect(page.locator('.prl[data-branch]')).toHaveCount(40);
    await page.evaluate(() => {
      window.scrollTo(0, document.body.scrollHeight);
      (window as unknown as { keep: number }).keep = 1;
    });
    const y = await page.evaluate(() => window.scrollY);
    expect(y).toBeGreaterThan(100);
    mock.routes[BRANCHES] = [...many, { branch: 'spec/the-new-one', head_sha: 'y', page: null }];
    await page.clock.runFor(60_000);
    await expect(page.locator('.prl[data-branch]')).toHaveCount(41);
    await expect(page.locator('.prl', { hasText: 'spec/the-new-one' })).toHaveCount(1);
    expect(Math.abs((await page.evaluate(() => window.scrollY)) - y)).toBeLessThanOrEqual(2);
    expect(await page.evaluate(() => (window as unknown as { keep: number }).keep)).toBe(1);
  });

  test("graph keeps its place: pan, zoom and the selected node stay, and b's card shows the new count", async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toContainText('0/2');
    await page.locator('.zoom button[aria-label="Zoom in"]').click();
    const canvas = (await page.locator('.canvas').boundingBox())!;
    await page.mouse.move(canvas.x + canvas.width - 30, canvas.y + canvas.height - 60);
    await page.mouse.down();
    await page.mouse.move(canvas.x + canvas.width - 80, canvas.y + canvas.height - 20, { steps: 4 });
    await page.mouse.up();
    await page.locator('.n[data-node=b]').click();
    await expect(page.locator('.n.sel')).toHaveAttribute('data-node', 'b');
    const transform = await page.locator('.stage').evaluate((e) => (e as HTMLElement).style.transform);
    expect(transform).not.toBe('');
    const snap = snapshot();
    snap.spec.nodes.find((n) => n.node === 'b')!.current = 1;
    mock.routes[SNAPSHOT] = snap;
    await page.clock.runFor(60_000);
    await expect(page.locator('.n[data-node=b]')).toContainText('1/2');
    await expect(page.locator('.n.sel')).toHaveAttribute('data-node', 'b');
    await expect(page.locator('.insp')).toContainText('The b contract.');
    expect(await page.locator('.stage').evaluate((e) => (e as HTMLElement).style.transform)).toBe(transform);
    // A selected node that is gone is deselected.
    const without = snapshot();
    without.spec.nodes = without.spec.nodes.filter((n) => n.node !== 'b');
    without.nodes = without.nodes.filter((n) => n.id !== 'b');
    mock.routes[SNAPSHOT] = without;
    await page.clock.runFor(60_000);
    await expect(page.locator('.n[data-node=b]')).toHaveCount(0);
    await expect(page.locator('.n.sel')).toHaveCount(0);
    await expect(page.locator('.insp.closed')).toHaveCount(1);
  });

  test('hidden tab: no refresh requests while hidden, and it asks again at once when visible', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await visibility(page, 'hidden');
    // Every refresh asks for the snapshot and the branches, so they count the refreshes. The
    // first load's other requests (the graph's implementation state, the Notify me switch's
    // key once the service worker is ready) can still arrive after the page is hidden under a
    // loaded machine's delays, and are no refresh.
    const refreshes = () => count(mock.requests, SNAPSHOT) + count(mock.requests, BRANCHES);
    const before = refreshes();
    const snapshots = count(mock.requests, SNAPSHOT);
    await page.clock.runFor(3 * 60_000);
    await page.waitForTimeout(200);
    expect(refreshes()).toBe(before);
    await visibility(page, 'visible');
    await expect.poll(() => count(mock.requests, SNAPSHOT)).toBeGreaterThan(snapshots);
  });

  test("nothing changed: the view's elements are not replaced, whatever the implementation state's now", async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/build');
    await expect(page.locator('.impl')).toBeVisible();
    await page.locator('.impl').evaluate((e) => ((e as HTMLElement).dataset.keep = '1'));
    mock.routes[IMPL] = implementation({ now: NOW + 60 });
    const before = count(mock.requests, IMPL);
    await page.clock.runFor(60_000);
    await expect.poll(() => count(mock.requests, IMPL)).toBe(before + 1);
    await page.waitForTimeout(200);
    await expect(page.locator('.impl[data-keep]')).toHaveCount(1);
  });

  test('the backlog filter and the full document’s place survive a refresh', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/backlog');
    await page.locator('.filter button[data-node=a]').click();
    await expect(page.locator('.backlog .bi')).toHaveCount(2);
    const snap = snapshot();
    snap.backlog.push({ slug: 'another', kind: 'bug', title: 'Another on a', nodes: ['a'], origin: null, blocks: [] });
    mock.routes[SNAPSHOT] = snap;
    await page.clock.runFor(60_000);
    await expect(page.locator('.backlog .bi')).toHaveCount(3);
    await expect(page.locator('.filter button.on')).toHaveText('a');

    await page.goto('/app/#/o/a/prs/spec%2Fc/full/specs/n/SPEC.md');
    await expect(page.locator('.pos')).toHaveText('Change 1 of 2');
    // The view scrolls to its first change on the next frame, which the clock drives here.
    await page.clock.runFor(50);
    await page.keyboard.press('j');
    await page.locator('.seg button', { hasText: 'After' }).click();
    await expect(page.locator('.pos')).toHaveText('Change 2 of 2');
    const pr = review();
    pr.change = { ...pr.change!, intent_section: 'Make b better, retitled.' };
    mock.routes[REVIEW] = pr;
    await page.clock.runFor(60_000);
    await expect(page.locator('.pos')).toHaveText('Change 2 of 2');
    await expect(page.locator('.chg.focus')).toContainText('It SHALL do s twice.');
    await expect(page.locator('.seg button.on')).toHaveText('After');
  });

  test('no refresh runs while an approval is in progress', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await expect(page.locator('button.approve')).toBeEnabled();
    mock.hold(APPROVE);
    await page.locator('button.approve').click();
    const before = count(mock.requests, SNAPSHOT);
    await page.clock.runFor(60_000);
    await page.waitForTimeout(200);
    expect(count(mock.requests, SNAPSHOT)).toBe(before);
    mock.release(APPROVE);
    await expect(page.locator('button.approve')).toHaveText('Approved');
    await page.clock.runFor(60_000);
    await expect.poll(() => count(mock.requests, SNAPSHOT)).toBe(before + 1);
  });

  test('Build keeps its switch: Blockers stays chosen while the state changes', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/build');
    await page.locator('.switch button', { hasText: 'Blockers' }).click();
    await expect(page.locator('.rt[data-root="backlog/setup"]')).toBeVisible();
    const impl = implementation({
      blockers: { view: 'blockers', roots: [{ root: 'a', kind: 'node', state: 'ready', waits_on_human: false, weight: 1, holds: [{ node: 'b', via: [] }] }] },
      flight: flight({
        nodes: [
          { ...flight().nodes[1], state: 'ready', blocked_by: [] },
          { ...flight().nodes[0], state: 'blocked', claim: null, blocked_by: [{ requirement: 'a/x', requirements: ['b/y'] }] },
        ],
      }),
    });
    mock.routes[IMPL] = impl;
    await page.clock.runFor(60_000);
    await expect(page.locator('.rt[data-root=a]')).toBeVisible();
    await expect(page.locator('.switch button.on')).toHaveText('Blockers');
    // A held node links to its entry in In flight.
    await page.locator('.held[data-node=b] a').click();
    await expect(page.locator('.switch button.on')).toHaveText('In flight');
    await expect(page.locator('.fn[data-node=b]')).toBeInViewport();
  });

  test('refresh fails: the graph is unchanged and no offline banner shows', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await page.locator('.canvas').evaluate((e) => ((e as HTMLElement).dataset.keep = '1'));
    mock.fail(SNAPSHOT, 502);
    const before = count(mock.requests, SNAPSHOT);
    await page.clock.runFor(60_000);
    await expect.poll(() => count(mock.requests, SNAPSHOT)).toBe(before + 1);
    await page.waitForTimeout(200);
    await expect(page.locator('.canvas[data-keep]')).toHaveCount(1);
    await expect(page.locator('.offline')).toHaveCount(0);
    // A network failure neither: the copy kept for offline use isn't shown in place of the view.
    mock.fail(SNAPSHOT, 'abort');
    await page.clock.runFor(60_000);
    await expect.poll(() => count(mock.requests, SNAPSHOT)).toBe(before + 2);
    await page.waitForTimeout(200);
    await expect(page.locator('.canvas[data-keep]')).toHaveCount(1);
    await expect(page.locator('.offline')).toHaveCount(0);
    // Signed out, though, shows the sign-in page.
    mock.fail(SNAPSHOT, 401);
    await page.clock.runFor(60_000);
    await expect(page.locator('.signin')).toBeVisible();
  });

});
