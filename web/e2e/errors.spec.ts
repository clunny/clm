// What the app says when the Worker can't give a repo's view, in the built app against a
// mocked Worker.

import { expect, test } from '@playwright/test';
import { mockApi } from './mock';

const SNAPSHOT = 'GET /api/repos/o/a/snapshot';

test.describe('honest errors', () => {
  // req:web/honest-errors@165e071b
  test('busy, nothing kept: says CLM is retrying, nothing about access, then draws without a reload', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    mock.fail(SNAPSHOT, 503, { error: 'unavailable' });
    // When the page asked, by its own clock.
    await page.addInitScript(() => {
      const asks: number[] = [];
      (window as unknown as { asks: number[] }).asks = asks;
      const original = window.fetch;
      window.fetch = (input, init) => {
        if (String(input instanceof Request ? input.url : input).includes('/snapshot')) asks.push(Date.now());
        return original(input, init);
      };
    });
    await page.goto('/app/#/o/a/graph');
    const card = page.locator('.signin');
    await expect(card).toContainText("CLM couldn't load the repo just now and is retrying.");
    await expect(card).not.toContainText('GitHub account');
    // The app arms its next ask only once it has handled the last response, which comes after
    // the mock counts the request, so one jump of the clock can pass before the timer exists:
    // keep advancing. The clock also runs on its own, so load can only widen the gaps between
    // asks: at least 5 seconds, then at least 10 more, holds however busy the machine is.
    const asked = () => mock.requests.filter((r) => r === SNAPSHOT).length;
    for (const times of [2, 3]) {
      await expect(async () => {
        await page.clock.runFor(2_000);
        expect(asked()).toBeGreaterThanOrEqual(times);
      }).toPass({ timeout: 15_000 });
    }
    const asks = await page.evaluate(() => (window as unknown as { asks: number[] }).asks);
    expect(asks[1] - asks[0]).toBeGreaterThanOrEqual(5_000);
    expect(asks[2] - asks[1]).toBeGreaterThanOrEqual(10_000);
    mock.fail(SNAPSHOT, null);
    await expect(async () => {
      await page.clock.runFor(5_000);
      await expect(page.locator('.n[data-node=b]')).toBeVisible({ timeout: 500 });
    }).toPass({ timeout: 20_000 });
    await expect(page.locator('.signin')).toHaveCount(0);
  });

  // req:web/honest-errors@165e071b
  test('busy, kept copy: draws the copy kept an hour ago under CLM couldn\'t be reached', async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    // The copy kept for offline use, as it would be had it been kept an hour ago.
    await page.evaluate(async () => {
      const cache = await caches.open('clm-api');
      const path = '/api/repos/o/a/snapshot';
      const copy = (await cache.match(path))!;
      const hourAgo = String(Date.now() - 3_600_000);
      await cache.put(path, new Response(await copy.text(), { headers: { 'content-type': 'application/json', 'x-clm-at': hourAgo } }));
    });
    mock.fail(SNAPSHOT, 502);
    await page.reload();
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect(page.locator('.offline')).toHaveText("CLM couldn't be reached · showing data from 1 h ago");
  });

  // req:web/honest-errors@165e071b
  test('busy, kept copy: asks again with backoff and draws live once a request succeeds', async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await page.clock.install();
    mock.fail(SNAPSHOT, 502);
    await page.reload();
    await expect(page.locator('.offline')).toBeVisible();
    mock.fail(SNAPSHOT, null);
    await expect(async () => {
      await page.clock.runFor(5_000);
      await expect(page.locator('.offline')).toHaveCount(0, { timeout: 500 });
    }).toPass({ timeout: 15_000 });
  });

  // req:web/honest-errors@165e071b
  test('preparing: shows the share done, asks again every 5 seconds, then draws', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    mock.fail(SNAPSHOT, 503, { preparing: 0.4 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.signin')).toContainText('40% done');
    mock.fail(SNAPSHOT, 503, { preparing: 0.8 });
    // The clock also runs on its own, so the wait is short: a longer interval fails.
    await page.clock.runFor(5_000);
    await expect(page.locator('.signin')).toContainText('80% done', { timeout: 2_000 });
    mock.fail(SNAPSHOT, null);
    await page.clock.runFor(5_000);
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
  });

  // req:web/honest-errors@165e071b
  test('preparing: a share past the whole shows 100% done', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(SNAPSHOT, 503, { preparing: 1.7 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.signin')).toContainText('CLM is preparing this repo: 100% done.');
  });

  // req:web/honest-errors@165e071b
  test('no access: says the repo is not readable with the account or not covered by the App', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(SNAPSHOT, 404);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.signin')).toContainText("isn't readable with your GitHub account, or isn't covered by the App");
  });

  // req:web/honest-errors@165e071b
  test('other refusals: a 403 is about access, a 429 says the request failed and nothing about access', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(SNAPSHOT, 403);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.signin')).toContainText("isn't readable with your GitHub account, or isn't covered by the App");
    mock.fail(SNAPSHOT, 429);
    await page.reload();
    const card = page.locator('.signin');
    await expect(card).toContainText('The request failed (429).');
    await expect(card).not.toContainText(/account|App|access|retrying/);
  });

  // req:web/honest-errors@165e071b
  test("a branch that's gone: its review's 404 says the branch is no longer open, leading back to the list", async ({ page, context }) => {
    await mockApi(context);
    // The mocked Worker answers 404 for a path it doesn't hold, as `spec/gone`'s review.
    await page.goto('/app/#/o/a/prs/spec%2Fgone');
    const card = page.locator('.gone');
    await expect(card).toContainText('spec/gone is no longer open.');
    await expect(card).not.toContainText(/account|App|access/);
    await expect(card.locator('a')).toHaveAttribute('href', '#/o/a/prs');
    await card.locator('a').click();
    await expect(page.locator('.prl[data-branch="spec/c"]')).toBeVisible();
  });

  // req:web/honest-errors@165e071b
  test("a branch that goes while its review is on screen: the next refresh says it is no longer open", async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await expect(page.locator('button.approve')).toBeVisible();
    mock.fail('GET /api/repos/o/a/branches/spec/c/review', 404);
    await expect(async () => {
      await page.clock.runFor(10_000);
      await expect(page.locator('.gone')).toContainText('spec/c is no longer open.', { timeout: 1_000 });
    }).toPass({ timeout: 30_000 });
  });

  // req:web/honest-errors@165e071b
  test('no branch list: the PR list and Home say the spec branches couldn\'t be loaded, and the graph is drawn', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail('GET /api/repos/o/a/branches', 502);
    await page.goto('/app/#/o/a/prs');
    await expect(page.locator('.prs .branches-failed')).toHaveText("The spec branches couldn't be loaded.");
    await expect(page.locator('.prs')).not.toContainText('No spec branches.');
    await page.goto('/app/#/o/a');
    await expect(page.locator('.home h1')).toBeVisible();
    await expect(page.locator('.branches-failed')).toHaveText("The spec branches couldn't be loaded.");
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect(page.locator('.branches-failed')).toHaveCount(0);
    // A list that loads again clears the message.
    mock.fail('GET /api/repos/o/a/branches', null);
    await page.goto('/app/#/o/a/prs');
    await expect(page.locator('.prl[data-branch="spec/c"]')).toBeVisible();
    await expect(page.locator('.branches-failed')).toHaveCount(0);
  });

  // req:web/honest-errors@165e071b
  test("a view that can't be drawn: a snapshot the graph can't be drawn from says so, not that CLM is retrying", async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes[SNAPSHOT] = { nothing: 'a graph is drawn from' };
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.signin')).toContainText("This view couldn't be drawn from what CLM answered.");
    await expect(page.locator('body')).not.toContainText('retrying');
  });

  // req:web/honest-errors@165e071b
  test('refreshes keep failing: the graph stays and shows how old it is', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    mock.fail(SNAPSHOT, 503, { error: 'unavailable' });
    // The first failed refresh is less than 3 minutes after the last good one: no banner yet.
    const asked = () => mock.requests.filter((r) => r === SNAPSHOT).length;
    const before = asked();
    await expect(async () => {
      await page.clock.runFor(5_000);
      expect(asked()).toBeGreaterThan(before);
    }).toPass({ timeout: 30_000 });
    await page.clock.runFor(1_000);
    await expect(page.locator('.offline')).toHaveCount(0);
    // A refresh may still be in flight when its minute comes, so keep the clock moving.
    await expect(async () => {
      await page.clock.runFor(10_000);
      await expect(page.locator('.offline.stale')).toHaveText(/^Couldn't refresh · showing data from \d+ min ago$/, { timeout: 1_000 });
    }).toPass({ timeout: 30_000 });
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    mock.fail(SNAPSHOT, null);
    await page.clock.runFor(60_000);
    await expect(page.locator('.offline')).toHaveCount(0);
  });
});
