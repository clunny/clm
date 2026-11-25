// The app's writes as the shell runs them (specs/web/SPEC.md, app-writes): no refresh while
// one is in progress, the sign-in page after a 401 that no refresh draws over, and a write
// that fails at the network saying so beside its button or form.
import { expect, test, type Page } from '@playwright/test';
import { APPROVE, COMMENTS, DECIDE, PROPOSALS, REQUEST, mockApi, type Mock } from './mock';
import { implementation, snapshot } from '../src/views/fixtures';

const SNAPSHOT = 'GET /api/repos/o/a/snapshot';
const ANSWER = 'POST /api/repos/o/a/backlog/q/answer';
const WITHDRAW = 'DELETE /api/repos/o/a/proposals/p1';

/** One of the app's writes: where it is made, how, and where it says why it wasn't sent. */
interface Write {
  name: string;
  key: string;
  /** Sets the Worker up for the page the write is made on. */
  setup?: (mock: Mock) => void;
  route: string;
  /** Makes the write on the page drawn. */
  make: (page: Page) => Promise<void>;
  /** The button or form the write was made from, and where it says why it wasn't sent. */
  usable: (page: Page) => ReturnType<Page['locator']>;
  said: (page: Page) => ReturnType<Page['locator']>;
}

const withQuestion = () => {
  const snap = snapshot();
  snap.backlog.push({ slug: 'q', kind: 'question', title: 'Ship it?', nodes: ['a'], origin: null, blocks: [], choices: ['yes', 'no'], body: 'Should we ship?', priority: 'P2' });
  return snap;
};
const proposal = { id: 'p1', writer: 'alice/web', at: '2026-10-27T10:00:00Z', slug: 'export-csv', title: 'Export CSV', body: 'Let me export.', nodes: ['a'], mine: true };

const WRITES: Write[] = [
  {
    name: 'an approve',
    key: APPROVE,
    route: '/app/#/o/a/prs/spec%2Fc',
    make: (page) => page.locator('button.approve').click(),
    usable: (page) => page.locator('button.approve'),
    said: (page) => page.locator('.review > .note'),
  },
  {
    name: 'a request for changes',
    key: REQUEST,
    route: '/app/#/o/a/prs/spec%2Fc',
    make: async (page) => {
      await page.locator('.approve-bar button.request').click();
      await page.locator('.approve-bar .fbform textarea').fill('Split it');
      await page.locator('.approve-bar .fbform button.send').click();
    },
    usable: (page) => page.locator('.approve-bar .fbform button.send'),
    said: (page) => page.locator('.approve-bar .fbform .err'),
  },
  {
    name: 'a comment',
    key: COMMENTS,
    route: '/app/#/o/a/prs/spec%2Fc',
    make: async (page) => {
      await page.locator('.review .intent button.comment').click();
      await page.locator('.review .intent textarea').fill('Why now?');
      await page.locator('.review .intent button.send').click();
    },
    usable: (page) => page.locator('.review .intent button.send'),
    said: (page) => page.locator('.review .intent .fbform .err'),
  },
  {
    name: 'a proposal',
    key: PROPOSALS,
    route: '/app/#/o/a/backlog',
    make: async (page) => {
      await page.locator('button.propose').click();
      await page.locator('.fbform.proposal input[name=title]').fill('Export CSV');
      await page.locator('.fbform.proposal button.send').click();
    },
    usable: (page) => page.locator('.fbform.proposal button.send'),
    said: (page) => page.locator('.fbform.proposal .err'),
  },
  {
    name: 'a withdrawal',
    key: WITHDRAW,
    setup: (mock) => (mock.routes['GET /api/repos/o/a/implementation'] = implementation({ proposals: [proposal] })),
    route: '/app/#/o/a/backlog',
    make: (page) => page.locator('.prop[data-id=p1] button.withdraw').click(),
    usable: (page) => page.locator('.prop[data-id=p1] button.withdraw'),
    said: (page) => page.locator('.prop[data-id=p1] .err'),
  },
  {
    name: 'an answer',
    key: ANSWER,
    setup: (mock) => (mock.routes[SNAPSHOT] = withQuestion()),
    route: '/app/#/o/a/backlog/q',
    make: (page) => page.locator('button.answer[data-answer=yes]').click(),
    usable: (page) => page.locator('button.answer[data-answer=yes]'),
    said: (page) => page.locator('.answers .err'),
  },
  {
    name: "a capability's decision",
    key: DECIDE,
    route: '/app/#/o/a/capabilities/a',
    make: async (page) => {
      await page.locator('button.value', { hasText: 'low' }).click();
      await page.locator('button.decide', { hasText: 'Cut' }).click();
    },
    usable: (page) => page.locator('button.decide', { hasText: 'Cut' }),
    said: (page) => page.locator('.decide-note'),
  },
];

