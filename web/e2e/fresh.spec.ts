// The reload after the app's own writes, in the built app against a mocked Worker.

import { expect, test } from '@playwright/test';
import { APPROVE, PROPOSALS, REVIEW, mockApi } from './mock';

const IMPL = 'GET /api/repos/o/a/implementation';
const UUID = /^[0-9a-f-]+$/;

/** The `fresh` value of every request for `key`, in order (`null` when it carried none). */
const freshOf = (urls: string[], key: string) =>
  urls
    .filter((u) => u.startsWith(`${key}?`) || u === key)
    .map((u) => new URL(`http://x${u.slice(u.indexOf(' ') + 1)}`).searchParams.get('fresh'));

// req:web/fresh-after-write@0e8d2143
test.describe('fresh after write', () => {
  test('approve: the review is requested again with a UUID, and the offline copy has no fresh', async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('button.approve').click();
    await expect.poll(() => freshOf(mock.urls, REVIEW).length).toBeGreaterThanOrEqual(2);
    const [first, again] = freshOf(mock.urls, REVIEW);
    expect(first).toBeNull();
    expect(again).toMatch(UUID);
    const kept = await page.evaluate(async () => (await (await caches.open('clm-api')).keys()).map((r) => new URL(r.url).pathname + new URL(r.url).search));
    expect(kept).toContain('/api/repos/o/a/branches/spec/c/review');
    expect(kept.filter((k) => k.includes('fresh'))).toEqual([]);
  });

  test('a new delivery per write: two proposals, two different values', async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/backlog');
    for (const title of ['One', 'Two']) {
      await page.locator('button.propose').click();
      const form = page.locator('.fbform.proposal');
      await form.locator('input[name=title]').fill(title);
      await form.locator('textarea').fill('body');
      await form.locator('.picker input[value="a"]').check();
      await form.locator('button.send').click();
      await expect.poll(() => mock.posts.filter((p) => p.key === PROPOSALS).length).toBe(title === 'One' ? 1 : 2);
      await expect(page.locator('.fbform.proposal')).toHaveCount(0);
    }
    const withFresh = () => freshOf(mock.urls, IMPL).filter((f) => f !== null);
    await expect.poll(() => withFresh().length).toBeGreaterThanOrEqual(2);
    const fresh = withFresh();
    expect(fresh[0]).toMatch(UUID);
    expect(fresh[0]).not.toEqual(fresh[1]);
  });

  test('a refused write: a 409 still requests the review again with fresh', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(APPROVE, 409);
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('button.approve').click();
    await expect.poll(() => freshOf(mock.urls, REVIEW).length).toBeGreaterThanOrEqual(2);
    expect(freshOf(mock.urls, REVIEW)[1]).toMatch(UUID);
  });
});