const count = (mock: Mock, key: string) => mock.requests.filter((r) => r === key).length;

/** Opens the write's page and waits until it can be made. */
async function open(page: Page, w: Write) {
  await page.goto(w.route);
  await expect(page.locator('.main')).toBeVisible();
  if (w.key === WITHDRAW) await expect(w.usable(page)).toBeEnabled();
  if (w.key === ANSWER) await expect(w.usable(page)).toBeEnabled();
}

for (const w of WRITES) {
  // The approve's own case is live.spec.ts's `no refresh runs while an approval is in progress`.
  if (w.key !== APPROVE) {
    // req:web/app-writes@d1c16934
    test(`a write in progress: no refresh runs while ${w.name} is held`, async ({ page, context }) => {
      await page.clock.install();
      const mock = await mockApi(context);
      w.setup?.(mock);
      await open(page, w);
      mock.hold(w.key);
      await w.make(page);
      await expect.poll(() => count(mock, w.key)).toBe(1);
      const before = count(mock, SNAPSHOT);
      await page.clock.runFor(60_000);
      await page.waitForTimeout(200);
      expect(count(mock, SNAPSHOT)).toBe(before);
      mock.release(w.key);
      // Once it has ended, the refresh runs again.
      await expect
        .poll(async () => {
          await page.clock.runFor(60_000);
          return count(mock, SNAPSHOT);
        })
        .toBeGreaterThan(before);
    });
  }

  // req:web/app-writes@d1c16934
  test(`signed out: a 401 to ${w.name} shows the sign-in page, and a refresh draws nothing over it`, async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    w.setup?.(mock);
    await open(page, w);
    mock.fail(w.key, 401);
    // Every other request answered as before, but for a snapshot that differs from the one
    // drawn, which a refresh would draw.
    const changed = (mock.routes[SNAPSHOT] as ReturnType<typeof snapshot>) ?? snapshot();
    mock.routes[SNAPSHOT] = { ...changed, sha: 'f'.repeat(40) };
    await w.make(page);
    await expect(page.locator('.signin')).toBeVisible();
    const before = count(mock, SNAPSHOT);
    await page.clock.runFor(60_000);
    await page.waitForTimeout(200);
    await expect(page.locator('.signin')).toBeVisible();
    await expect(page.locator('.main')).toHaveCount(0);
    expect(count(mock, SNAPSHOT)).toBe(before);
  });

  if (w.key !== REQUEST && w.key !== DECIDE) {
    // req:web/app-writes@d1c16934
    test(`not sent: ${w.name} that fails at the network, online, says the app is offline and stays usable`, async ({ page, context }) => {
      const mock = await mockApi(context);
      w.setup?.(mock);
      await open(page, w);
      mock.fail(w.key, 'abort');
      await w.make(page);
      await expect(w.said(page)).toContainText(/offline/i);
      await expect(w.usable(page)).toBeEnabled();
      expect(await page.evaluate(() => navigator.onLine)).toBe(true);
    });
  }
}

// req:web/app-writes@d1c16934
test("signed out: a 401 to a Notify me switch's POST shows the sign-in page, and a refresh draws nothing over it", async ({ page, context }) => {
  const endpoint = 'https://fcm.googleapis.com/fcm/send/1';
  // The browser's push service, faked: permission granted, and a subscription once asked.
  await page.addInitScript((endpoint) => {
    Object.defineProperty(Notification, 'permission', { get: () => 'granted' });
    let sub: unknown = null;
    PushManager.prototype.getSubscription = async () => sub as PushSubscription;
    PushManager.prototype.subscribe = async () => {
      sub = { endpoint, toJSON: () => ({ endpoint, keys: { p256dh: 'p', auth: 'a' } }), unsubscribe: async () => true };
      return sub as PushSubscription;
    };
  }, endpoint);
  await page.clock.install();
  const mock = await mockApi(context);
  mock.routes['GET /api/push/key'] = { key: 'k' };
  mock.routes['GET /api/repos/o/a/push'] = { subscribed: false };
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/app/#/o/a/graph');
  const toggle = page.locator('.sidebar .notify input[type=checkbox]');
  await expect(toggle).toBeEnabled();
  mock.fail('POST /api/repos/o/a/push', 401);
  mock.routes[SNAPSHOT] = { ...snapshot(), sha: 'f'.repeat(40) };
  await toggle.click();
  await expect(page.locator('.signin')).toBeVisible();
  await page.clock.runFor(60_000);
  await page.waitForTimeout(200);
  await expect(page.locator('.signin')).toBeVisible();
  await expect(page.locator('.main')).toHaveCount(0);
});
