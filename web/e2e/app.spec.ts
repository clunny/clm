import { expect, test } from '@playwright/test';
import { APPROVE, ARCHITECTURE, ARCH_DECIDE, CAPABILITIES, COMMENTS, DECIDE, PROPOSALS, REQUEST, REVIEW, mockApi, reviewOfB } from './mock';
import { architecture, capabilities, cost, implementation, review, snapshot } from '../src/views/fixtures';

/** The answer route of question `q`. */
const ANSWER = 'POST /api/repos/o/a/backlog/q/answer';
/** The fixture snapshot with a `question` that lists the choices `yes` and `no`. */
const withQuestion = () => {
  const snap = snapshot();
  snap.backlog.push({ slug: 'q', kind: 'question', title: 'Ship it?', nodes: ['a'], origin: null, blocks: [], choices: ['yes', 'no'], body: 'Should we **ship**?', priority: 'P2' });
  return snap;
};

// req:web/mobile-layout@cc420aa7
test.describe('phone', () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

  test('a PR at 390 px: tab bar, no horizontal scroll, Approve in view', async ({ page, context }) => {
    // A PR long enough to scroll, with an ID too long for one line.
    const pr = review();
    pr.nodes[0].requirements.modified = Array.from({ length: 30 }, (_, i) => ({
      id: `n/requirement-${i}-with-a-long-identifier-that-has-to-wrap-somewhere-on-a-phone`,
      before: 'It SHALL do r.',
      after: 'It SHALL do r quickly, in a sentence long enough to wrap onto more than one line.',
    }));
    await mockApi(context, pr);
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    const approve = page.locator('button.approve');
    await expect(approve).toBeVisible();
    await expect(page.locator('.tabbar')).toBeVisible();
    await expect(page.locator('.sidebar')).toBeHidden();
    await expect(approve).toBeInViewport({ ratio: 1 });
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    await expect(approve).toBeInViewport({ ratio: 1 });
    await expect(page.locator('.tabbar')).toBeInViewport({ ratio: 1 });
    const fits = await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth);
    expect(fits).toBe(true);
  });

  test('Home on a phone: no sideways scroll, and a question\'s answer buttons can be tapped', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos/o/a/snapshot'] = withQuestion();
    await page.goto('/app/#/o/a');
    const yes = page.locator('button.answer[data-answer=yes]');
    await expect(yes).toBeVisible();
    // Home is drawn again as the reviews, capabilities and architecture arrive, and a scroll
    // that overlaps a redraw finds its button detached, so scroll and measure in a retrying block.
    await expect(async () => {
      await yes.scrollIntoViewIfNeeded({ timeout: 2_000 });
      await expect(yes).toBeInViewport({ ratio: 1, timeout: 2_000 });
    }).toPass({ timeout: 15_000 });
    const fits = await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth);
    expect(fits).toBe(true);
    await yes.tap();
    await expect.poll(() => mock.posts).toEqual([{ key: ANSWER, body: '{"answer":"yes"}' }]);
  });

  test('every view fits the width', async ({ page, context }) => {
    await mockApi(context);
    for (const view of ['graph', 'prs', 'build', 'backlog', 'capabilities', 'capabilities/a', 'architecture', 'architecture/e', 'prs/spec%2Fc/full/specs/n/SPEC.md']) {
      await page.goto(`/app/#/o/a/${view}`);
      await expect(page.locator('.tabbar')).toBeVisible();
      await page.waitForSelector('.main > :not(.mtop):not(.offline)');
      const fits = await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth);
      expect(fits, view).toBe(true);
    }
  });

  test('the tab bar badges the items waiting, and a tapped node opens a bottom sheet', async ({ page, context }) => {
    await mockApi(context);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.tabbar a[data-view=backlog] .bdg')).toHaveText('1');
    await expect(page.locator('.insp')).toBeHidden();
    await page.locator('.n[data-node=b]').tap();
    const sheet = page.locator('.insp');
    await expect(sheet).toBeInViewport();
    await expect(sheet).toContainText('The b contract.');
    // The sheet is swapped again as the open PRs and the implementation state arrive, and a
    // box taken mid-swap is null, so measure it in a retrying assertion.
    const tabbar = (await page.locator('.tabbar').boundingBox())!;
    await expect
      .poll(async () => {
        const box = await sheet.boundingBox();
        return box ? Math.round(box.y + box.height) : Infinity;
      })
      .toBeLessThanOrEqual(Math.round(tabbar.y) + 1);
  });

  test('capabilities on a phone: cards, nothing scrolls sideways, and the decision buttons can be reached and tapped', async ({ page, context }) => {
    const mock = await mockApi(context);
    const fits = () => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth);
    await page.goto('/app/#/o/a/capabilities');
    await expect(page.locator('.ccard')).toHaveCount(2);
    await expect(page.locator('.ccard').first()).toBeVisible();
    await expect(page.locator('table.caps')).toBeHidden();
    expect(await fits()).toBe(true);
    await page.locator('.tabbar .tab-more').tap();
    await expect(page.locator('.more-menu a[data-view=capabilities]')).toBeVisible();
    await page.locator('.tabbar .tab-more').tap();
    await page.locator('.ccard[data-slug=a]').tap();
    await expect(page.locator('section[data-sec=workarounds]')).toBeVisible();
    expect(await fits()).toBe(true);
    const cut = page.locator('button.decide', { hasText: 'Cut' });
    // Scrolled to the end, it sits above the tab bar, so a tap reaches it.
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    await expect(cut).toBeInViewport({ ratio: 1 });
    const tabbar = (await page.locator('.tabbar').boundingBox())!;
    const box = (await cut.boundingBox())!;
    expect(box.y + box.height).toBeLessThanOrEqual(tabbar.y + 1);
    // The 2×2 grid: Keep and Cut side by side, Redesign below Keep. The three boxes are read in
    // one frame, and retried, since the page can still be settling under a loaded suite.
    await expect
      .poll(() =>
        page.evaluate(() => {
          const y = (t: string) =>
            [...document.querySelectorAll('button.decide')].find((b) => b.textContent?.includes(t))!.getBoundingClientRect().top;
          return { sideBySide: Math.round(y('Keep')) === Math.round(y('Cut')), below: y('Redesign') > y('Keep') };
        }),
      )
      .toEqual({ sideBySide: true, below: true });
    await page.locator('button.value', { hasText: 'low' }).tap();
    await cut.tap();
    await expect.poll(() => mock.posts).toEqual([{ key: DECIDE, body: '{"version":"61000000","value":"low","decision":"cut"}' }]);
  });

  test('architecture on a phone: five tabs, More listing PRs, Capabilities and Arch., cards, nothing scrolls sideways, and the decision buttons can be reached and tapped', async ({ page, context }) => {
    const mock = await mockApi(context);
    const fits = () => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth);
    await page.goto('/app/#/o/a/architecture');
    await expect(page.locator('.ccard')).toHaveCount(2);
    await expect(page.locator('.ccard').first()).toBeVisible();
    await expect(page.locator('table.caps')).toBeHidden();
    const tabs = page.locator('.tabbar a');
    const text = (all: string[]) => all.map((t) => t.replace(/\s+/g, ' ').replace(/ \d+$/, '').trim());
    await expect(tabs).toHaveCount(4);
    await expect(page.locator('.tabbar > *')).toHaveCount(5);
    expect(text(await page.locator('.tabbar > *').allInnerTexts())).toEqual(['⌂ Home', '☰ Backlog', '◇ Graph', '▶ Build', '⋯ More']);
    for (const tab of await page.locator('.tabbar > *').all()) await expect(tab).toBeInViewport({ ratio: 1 });
    await page.locator('.tabbar .tab-more').tap();
    expect(text(await page.locator('.more-menu a').allInnerTexts())).toEqual(['PRs', 'Capabilities', 'Arch.']);
    await page.locator('.tabbar .tab-more').tap();
    await expect(page.locator('.more-menu')).toBeHidden();
    expect(await fits()).toBe(true);
    await page.locator('.ccard[data-slug=e]').tap();
    await expect(page.locator('section[data-sec=held]')).toBeVisible();
    expect(await fits()).toBe(true);
    const redesign = page.locator('button.decide', { hasText: 'Redesign' });
    await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
    await expect(redesign).toBeInViewport({ ratio: 1 });
    const tabbar = (await page.locator('.tabbar').boundingBox())!;
    const box = (await redesign.boundingBox())!;
    expect(box.y + box.height).toBeLessThanOrEqual(tabbar.y + 1);
    // Laid out as a capability's: a 2×2 grid, Keep and Cut side by side, Redesign below Keep.
    await expect
      .poll(() =>
        page.evaluate(() => {
          const y = (t: string) =>
            [...document.querySelectorAll('button.decide')].find((b) => b.textContent?.includes(t))!.getBoundingClientRect().top;
          return { sideBySide: Math.round(y('Keep')) === Math.round(y('Cut')), below: y('Redesign') > y('Keep') };
        }),
      )
      .toEqual({ sideBySide: true, below: true });
    await page.locator('button.value', { hasText: 'medium' }).tap();
    await redesign.tap();
    await expect.poll(() => mock.posts).toEqual([{ key: ARCH_DECIDE, body: '{"version":"65000000","value":"medium","decision":"redesign"}' }]);
  });
});

// req:web/cost-views-in-the-browser@3d307cc3
test('capabilities from 900 px: a table in the report order, ⚑ only filters, and a row opens its page', async ({ page, context }) => {
  await mockApi(context);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/app/#/o/a/capabilities');
  await expect(page.locator('table.caps tr.cap')).toHaveCount(2);
  await expect(page.locator('.ccard').first()).toBeHidden();
  await expect(page.locator('table.caps tr.cap').first()).toHaveAttribute('data-slug', 'b');
  await page.locator('button.flag-only').click();
  await expect(page.locator('table.caps tr.cap')).toHaveCount(1);
  await page.locator('tr.cap[data-slug=a] a').click();
  await expect(page).toHaveURL(/#\/o\/a\/capabilities\/a$/);
  await expect(page.locator('section[data-sec=entry]')).toContainText('Without a, nothing works.');
  await expect(page.locator('.sidebar .nv[data-view=capabilities]')).toHaveClass(/on/);
});

test.describe('architecture list', () => {
  test.use({ viewport: { width: 1280, height: 800 } });

  // req:web/cost-views-in-the-browser@3d307cc3
  test('its own view: listed after Capabilities in the sidebar, opening #/o/a/architecture', async ({ page, context }) => {
    await mockApi(context);
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.sidebar .nv[data-view=architecture]')).toBeVisible();
    const views = await page.locator('.sidebar .nv[data-view]').evaluateAll((a) => a.map((e) => [(e as HTMLElement).dataset.view, e.textContent]));
    expect(views.slice(-2)).toEqual([
      ['capabilities', 'Capabilities'],
      ['architecture', 'Architecture'],
    ]);
    await page.locator('.sidebar .nv[data-view=architecture]').click();
    await expect(page).toHaveURL(/#\/o\/a\/architecture$/);
    await expect(page.locator('.sidebar .nv[data-view=architecture]')).toHaveClass(/on/);
  });

  // req:web/cost-views-in-the-browser@3d307cc3
  test('ranked, held and flagged: a table in the report order, held 1/2 in amber, ⚑ only filters, and a row opens its page', async ({ page, context }) => {
    await mockApi(context);
    await page.goto('/app/#/o/a/architecture');
    await expect(page.locator('table.caps tr.cap')).toHaveCount(2);
    await expect(page.locator('.ccard').first()).toBeHidden();
    await expect(page.locator('table.caps tr.cap').first()).toHaveAttribute('data-slug', 'e');
    const held = page.locator('tr.cap[data-slug=e] .proof');
    await expect(held).toHaveText('held 1/2 + untested');
    const [amber, color] = await held.evaluate((e) => [getComputedStyle(document.documentElement).getPropertyValue('--amber').trim(), getComputedStyle(e).color]);
    const probe = await page.evaluate((c) => {
      const d = document.createElement('i');
      d.style.color = c;
      document.body.append(d);
      const v = getComputedStyle(d).color;
      d.remove();
      return v;
    }, amber);
    expect(color).toBe(probe);
    await expect(page.locator('tr.cap[data-slug=g] .proof')).toHaveText('untested');
    await page.locator('button.flag-only').click();
    await expect(page.locator('table.caps tr.cap')).toHaveCount(1);
    await page.locator('tr.cap[data-slug=e] a').click();
    await expect(page).toHaveURL(/#\/o\/a\/architecture\/e$/);
    await expect(page.locator('section[data-sec=entry]')).toContainText('Partly judgement.');
  });

  // req:web/cost-views-in-the-browser@3d307cc3
  test('indexing: the proof reads ? / 2 and the note says coverage is still indexing', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes[ARCHITECTURE] = architecture({ indexing: true });
    await page.goto('/app/#/o/a/architecture');
    await expect(page.locator('tr.cap[data-slug=e] .proof')).toHaveText('? / 2 + untested');
    await expect(page.locator('.cost-note')).toContainText('still indexing');
  });
});

test.describe('deciding an architectural constraint', () => {
  test.use({ viewport: { width: 1280, height: 800 } });

  // req:web/cost-views-in-the-browser@3d307cc3
  test('one request per tap: two quick taps post once, the report is asked again and the decision shows', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.onPost = () => {
      const a = architecture();
      a.architecture[0].decisions = [{ at: '2026-10-30T09:00:00Z', writer: 'clunny/web', value: 'medium', decision: 'redesign' }];
      mock.routes[ARCHITECTURE] = a;
    };
    mock.hold(ARCH_DECIDE);
    await page.goto('/app/#/o/a/architecture/e');
    await page.locator('button.value', { hasText: 'medium' }).click();
    const redesign = page.locator('button.decide', { hasText: 'Redesign' });
    await redesign.click();
    await expect(redesign).toBeDisabled();
    await redesign.click({ force: true });
    mock.release(ARCH_DECIDE);
    await expect(page.locator('section[data-sec=decisions]')).toContainText('redesign (value medium) by clunny/web');
    expect(mock.posts).toEqual([{ key: ARCH_DECIDE, body: '{"version":"65000000","value":"medium","decision":"redesign"}' }]);
    expect(mock.requests.filter((r) => r === ARCHITECTURE)).toHaveLength(2);
    expect(mock.urls.filter((u) => u.startsWith(`${ARCHITECTURE}?fresh=`))).toHaveLength(1);
  });

  // req:web/cost-views-in-the-browser@3d307cc3
  test('moved: a 409 says the entry changed and draws it as it is now', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(ARCH_DECIDE, 409);
    mock.onPost = () => {
      const a = architecture();
      a.architecture[0] = { ...a.architecture[0], version: 'feedbeef', text: '### Constraint: e\nCLM SHALL e, reworded.' };
      mock.routes[ARCHITECTURE] = a;
    };
    await page.goto('/app/#/o/a/architecture/e');
    await page.locator('button.value', { hasText: 'high' }).click();
    await page.locator('button.decide', { hasText: 'Keep' }).click();
    await expect(page.locator('.decide-note')).toContainText('The entry changed since it was shown');
    await expect(page.locator('.capage')).toContainText('CLM SHALL e, reworded.');
  });

  // req:web/cost-views-in-the-browser@3d307cc3
  test('signed: the buttons are disabled and the note gives clm architecture decide', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes[ARCHITECTURE] = architecture({ signed: true });
    await page.goto('/app/#/o/a/architecture/e');
    await page.locator('button.value', { hasText: 'high' }).click();
    await expect(page.locator('button.decide').first()).toBeDisabled();
    await expect(page.locator('.signed-note code')).toHaveText('clm architecture decide e --value high|medium|low --decision keep|cut|redesign|accept');
  });
});

test.describe('deciding a capability', () => {
  test.use({ viewport: { width: 1280, height: 800 } });

  // req:web/cost-views-in-the-browser@3d307cc3
  test('decided: two quick taps post once, the report is asked again and the decision shows', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.onPost = () => {
      const c = capabilities();
      c.report.capabilities[1].decisions = [{ at: '2026-10-30T09:00:00Z', writer: 'clunny/web', value: 'low', decision: 'cut' }];
      mock.routes[CAPABILITIES] = c;
    };
    mock.hold(DECIDE);
    await page.goto('/app/#/o/a/capabilities/a');
    await page.locator('button.value', { hasText: 'low' }).click();
    const cut = page.locator('button.decide', { hasText: 'Cut' });
    await cut.click();
    await expect(cut).toBeDisabled();
    await cut.click({ force: true });
    // Choosing a value draws the page again: the buttons stay off while the request is out.
    await page.locator('button.value', { hasText: 'high' }).click();
    await expect(cut).toBeDisabled();
    await cut.click({ force: true });
    mock.release(DECIDE);
    await expect(page.locator('section[data-sec=decisions]')).toContainText('cut (value low) by clunny/web');
    expect(mock.posts).toEqual([{ key: DECIDE, body: '{"version":"61000000","value":"low","decision":"cut"}' }]);
    expect(mock.requests.filter((r) => r === CAPABILITIES)).toHaveLength(2);
    // Asked anew, so the Worker reads the ledgers again rather than its minute-old copy.
    expect(mock.urls.filter((u) => u.startsWith(`${CAPABILITIES}?fresh=`))).toHaveLength(1);
  });

  // req:web/app-writes@d1c16934
  // req:web/cost-views-in-the-browser@3d307cc3
  test('a refresh begun before a decision never draws its older report over the decision', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.onPost = () => {
      const c = capabilities();
      c.report.capabilities[1].decisions = [{ at: '2026-10-30T09:00:00Z', writer: 'clunny/web', value: 'low', decision: 'cut' }];
      mock.routes[CAPABILITIES] = c;
    };
    // The second ask is the refresh: it is answered with the report before the decision, but
    // only after the decision has been drawn.
    let asks = 0;
    let answered = false;
    let go = () => {};
    const later = new Promise<void>((r) => (go = r));
    await context.route('**/api/repos/o/a/capabilities*', async (route) => {
      if (++asks !== 2) return route.fallback();
      const stale = JSON.stringify(capabilities());
      await later;
      await route.fulfill({ status: 200, contentType: 'application/json', body: stale });
      answered = true;
    });
    await page.goto('/app/#/o/a/capabilities/a');
    await expect(page.locator('section[data-sec=decisions]')).toContainText('never decided');
    await page.evaluate(() => document.dispatchEvent(new Event('visibilitychange')));
    await expect.poll(() => asks).toBe(2);
    await page.locator('button.value', { hasText: 'low' }).click();
    await page.locator('button.decide', { hasText: 'Cut' }).click();
    await expect(page.locator('section[data-sec=decisions]')).toContainText('cut (value low) by clunny/web');
    go();
    await expect.poll(() => answered).toBe(true);
    await page.evaluate(() => new Promise((r) => setTimeout(r, 200)));
    await expect(page.locator('section[data-sec=decisions]')).toContainText('cut (value low) by clunny/web');
  });

  // req:web/cost-views-in-the-browser@3d307cc3
  test('moved: a 409 says the capability changed and draws it as it is now', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(DECIDE, 409);
    mock.onPost = () => {
      const c = capabilities();
      c.report.capabilities[1] = cost('a', 2, { version: 'feedbeef', text: '### Capability: a\nA person can a, reworded.' });
      mock.routes[CAPABILITIES] = c;
    };
    await page.goto('/app/#/o/a/capabilities/a');
    await page.locator('button.value', { hasText: 'high' }).click();
    await page.locator('button.decide', { hasText: 'Keep' }).click();
    await expect(page.locator('.decide-note')).toContainText('changed since it was shown');
    await expect(page.locator('.capage')).toContainText('A person can a, reworded.');
    mock.fail(DECIDE, null);
    await page.locator('button.decide', { hasText: 'Keep' }).click();
    await expect.poll(() => mock.posts.map((p) => p.body)).toEqual([
      '{"version":"61000000","value":"high","decision":"keep"}',
      '{"version":"feedbeef","value":"high","decision":"keep"}',
    ]);
  });

  // req:web/cost-views-in-the-browser@3d307cc3
  test('became signed: a 422 signed shows the Worker’s command and disables the buttons', async ({ page, context }) => {
    const mock = await mockApi(context);
    const command = 'clm capabilities decide a --value high --decision keep';
    mock.fail(DECIDE, 422, { error: 'signed', command });
    mock.onPost = () => (mock.routes[CAPABILITIES] = capabilities({ signed: true }));
    await page.goto('/app/#/o/a/capabilities/a');
    await page.locator('button.value', { hasText: 'high' }).click();
    await page.locator('button.decide', { hasText: 'Keep' }).click();
    await expect(page.locator('.decide-note code')).toHaveText(command);
    await expect(page.locator('button.decide').first()).toBeDisabled();
  });

  // req:web/cost-views-in-the-browser@3d307cc3
  test("a report that can't be loaded again after a decision: no kept copy drawn, and the page says so", async ({ page, context }) => {
    for (const [layer, route, key, slug] of [
      ['capabilities', CAPABILITIES, DECIDE, 'a'],
      ['architecture', ARCHITECTURE, ARCH_DECIDE, 'e'],
    ] as const) {
      const mock = await mockApi(context);
      // The report is kept for offline use when it is first drawn; once the decision is
      // taken, the Worker answers the report with 502.
      mock.onPost = (k) => k === key && mock.fail(route, 502);
      await page.goto(`/app/#/o/a/${layer}/${slug}`);
      // A copy of the report is kept for offline use.
      await expect
        .poll(() => page.evaluate(async (p) => !!(await (await caches.open('clm-api')).match(p)), `/api/repos/o/a/${layer}`))
        .toBe(true);
      await page.locator('button.value', { hasText: 'low' }).click();
      await page.locator('button.decide', { hasText: 'Cut' }).click();
      await expect(page.locator('.decide-note')).toContainText('Its report could not be loaded again');
      expect(mock.posts.map((p) => p.key)).toEqual([key]);
      // The kept copy would show under CLM couldn't be reached.
      await expect(page.locator('.offline')).toHaveCount(0);
      await context.unroute('**/api/**');
    }
  });
});

test('wide screens get the sidebar and no tab bar', async ({ page, context }) => {
  await mockApi(context);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/app/#/o/a/backlog');
  await expect(page.locator('.sidebar')).toBeVisible();
  await expect(page.locator('.tabbar')).toBeHidden();
  await expect(page.locator('.sidebar .nv[data-view=backlog] .c')).toHaveText('1');
});

// req:web/review-in-the-browser@9b9f5325
test('opening a file in full scrolls to its first change, and j steps to the next', async ({ page, context }) => {
  const requirements = Array.from({ length: 40 }, (_, i) => `### Requirement: r${i}\nIt SHALL do ${i}.`).join('\n\n');
  const before = `# N\n\n### Design\n\nThe design.\n\n### Requirements\n\n${requirements}\n`;
  // Both changes are far below the top, so the first is in view only if the app scrolls there.
  const after = before.replace('It SHALL do 25.', 'It SHALL do 25, changed.').replace('It SHALL do 39.', 'It SHALL do 39 twice.');
  await mockApi(context, review({ documents: [{ path: 'specs/n/SPEC.md', before, after }] }));
  await page.goto('/app/#/o/a/prs/spec%2Fc/full/specs/n/SPEC.md');
  await expect(page.locator('.pos')).toHaveText('Change 1 of 2');
  const focus = page.locator('.chg.focus');
  await expect(focus).toContainText('It SHALL do 25, changed.');
  await expect(focus).toBeInViewport();
  await page.keyboard.press('j');
  await expect(page.locator('.pos')).toHaveText('Change 2 of 2');
  await expect(page.locator('.chg.focus')).toContainText('It SHALL do 39 twice.');
  await expect(page.locator('.chg.focus')).toBeInViewport();
});

// req:web/review-in-the-browser@9b9f5325
test("a file not in the review: the page says it isn't changed in the branch", async ({ page, context }) => {
  await mockApi(context);
  await page.goto('/app/#/o/a/prs/spec%2Fc/full/README.md');
  await expect(page.locator('.main')).toContainText("README.md isn't changed in spec/c.");
});

test.describe('graph', () => {
  // req:web/graph-in-the-browser@3a6ebaea
  test('queue states: once the implementation state arrives, b is ringed amber and named claimed, a rose and blocked', async ({ page, context }) => {
    await mockApi(context);
    await page.goto('/app/#/o/a/graph');
    // The crumb names the snapshot's repo.
    await expect(page.locator('.main .crumb')).toHaveText('o/a');
    const b = page.locator('.n[data-node=b]');
    const a = page.locator('.n[data-node=a]');
    await expect(b).toHaveClass(/\bq-claimed\b/);
    await expect(b.locator('.q')).toHaveText('claimed');
    await expect(a).toHaveClass(/\bq-blocked\b/);
    await expect(a.locator('.q')).toHaveText('blocked');
    // The ring is drawn in the theme's colour for the state.
    const colours = await page.evaluate(() => {
      const resolve = (v: string) => {
        const probe = document.body.appendChild(document.createElement('i'));
        probe.style.color = v;
        const c = getComputedStyle(probe).color;
        probe.remove();
        return c;
      };
      const outline = (id: string) => {
        const s = getComputedStyle(document.querySelector(`.n[data-node=${id}]`)!);
        return [s.outlineStyle, s.outlineColor];
      };
      return { a: outline('a'), b: outline('b'), amber: resolve('var(--amber)'), rose: resolve('var(--rose)') };
    });
    expect(colours.b).toEqual(['solid', colours.amber]);
    expect(colours.a).toEqual(['solid', colours.rose]);
  });

  // req:web/graph-in-the-browser@3a6ebaea
  test("the crumb names the snapshot's repo, not the route's", async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos/o/a/snapshot'] = snapshot({ repo: 'O/A' });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect(page.locator('.main .crumb')).toHaveText('O/A');
  });

  // req:web/graph-in-the-browser@3a6ebaea
  test('a long node ID gives way to the state word, which stays inside the card', async ({ page, context }) => {
    const mock = await mockApi(context);
    const long = 'b-with-an-identifier-far-too-long-for-its-card';
    const snap = snapshot();
    for (const n of snap.spec.nodes) if (n.node === 'b') n.node = long;
    const impl = implementation();
    for (const n of impl.flight.nodes) if (n.node === 'b') n.node = long;
    mock.routes['GET /api/repos/o/a/snapshot'] = snap;
    mock.routes['GET /api/repos/o/a/implementation'] = impl;
    await page.goto('/app/#/o/a/graph');
    const card = page.locator(`.n[data-node="${long}"]`);
    const word = card.locator('.q');
    await expect(word).toHaveText('claimed');
    const [c, w] = [(await card.boundingBox())!, (await word.boundingBox())!];
    expect(w.width).toBeGreaterThan(0);
    expect(w.x + w.width).toBeLessThanOrEqual(c.x + c.width);
  });

  // req:web/graph-in-the-browser@3a6ebaea
  test('a graph drawn with the implementation state already held is ringed at once', async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/build');
    await expect(page.locator('.fn[data-node=b]')).toBeVisible();
    // Nothing more arrives: the rings can only come from the state Build loaded.
    mock.hold('GET /api/repos/o/a/implementation');
    await page.evaluate(() => (location.hash = '#/o/a/graph'));
    await expect(page.locator('.n[data-node=b]')).toHaveClass(/\bq-claimed\b/);
    await expect(page.locator('.n[data-node=a] .q')).toHaveText('blocked');
  });

  // req:web/graph-in-the-browser@3a6ebaea
  test("a PR's Graph tab marks only the nodes it touches", async ({ page, context }) => {
    await mockApi(context, reviewOfB());
    await page.goto('/app/#/o/a/prs/spec%2Fc/graph');
    await expect(page.locator('.n.hit')).toHaveCount(1);
    await expect(page.locator('.n.hit')).toHaveAttribute('data-node', 'b');
    await expect(page.locator('path.e.eh[data-from=b][data-to=a]')).toHaveCount(1);
  });

  // req:web/graph-in-the-browser@3a6ebaea
  test("a selected node's inspector lists the open PRs that change it", async ({ page, context }) => {
    await mockApi(context, reviewOfB());
    await page.goto('/app/#/o/a/graph');
    await page.locator('.n[data-node=b]').click();
    await expect(page.locator('.insp .inpr')).toContainText('spec/c');
    // The drawer may cover the card, so tap it without a pointer.
    await page.locator('.n[data-node=a]').dispatchEvent('click');
    await expect(page.locator('.insp .inpr')).toContainText('No open PR changes it.');
  });
});

test.describe('spec branches', () => {
  // req:web/review-in-the-browser@9b9f5325
  test("branch without a page: both listed, and spec/d's review loads from its branch route with no page link", async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos/o/a/branches/spec/d/review'] = review({ branch: 'spec/d', page: null });
    await page.goto('/app/#/o/a/prs');
    const rows = page.locator('.prl[data-branch]');
    await expect(rows).toHaveCount(2);
    await expect(rows.nth(0)).toContainText('spec/c');
    await expect(rows.nth(0)).toContainText('#7');
    await expect(rows.nth(0)).toContainText('Change b');
    await expect(rows.nth(1)).toHaveText('spec/d');
    await rows.nth(1).locator('a.open').click();
    await expect(page).toHaveURL(/#\/o\/a\/prs\/spec%2Fd$/);
    await expect(page.locator('.review > .head h1')).toHaveText('spec/d');
    await expect(page.locator('.review > .head a.page')).toHaveCount(0);
    expect(mock.requests).toContain('GET /api/repos/o/a/branches/spec/d/review');
  });

  // req:web/review-in-the-browser@9b9f5325
  test('approved by: the header names clunny/web with its time and says not checked, and no request went to /pulls', async ({ page, context }) => {
    const mock = await mockApi(context, review({ approvals: [{ writer: 'clunny/web', at: '2026-10-27T10:00:00Z' }], checked: false }));
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    const head = page.locator('.review > .head');
    await expect(head).toContainText('Approved by clunny/web · 2026-10-27 10:00 UTC');
    await expect(head).toContainText('not checked');
    await page.goto('/app/#/o/a/graph');
    await page.locator('.n[data-node=a]').click();
    // The inspector has drawn the open PRs it loaded before Build is opened.
    await expect(page.locator('.insp .inpr')).toBeVisible();
    await page.goto('/app/#/o/a/build');
    await expect(page.locator('.impl')).toBeVisible();
    expect(mock.requests.filter((r) => r.includes('/pulls'))).toEqual([]);
  });
});

test.describe('Approve by branch', () => {
  // req:web/approve-by-branch@110f0e02
  test('approve: one post of the fingerprint to the branch, the review asked again, clunny/web named and Approve disabled', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.onPost = () => (mock.routes[REVIEW] = review({ approvals: [{ writer: 'clunny/web', at: '2026-10-27T10:00:00Z' }] }));
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('button.approve').click();
    await expect(page.locator('.review > .head')).toContainText('Approved by clunny/web');
    await expect(page.locator('button.approve')).toBeDisabled();
    expect(mock.posts).toEqual([{ key: APPROVE, body: '{"fingerprint":"f1"}' }]);
    expect(mock.requests.filter((r) => r === REVIEW)).toHaveLength(2);
  });

  // req:web/approve-by-branch@110f0e02
  test('branch moved: a 409 says the branch changed, shows no approval, and draws the review with fingerprint f2', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(APPROVE, 409);
    mock.onPost = () => (mock.routes[REVIEW] = review({ fingerprint: 'f2', change: { ...review().change!, intent_section: 'Make b better, now.' } }));
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('button.approve').click();
    await expect(page.locator('.review')).toContainText('The branch changed since this review was shown');
    await expect(page.locator('.review .intent')).toContainText('Make b better, now.');
    await expect(page.locator('.review > .head')).toContainText('not approved');
    // The next Approve pins what is now on screen.
    mock.fail(APPROVE, null);
    await page.locator('button.approve').click();
    await expect.poll(() => mock.posts.map((p) => p.body)).toEqual(['{"fingerprint":"f1"}', '{"fingerprint":"f2"}']);
  });

  // req:web/approve-by-branch@110f0e02
  test("a 409 whose new review can't be loaded says to reload before approving", async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(APPROVE, 409);
    mock.onPost = () => mock.fail(REVIEW, 502);
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('button.approve').click();
    await expect(page.locator('.review')).toContainText('The branch changed since this review was shown, and its new review could not be loaded. Reload before approving.');
    // The kept copy would show under CLM couldn't be reached.
    await expect(page.locator('.offline')).toHaveCount(0);
  });

  // req:web/approve-by-branch@110f0e02
  test('a 409 whose reload fails at the network never draws the copy kept for offline use', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(APPROVE, 409);
    mock.onPost = () => mock.fail(REVIEW, 'abort');
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('button.approve').click();
    await expect(page.locator('.review')).toContainText('The branch changed since this review was shown, and its new review could not be loaded. Reload before approving.');
    await expect(page.locator('.offline')).toHaveCount(0);
  });

  // req:web/approve-by-branch@110f0e02
  test("approved, but the review can't be loaded again: says Approved. and to reload", async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.onPost = () => mock.fail(REVIEW, 502);
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('button.approve').click();
    await expect(page.locator('.review')).toContainText('Approved. Reload to see the review as it is now.');
    await expect(page.locator('.offline')).toHaveCount(0);
    expect(mock.posts).toEqual([{ key: APPROVE, body: '{"fingerprint":"f1"}' }]);
  });

  // req:web/approve-by-branch@110f0e02
  test('any other status says Not approved with the status, and leaves Approve enabled', async ({ page, context }) => {
    const mock = await mockApi(context);
    for (const status of [500, 422]) {
      mock.fail(APPROVE, status);
      await page.goto('/app/#/o/a/prs/spec%2Fc');
      await page.reload();
      await page.locator('button.approve').click();
      await expect(page.locator('.review')).toContainText(`Not approved (${status}).`);
      await expect(page.locator('button.approve')).toBeEnabled();
    }
  });
});

test.describe('signed repositories', () => {
  // req:web/approve-by-branch@110f0e02
  test('became signed: a 422 signed loads the review again, and the note shows clm approve', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(APPROVE, 422, { error: 'signed', command: 'clm approve spec/c' });
    mock.onPost = () => (mock.routes[REVIEW] = review({ signed: true }));
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await expect(page.locator('.signed-note')).toHaveCount(0);
    const before = mock.requests.filter((r) => r === REVIEW).length;
    await page.locator('button.approve').click();
    await expect(page.locator('.signed-note code')).toHaveText('clm approve spec/c');
    expect(mock.requests.filter((r) => r === REVIEW).length).toBeGreaterThan(before);
    await expect(page.locator('button.approve')).toBeDisabled();
    await expect(page.locator('button.approve')).not.toHaveText(/Approved/);
  });

  // req:web/review-in-the-browser@9b9f5325
  test('on a phone the note sits in the Approve bar, above the tab bar', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes[REVIEW] = review({ signed: true, approvals: [{ writer: 'clunny/web', at: '2026-10-27T10:00:00Z', signed: false }] });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await expect(page.locator('.approve-bar .signed-note')).toBeInViewport();
    await expect(page.locator('.review > .head')).toContainText("unsigned, doesn't count");
    const note = (await page.locator('.approve-bar .signed-note').boundingBox())!;
    const tabbar = (await page.locator('.tabbar').boundingBox())!;
    expect(note.y + note.height).toBeLessThanOrEqual(tabbar.y + 1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
});

// req:web/review-in-the-browser@9b9f5325
test('one request, then approved: two quick taps send one approve, and the button reads Approved, disabled', async ({ page, context }) => {
  const mock = await mockApi(context);
  mock.hold(APPROVE);
  await page.goto('/app/#/o/a/prs/spec%2Fc');
  const approve = page.locator('button.approve');
  await approve.click();
  await approve.click({ force: true });
  mock.release(APPROVE);
  await expect(approve).toHaveText('Approved');
  await expect(approve).toBeDisabled();
  expect(mock.posts).toHaveLength(1);
});

// req:web/backlog-in-the-browser@5ad99bbe
test('the backlog is drawn before the implementation state, and takes its marks when it arrives', async ({ page, context }) => {
  const mock = await mockApi(context);
  mock.routes['GET /api/repos/o/a/implementation'] = implementation({ taken: { changes: [{ id: 'x', state: 'main', items: ['crash'], nodes: ['a'] }], claims: [{ item: 'later', nodes: [] }] } });
  mock.hold('GET /api/repos/o/a/implementation');
  await page.goto('/app/#/o/a/backlog');
  await expect(page.locator('.bi[data-slug=crash]')).toBeVisible();
  await expect(page.locator('.taken')).toHaveCount(0);
  mock.release('GET /api/repos/o/a/implementation');
  await expect(page.locator('.bi[data-slug=crash] .taken')).toHaveText('resolved by x');
  await expect(page.locator('.bi[data-slug=later] .taken')).toHaveText('claimed');
  await expect(page.locator('.bi[data-slug=setup] .taken')).toHaveCount(0);
});

// req:web/graph-in-the-browser@3a6ebaea
test("an oversized node's card is outlined amber, unless a queue ring colours it", async ({ page, context }) => {
  const snap = snapshot();
  snap.spec.nodes = snap.spec.nodes.map((n) => (n.parent ? { ...n, oversized: true } : n));
  const mock = await mockApi(context);
  mock.routes['GET /api/repos/o/a/snapshot'] = snap;
  // `a` is blocked, so its ring is rose once the implementation state arrives.
  mock.hold('GET /api/repos/o/a/implementation');
  await page.goto('/app/#/o/a/graph');
  const outline = (id: string) => page.locator(`.n[data-node=${id}]`).evaluate((e) => getComputedStyle(e).outlineColor);
  const colour = (token: string) =>
    page.evaluate((t) => {
      const probe = document.createElement('i');
      probe.style.color = `var(${t})`;
      document.body.append(probe);
      return getComputedStyle(probe).color;
    }, token);
  await expect.poll(() => outline('a')).toBe(await colour('--amber'));
  await expect(page.locator('.legend')).toContainText('behind · claimed · oversized');
  mock.release('GET /api/repos/o/a/implementation');
  // Queue ring wins the outline: a keeps its ⚠ and is outlined in the blocked ring's rose.
  await expect(page.locator('.n[data-node=a]')).toHaveClass(/\bq-blocked\b/);
  await expect(page.locator('.n[data-node=a] .t')).toContainText('⚠');
  await expect.poll(() => outline('a')).toBe(await colour('--rose'));
});

test.describe('Notify me', () => {
  // req:web/push-in-the-browser@06ad4e4a
  test('no key: a Worker answering 404 for the push key shows no switch', async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect.poll(() => mock.requests.includes('GET /api/push/key')).toBe(true);
    await expect(page.locator('.sidebar .notify')).toBeHidden();
  });

  // req:web/push-in-the-browser@06ad4e4a
  test('with a key, the sidebar shows the switch under the repo picker, off without a subscription', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/push/key'] = { key: 'BKey' };
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    const toggle = page.locator('.sidebar .notify input[type=checkbox]');
    await expect(toggle).toBeVisible();
    await expect(toggle).not.toBeChecked();
    await expect(toggle).toBeEnabled();
    // No subscription in this browser: its state needs no request.
    expect(mock.requests.filter((r) => r.startsWith('GET /api/repos/o/a/push'))).toEqual([]);
    // On a phone the switch sits in the top bar, beside the repo switcher.
    await page.setViewportSize({ width: 390, height: 844 });
    const top = page.locator('.mtop .notify input[type=checkbox]');
    await expect(top).toBeVisible();
    await expect(top).not.toBeChecked();
    await expect(top).toBeEnabled();
    await expect(page.locator('.mtop > select.repo + .notify')).toHaveCount(1);
    expect(mock.requests.filter((r) => r.startsWith('GET /api/repos/o/a/push'))).toEqual([]);
  });

  // req:web/push-in-the-browser@06ad4e4a
  test('a key that fails to load hides the switch, and it is asked for again by the next refresh', async ({ page, context }) => {
    await page.clock.install();
    const mock = await mockApi(context);
    mock.fail('GET /api/push/key', 500);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    await expect.poll(() => mock.requests.filter((r) => r === 'GET /api/push/key').length).toBe(1);
    await expect(page.locator('.sidebar .notify')).toBeHidden();
    mock.fail('GET /api/push/key', null);
    mock.routes['GET /api/push/key'] = { key: 'k' };
    await page.clock.runFor(60_000);
    await expect(page.locator('.sidebar .notify input[type=checkbox]')).toBeVisible();
    expect(mock.requests.filter((r) => r === 'GET /api/push/key')).toHaveLength(2);
  });

  // req:web/push-in-the-browser@06ad4e4a
  test("a notice's refresh asks the push route with its fresh", async ({ page, context }) => {
    const endpoint = 'https://fcm.googleapis.com/fcm/send/1';
    await page.addInitScript((endpoint) => {
      const sub = { endpoint, toJSON: () => ({ endpoint }), unsubscribe: async () => true };
      PushManager.prototype.getSubscription = async () => sub as unknown as PushSubscription;
    }, endpoint);
    const sockets: import('@playwright/test').WebSocketRoute[] = [];
    await page.routeWebSocket('**/api/repos/o/a/live', (ws) => void sockets.push(ws));
    const mock = await mockApi(context);
    mock.routes['GET /api/push/key'] = { key: 'k' };
    mock.routes['GET /api/repos/o/a/push'] = { subscribed: true };
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.sidebar .notify input[type=checkbox]')).toBeChecked();
    await expect.poll(() => sockets.length).toBe(1);
    sockets[0].send(JSON.stringify({ delivery: 'd1' }));
    await expect.poll(() => mock.urls).toContain(`GET /api/repos/o/a/push?endpoint=${encodeURIComponent(endpoint)}&fresh=d1`);
  });

  // req:web/push-in-the-browser@06ad4e4a
  test('no Web Push: without a PushManager, or with a service worker that fails to register, no switch and no key asked', async ({ browser }) => {
    for (const lacking of [
      () => void delete (window as unknown as { PushManager?: unknown }).PushManager,
      () => {
        // Registration fails, while `ready` still answers a push manager: only the app's own
        // check of the registration keeps the key from being asked for.
        ServiceWorkerContainer.prototype.register = () => Promise.reject(new Error('refused'));
        const pushManager = { getSubscription: async () => null, subscribe: async () => null };
        Object.defineProperty(ServiceWorkerContainer.prototype, 'ready', { configurable: true, get: () => Promise.resolve({ pushManager }) });
      },
    ]) {
      const context = await browser.newContext({ serviceWorkers: 'allow', viewport: { width: 1280, height: 800 } });
      try {
        const page = await context.newPage();
        await page.addInitScript(lacking);
        const mock = await mockApi(context);
        mock.routes['GET /api/push/key'] = { key: 'k' };
        mock.routes['GET /api/repos/o/a/push'] = { subscribed: true };
        await page.goto('/app/#/o/a/graph');
        await expect(page.locator('.n[data-node=b]')).toBeVisible();
        // Settled: the implementation state has ringed the graph, the service worker is ready,
        // and a moment more has passed.
        await expect(page.locator('.n[data-node=b]')).toHaveClass(/\bq-claimed\b/);
        await page.evaluate(() => navigator.serviceWorker.ready.then(() => undefined));
        await page.waitForTimeout(500);
        await expect(page.locator('.sidebar .notify')).toBeHidden();
        expect(mock.requests.filter((r) => r === 'GET /api/push/key' || r.startsWith('GET /api/repos/o/a/push'))).toEqual([]);
      } finally {
        await context.close();
      }
    }
  });

  // req:web/push-in-the-browser@06ad4e4a
  test('turn on: subscribes with the key, posts the subscription, then reads its state from the push route', async ({ page, context }) => {
    const endpoint = 'https://fcm.googleapis.com/fcm/send/1';
    // The browser's push service, faked: permission granted, and a subscription once asked.
    await page.addInitScript((endpoint) => {
      Object.defineProperty(Notification, 'permission', { get: () => 'granted' });
      let sub: unknown = null;
      const calls: unknown[] = [];
      (window as unknown as { pushCalls: unknown[] }).pushCalls = calls;
      PushManager.prototype.getSubscription = async () => sub as PushSubscription;
      PushManager.prototype.subscribe = async (o) => {
        calls.push(o);
        sub = { endpoint, toJSON: () => ({ endpoint, keys: { p256dh: 'p', auth: 'a' } }), unsubscribe: async () => true };
        return sub as PushSubscription;
      };
    }, endpoint);
    const mock = await mockApi(context);
    mock.routes['GET /api/push/key'] = { key: 'k' };
    mock.routes['GET /api/repos/o/a/push'] = { subscribed: false };
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    const toggle = page.locator('.sidebar .notify input[type=checkbox]');
    await expect(toggle).toBeEnabled();
    mock.onPost = () => (mock.routes['GET /api/repos/o/a/push'] = { subscribed: true });
    await toggle.click();
    await expect(toggle).toBeChecked();
    expect(await page.evaluate(() => (window as unknown as { pushCalls: unknown[] }).pushCalls)).toEqual([{ userVisibleOnly: true, applicationServerKey: 'k' }]);
    expect(mock.posts).toEqual([{ key: 'POST /api/repos/o/a/push', body: JSON.stringify({ endpoint, keys: { p256dh: 'p', auth: 'a' } }) }]);
    // A redraw asks the push route for this browser's endpoint.
    await page.locator('.sidebar .nv[data-view=backlog]').click();
    await expect.poll(() => mock.urls).toContain(`GET /api/repos/o/a/push?endpoint=${encodeURIComponent(endpoint)}`);
    await expect(page.locator('.sidebar .notify input[type=checkbox]')).toBeChecked();
  });

  // req:web/push-in-the-browser@06ad4e4a
  test('signed out on a turn-on: a 401 to the push route shows the sign-in page', async ({ page, context }) => {
    await page.addInitScript(() => {
      Object.defineProperty(Notification, 'permission', { get: () => 'granted' });
      const endpoint = 'https://fcm.googleapis.com/fcm/send/1';
      let sub: unknown = null;
      PushManager.prototype.getSubscription = async () => sub as PushSubscription;
      PushManager.prototype.subscribe = async () =>
        (sub = { endpoint, toJSON: () => ({ endpoint, keys: { p256dh: 'p', auth: 'a' } }), unsubscribe: async () => true }) as unknown as PushSubscription;
    });
    const mock = await mockApi(context);
    mock.routes['GET /api/push/key'] = { key: 'k' };
    mock.fail('POST /api/repos/o/a/push', 401);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    const toggle = page.locator('.sidebar .notify input[type=checkbox]');
    await expect(toggle).toBeEnabled();
    await toggle.click();
    await expect(page.locator('.signin a', { hasText: 'Sign in with GitHub' })).toHaveAttribute('href', '/auth/login');
    // A 401 is tried once more (an expired session another request refreshed), and nothing else is written.
    expect(mock.posts.map((p) => p.key)).toEqual(['POST /api/repos/o/a/push', 'POST /api/repos/o/a/push']);
  });

  // req:web/push-in-the-browser@06ad4e4a
  test('storage that throws: drawn on from the push route, and turned off with one DELETE and no unsubscribe', async ({ page, context }) => {
    const endpoint = 'https://fcm.googleapis.com/fcm/send/1';
    await page.addInitScript((endpoint) => {
      const fail = () => {
        throw new DOMException('denied', 'SecurityError');
      };
      Object.defineProperty(window, 'localStorage', { configurable: true, get: () => ({ getItem: fail, setItem: fail, removeItem: fail, clear: fail, key: fail, length: 0 }) });
      Object.defineProperty(Notification, 'permission', { get: () => 'granted' });
      const w = window as unknown as { unsubscribed: number };
      w.unsubscribed = 0;
      // A subscription the browser holds from page load.
      const sub = { endpoint, toJSON: () => ({ endpoint, keys: { p256dh: 'p', auth: 'a' } }), unsubscribe: async () => ((w.unsubscribed += 1), true) };
      PushManager.prototype.getSubscription = async () => sub as unknown as PushSubscription;
    }, endpoint);
    const mock = await mockApi(context);
    mock.routes['GET /api/push/key'] = { key: 'k' };
    mock.routes['GET /api/repos/o/a/push'] = { subscribed: true };
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    const toggle = page.locator('.sidebar .notify input[type=checkbox]');
    await expect(toggle).toBeChecked();
    await expect(toggle).toBeEnabled();
    mock.onPost = () => (mock.routes['GET /api/repos/o/a/push'] = { subscribed: false });
    await toggle.click();
    await expect(toggle).not.toBeChecked();
    await expect(toggle).toBeEnabled();
    expect(mock.posts.map((p) => p.key)).toEqual(['DELETE /api/repos/o/a/push']);
    expect(await page.evaluate(() => (window as unknown as { unsubscribed: number }).unsubscribed)).toBe(0);
  });
});

// req:web/push-in-the-browser@06ad4e4a
test('the service worker imports the push handlers and shows a push', async ({ page, context, baseURL }) => {
  await mockApi(context);
  // The worker, not a view: drawing one (the graph's layout above all) is the slowest step of a
  // loaded run and proves nothing about the worker, whose `ready` this awaits directly.
  await page.goto('/app/');
  const sw = await page.evaluate(async () => {
    const reg = await navigator.serviceWorker.ready;
    return { script: reg.active?.scriptURL ?? null, scope: reg.scope };
  });
  expect(new URL(sw.script!).pathname).toBe('/app/sw.js');
  expect(new URL(sw.scope).pathname).toBe('/app/');
  const worker = context.serviceWorkers().find((w) => w.url().endsWith('/app/sw.js')) ?? (await context.waitForEvent('serviceworker'));
  // Shows the push in the real service worker, through the module it imports.
  const shown = await worker.evaluate(async () => {
    const event = new Event('push') as Event & { data: unknown; waitUntil(p: Promise<unknown>): void };
    const waits: Promise<unknown>[] = [];
    event.data = { json: () => ({ title: 'o/a: waiting on you', body: 'Which region?', url: '/app/#/o/a/backlog' }) };
    event.waitUntil = (p) => void waits.push(p);
    const scope = self as unknown as { registration: ServiceWorkerRegistration; dispatchEvent(e: Event): boolean };
    const calls: [string, NotificationOptions | undefined][] = [];
    scope.registration.showNotification = async (title: string, options?: NotificationOptions) => void calls.push([title, options]);
    scope.dispatchEvent(event);
    await Promise.all(waits);
    return calls.map(([t, o]) => ({ title: t, body: o?.body, tag: o?.tag, icon: o?.icon }));
  });
  expect(shown).toEqual([{ title: 'o/a: waiting on you', body: 'Which region?', tag: '/app/#/o/a/backlog', icon: '/app/icon-192.png' }]);
  // A click on it, with no app window, opens one at its url.
  const opened = await worker.evaluate(async () => {
    const event = new Event('notificationclick') as Event & { notification: unknown; waitUntil(p: Promise<unknown>): void };
    const waits: Promise<unknown>[] = [];
    event.notification = { data: { url: '/app/#/o/a/backlog' }, close: () => undefined };
    event.waitUntil = (p) => void waits.push(p);
    const scope = self as unknown as { clients: Clients; dispatchEvent(e: Event): boolean };
    const urls: string[] = [];
    scope.clients.matchAll = (async () => []) as unknown as Clients['matchAll'];
    scope.clients.openWindow = async (url: string | URL) => {
      urls.push(String(url));
      return null;
    };
    scope.dispatchEvent(event);
    await Promise.all(waits);
    return urls;
  });
  expect(opened).toEqual([`${baseURL}/app/#/o/a/backlog`]);
});

test.describe('PWA', () => {
  // req:web/installable@9f13442c
  test('it has a manifest with icons, which installs it', async ({ page, request }) => {
    await page.goto('/app/');
    const link = await page.locator('link[rel=manifest]').getAttribute('href');
    const manifest = await (await request.get(link!)).json();
    expect(manifest.name).toBe('CLM');
    expect(manifest.display).toBe('standalone');
    expect(manifest.start_url).toBe('/app/');
    const sizes = manifest.icons.map((i: { sizes: string }) => i.sizes);
    expect(sizes).toContain('192x192');
    expect(sizes).toContain('512x512');
    for (const icon of manifest.icons) {
      const res = await request.get(icon.src);
      expect(res.status(), icon.src).toBe(200);
      expect(res.headers()['content-type']).toContain(icon.type);
    }
  });

  // req:web/installable@9f13442c
  test('offline launch: each view viewed once online shows its last data with its age, every write disabled', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos/o/a/snapshot'] = withQuestion();
    const mine = { id: 'p1', writer: 'alice/web', at: '2026-10-27T10:00:00Z', slug: 'export-csv', title: 'Export CSV', body: 'Let me export.', nodes: ['a'], mine: true };
    mock.routes['GET /api/repos/o/a/implementation'] = implementation({ proposals: [mine] });
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await expect(page.locator('button.approve')).toBeEnabled();
    // The service worker has installed and controls the page.
    await page.evaluate(async () => {
      await navigator.serviceWorker.ready;
      if (!navigator.serviceWorker.controller) {
        await new Promise((r) => navigator.serviceWorker.addEventListener('controllerchange', r, { once: true }));
      }
    });
    // Each view visited once online.
    const views = ['backlog/q', 'backlog', 'capabilities/a', 'architecture/e'];
    for (const v of views) {
      await page.goto(`/app/#/o/a/${v}`);
      await expect(page.locator('.main')).toBeVisible();
      await expect(page.locator('#loadbar')).toBeHidden();
    }
    await expect(page.locator('.capage')).toBeVisible();
    // Then the network goes.
    await context.unroute('**/api/**');
    await context.setOffline(true);
    const banner = /^Offline · showing data from (just now|\d+ min ago)$/;
    const offlineAt = async (route: string) => {
      await page.goto(`/app/#/o/a/${route}`);
      await page.reload();
      await expect(page.locator('.offline')).toHaveText(banner);
    };
    await offlineAt('prs/spec%2Fc');
    await expect(page.locator('.review > .head h1')).toHaveText('spec/c');
    await expect(page.locator('.repo-meta')).toContainText('3 components');
    await expect(page.locator('button.approve')).toBeDisabled();
    await expect(page.locator('.approve-bar button.request')).toBeDisabled();
    await expect(page.locator('.review .intent button.comment')).toBeDisabled();
    await offlineAt('backlog/q');
    await expect(page.locator('h1')).toContainText('Ship it?');
    await expect(page.locator('button.answer[data-answer=yes]')).toBeDisabled();
    await offlineAt('backlog');
    await expect(page.locator('.prop[data-id=p1]')).toContainText('Export CSV');
    await expect(page.locator('button.propose')).toBeDisabled();
    await expect(page.locator('.prop[data-id=p1] button.withdraw')).toBeDisabled();
    for (const route of ['capabilities/a', 'architecture/e']) {
      await offlineAt(route);
      // With a value chosen, only being offline keeps the decision buttons off.
      await expect(page.locator('section[data-sec=entry]')).toContainText(route.startsWith('capabilities') ? 'Without a, nothing works.' : 'Partly judgement.');
      await page.locator('button.value', { hasText: 'low' }).click();
      for (const b of await page.locator('button.decide').all()) {
        await expect(b).toBeDisabled();
        await expect(b).toHaveAttribute('title', 'Offline');
      }
    }
  });

  // req:web/installable@9f13442c
  test('nothing kept: a repo never opened on this device, opened offline, reads Offline', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos'] = [
      { owner: 'o', name: 'a', default_branch: 'main' },
      { owner: 'o', name: 'b', default_branch: 'main' },
    ];
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await page.evaluate(async () => {
      await navigator.serviceWorker.ready;
      if (!navigator.serviceWorker.controller) {
        await new Promise((r) => navigator.serviceWorker.addEventListener('controllerchange', r, { once: true }));
      }
    });
    await context.unroute('**/api/**');
    await context.setOffline(true);
    await page.goto('/app/#/o/b/graph');
    await page.reload();
    await expect(page.locator('.offline')).toHaveText('Offline');
  });

  // req:web/answer-from-app@ba9bce6f
  test('offline: an item page drawn from the kept copy has its answer buttons disabled', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos/o/a/snapshot'] = withQuestion();
    await page.goto('/app/#/o/a/backlog/q');
    await expect(page.locator('button.answer[data-answer=yes]')).toBeEnabled();
    await page.evaluate(async () => {
      await navigator.serviceWorker.ready;
      if (!navigator.serviceWorker.controller) {
        await new Promise((r) => navigator.serviceWorker.addEventListener('controllerchange', r, { once: true }));
      }
    });
    await context.unroute('**/api/**');
    await context.setOffline(true);
    await page.reload();
    await expect(page.locator('.offline')).toBeVisible();
    await expect(page.locator('button.answer[data-answer=yes]')).toBeDisabled();
  });
});

test.describe('comments', () => {
  // req:web/review-in-the-browser@9b9f5325
  test('comment on a requirement: one post of the fingerprint, target and text, the review asked again, and the card shows the comment by alice', async ({ page, context }) => {
    const mock = await mockApi(context);
    const said = { id: 'c1', writer: 'alice/web', at: '2026-10-27T10:00:00Z', fingerprint: 'f1', target: 'n/r', text: 'Name the rule', current: true };
    mock.onPost = () => (mock.routes[REVIEW] = review({ comments: [said] }));
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    const card = page.locator('.req[data-el="n/r"]');
    await card.locator('button.comment').click();
    await card.locator('textarea').fill('Name the rule');
    await card.locator('button.send').click();
    await expect(card.locator('.fb')).toContainText('alice');
    await expect(card.locator('.fbform')).toHaveCount(0);
    expect(mock.posts).toEqual([{ key: COMMENTS, body: '{"fingerprint":"f1","target":"n/r","text":"Name the rule"}' }]);
    expect(mock.requests.filter((r) => r === REVIEW)).toHaveLength(2);
  });

  // req:web/review-in-the-browser@9b9f5325
  test("a 422 shows the Worker's reason under the form and keeps it", async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(COMMENTS, 422, { error: 'text' });
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    const intent = page.locator('.review .intent');
    await intent.locator('button.comment').click();
    await intent.locator('textarea').fill('Why now?');
    await intent.locator('button.send').click();
    await expect(intent.locator('.fbform .err')).toHaveText('Not sent: the Worker refused the text.');
    await expect(intent.locator('textarea')).toHaveValue('Why now?');
    await expect(intent.locator('button.send')).toBeEnabled();
  });
});

test.describe('Request changes', () => {
  const requested = { id: 'q1', writer: 'alice/web', at: '2026-10-27T12:00:00Z', fingerprint: 'f1', text: 'Split it', current: true, held: true };

  // req:web/review-in-the-browser@9b9f5325
  test('requested: two quick submits post once, the review is asked again, and the header says changes requested by alice', async ({ page, context }) => {
    const mock = await mockApi(context, review({ signed: true }));
    mock.onPost = () => (mock.routes[REVIEW] = review({ signed: true, change_requests: [requested] }));
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('.approve-bar button.request').click();
    const form = page.locator('.approve-bar .fbform');
    await form.locator('textarea').fill('Split it');
    mock.hold(REQUEST);
    await form.locator('button.send').click();
    await expect.poll(() => mock.requests.filter((r) => r === REQUEST).length).toBe(1);
    await form.locator('button.send').click({ force: true });
    mock.release(REQUEST);
    const held = page.locator('.review > .head .changes-held');
    await expect(held).toContainText('Changes requested by alice');
    await expect(held).toContainText('Split it');
    expect(mock.posts).toEqual([{ key: REQUEST, body: '{"fingerprint":"f1","text":"Split it"}' }]);
    expect(mock.requests.filter((r) => r === REVIEW)).toHaveLength(2);
  });

  // req:web/review-in-the-browser@9b9f5325
  test('branch moved: a 409 says the branch changed, draws the review with fingerprint f2 and keeps the text', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(REQUEST, 409, { error: 'moved' });
    mock.onPost = () => (mock.routes[REVIEW] = review({ fingerprint: 'f2', change: { ...review().change!, intent_section: 'Make b better, now.' } }));
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('.approve-bar button.request').click();
    await page.locator('.approve-bar .fbform textarea').fill('Split it');
    await page.locator('.approve-bar .fbform button.send').click();
    await expect(page.locator('.review')).toContainText('The branch changed since this review was shown');
    await expect(page.locator('.review .intent')).toContainText('Make b better, now.');
    // Nothing was recorded, so the text stays for the review now on screen, and the next
    // send pins it.
    await expect(page.locator('.approve-bar .fbform textarea')).toHaveValue('Split it');
    mock.fail(REQUEST, null);
    await page.locator('.approve-bar .fbform button.send').click();
    await expect.poll(() => mock.posts.map((p) => p.body)).toEqual(['{"fingerprint":"f1","text":"Split it"}', '{"fingerprint":"f2","text":"Split it"}']);
  });

  test.describe('on a phone', () => {
    test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

    // req:web/review-in-the-browser@9b9f5325
    test('Request changes is in the Approve bar beside Approve, and its form stays in view', async ({ page, context }) => {
      await mockApi(context);
      await page.goto('/app/#/o/a/prs/spec%2Fc');
      const request = page.locator('.approve-bar button.request');
      await expect(request).toBeInViewport({ ratio: 1 });
      await expect(page.locator('.approve-bar button.approve')).toBeInViewport({ ratio: 1 });
      await request.click();
      await expect(page.locator('.approve-bar .fbform button.send')).toBeInViewport({ ratio: 1 });
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    });
  });

  // req:web/review-in-the-browser@9b9f5325
  test('a 403 says write access is needed and leaves Request changes usable', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(REQUEST, 403, { error: 'permission' });
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('.approve-bar button.request').click();
    await page.locator('.approve-bar .fbform textarea').fill('Split it');
    await page.locator('.approve-bar .fbform button.send').click();
    await expect(page.locator('.approve-bar .fbform .err')).toContainText('needs write access');
    await expect(page.locator('.approve-bar .fbform button.send')).toBeEnabled();
  });

  // req:web/review-in-the-browser@9b9f5325
  test("a comment's 403 says read access is needed, and the form stays usable with its text", async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(COMMENTS, 403, { error: 'permission' });
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    const intent = page.locator('.review .intent');
    await intent.locator('button.comment').click();
    await intent.locator('textarea').fill('Why now?');
    await intent.locator('button.send').click();
    await expect(intent.locator('.fbform .err')).toContainText('needs read access');
    await expect(intent.locator('textarea')).toHaveValue('Why now?');
    await expect(intent.locator('button.send')).toBeEnabled();
  });

  // req:web/review-in-the-browser@9b9f5325
  test('a 422 naming no field says the Worker refused the request, and a 500 says Not sent (500)', async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('.approve-bar button.request').click();
    const form = page.locator('.approve-bar .fbform');
    await form.locator('textarea').fill('Split it');
    for (const [status, says] of [
      [422, 'Not sent: the Worker refused the request.'],
      [500, 'Not sent (500).'],
    ] as const) {
      mock.fail(REQUEST, status, status === 422 ? {} : undefined);
      await form.locator('button.send').click();
      await expect(form.locator('.err')).toHaveText(says);
      await expect(form.locator('textarea')).toHaveValue('Split it');
      await expect(form.locator('button.send')).toBeEnabled();
    }
  });
});

test.describe('proposals', () => {
  const IMPL = 'GET /api/repos/o/a/implementation';
  const proposal = (over = {}) => ({ id: 'p1', writer: 'alice/web', at: '2026-10-27T10:00:00Z', slug: 'export-csv', title: 'Export CSV', body: 'Let me export.', nodes: ['a'], mine: true, ...over });

  // req:web/backlog-in-the-browser@5ad99bbe
  test('proposed: two quick submits post once, the form closes, and Proposed lists it by alice with the note', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.onPost = () => (mock.routes[IMPL] = implementation({ proposals: [proposal()] }));
    await page.goto('/app/#/o/a/backlog');
    await page.locator('button.propose').click();
    const form = page.locator('.fbform.proposal');
    await form.locator('input[name=title]').fill('Export CSV');
    await form.locator('textarea').fill('Let me export.');
    // The answer is held, so the second tap lands while the first is in flight.
    mock.hold(PROPOSALS);
    await form.locator('button.send').click();
    await expect.poll(() => mock.requests.filter((r) => r === PROPOSALS).length).toBe(1);
    await form.locator('button.send').click({ force: true });
    mock.release(PROPOSALS);
    await expect(page.locator('.fbform.proposal')).toHaveCount(0);
    const row = page.locator('.proposed .prop[data-id=p1]');
    await expect(row).toContainText('Export CSV');
    await expect(row).toContainText('alice');
    await expect(row).toContainText('next build files it');
    expect(mock.posts).toEqual([{ key: PROPOSALS, body: '{"title":"Export CSV","body":"Let me export.","nodes":[]}' }]);
    expect(mock.requests.filter((r) => r === IMPL).length).toBeGreaterThanOrEqual(2);
  });

  // req:web/backlog-in-the-browser@5ad99bbe
  test('title taken: a 409 keeps the form open, saying so', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(PROPOSALS, 409, { error: 'taken' });
    await page.goto('/app/#/o/a/backlog');
    await page.locator('button.propose').click();
    await page.locator('.fbform.proposal input[name=title]').fill('Export CSV');
    await page.locator('.fbform.proposal button.send').click();
    await expect(page.locator('.fbform.proposal .err')).toHaveText('That title is taken: choose another.');
    await expect(page.locator('.fbform.proposal input[name=title]')).toHaveValue('Export CSV');
  });

  // req:web/backlog-in-the-browser@5ad99bbe
  test("a 422 names the Worker's field and keeps the form", async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(PROPOSALS, 422, { error: 'nodes' });
    await page.goto('/app/#/o/a/backlog');
    await page.locator('button.propose').click();
    await page.locator('.fbform.proposal input[name=title]').fill('Export CSV');
    await page.locator('.fbform.proposal button.send').click();
    await expect(page.locator('.fbform.proposal .err')).toHaveText('Not proposed: the Worker refused the nodes.');
    // A 422 naming no field refuses the request, and any other status is named; each keeps the form.
    for (const [status, body, says] of [
      [422, {}, 'Not proposed: the Worker refused the request.'],
      [500, undefined, 'Not proposed (500).'],
    ] as const) {
      mock.fail(PROPOSALS, status, body);
      await page.locator('.fbform.proposal button.send').click();
      await expect(page.locator('.fbform.proposal .err')).toHaveText(says);
      await expect(page.locator('.fbform.proposal input[name=title]')).toHaveValue('Export CSV');
      await expect(page.locator('.fbform.proposal button.send')).toBeEnabled();
    }
    expect(mock.requests.filter((r) => r === IMPL)).toHaveLength(1);
  });

  // req:web/backlog-in-the-browser@5ad99bbe
  test('withdraw: only p1 carries Withdraw, one DELETE goes to its id, and the implementation state is asked again', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes[IMPL] = implementation({ proposals: [proposal(), proposal({ id: 'p2', writer: 'bob/web', mine: false, title: 'Other', slug: 'other' })] });
    await page.goto('/app/#/o/a/backlog');
    await expect(page.locator('.prop[data-id=p2]')).toBeVisible();
    await expect(page.locator('.prop[data-id=p2] button.withdraw')).toHaveCount(0);
    const before = mock.requests.filter((r) => r === IMPL).length;
    mock.onPost = () => (mock.routes[IMPL] = implementation({ proposals: [proposal({ id: 'p2', writer: 'bob/web', mine: false, title: 'Other', slug: 'other' })] }));
    await page.locator('.prop[data-id=p1] button.withdraw').click();
    await expect(page.locator('.prop[data-id=p1]')).toHaveCount(0);
    expect(mock.posts).toEqual([{ key: 'DELETE /api/repos/o/a/proposals/p1', body: null }]);
    expect(mock.requests.filter((r) => r === IMPL).length).toBe(before + 1);
  });

  // req:web/backlog-in-the-browser@5ad99bbe
  test('a withdrawal refused: the status shows under p1, which stays, and nothing is asked again', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes[IMPL] = implementation({ proposals: [proposal()] });
    mock.fail('DELETE /api/repos/o/a/proposals/p1', 409);
    await page.goto('/app/#/o/a/backlog');
    const row = page.locator('.prop[data-id=p1]');
    await expect(row.locator('button.withdraw')).toBeEnabled();
    await row.locator('button.withdraw').click();
    await expect(row.locator('.err')).toHaveText('Not withdrawn (409).');
    await expect(row).toContainText('Export CSV');
    await expect(row.locator('button.withdraw')).toBeEnabled();
    expect(mock.posts).toEqual([{ key: 'DELETE /api/repos/o/a/proposals/p1', body: null }]);
    const after = mock.requests.slice(mock.requests.indexOf('DELETE /api/repos/o/a/proposals/p1') + 1);
    expect(after.filter((r) => r.startsWith(IMPL))).toEqual([]);
  });
});

test.describe('latest navigation wins', () => {
  // req:web/latest-navigation-wins@ed849c57
  test('opened at the app root: the route becomes the first repo graph', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos'] = [
      { owner: 'o', name: 'a', default_branch: 'main' },
      { owner: 'o', name: 'b', default_branch: 'main' },
    ];
    await page.goto('/app/');
    await expect(page.locator('.n[data-node=a]')).toBeVisible();
    expect(new URL(page.url()).hash).toBe('#/o/a/graph');
  });

  // req:web/latest-navigation-wins@ed849c57
  test('a navigation made while the repo list is pending is not undone by its answer', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.hold('GET /api/repos');
    await page.goto('/app/');
    await page.evaluate(() => (location.hash = '#/o/a/prs/spec%2Fc'));
    mock.release('GET /api/repos');
    await expect(page.locator('button.approve')).toBeVisible();
    await page.waitForTimeout(300);
    expect(new URL(page.url()).hash).toBe('#/o/a/prs/spec%2Fc');
    await expect(page.locator('button.approve')).toBeVisible();
  });

  // req:web/latest-navigation-wins@ed849c57
  test('no repos: the page says No CLM repos yet and links to installing the App, the route unchanged', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos'] = [];
    await page.goto('/app/');
    await expect(page.locator('h1')).toHaveText('No CLM repos yet');
    await expect(page.locator('a', { hasText: 'Install the GitHub App' })).toHaveAttribute('href', 'https://github.com/apps/clm-spec-review/installations/new');
    expect(new URL(page.url()).hash).toBe('');
  });
});

test.describe('links to the site', () => {
  // req:web/links-to-site@110ad07e
  test('desktop sidebar: the logo links to / and Guide sits beside Add repos', async ({ page, context }) => {
    await mockApi(context);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=a]')).toBeVisible();
    await expect(page.locator('.sidebar a.logo')).toHaveAttribute('href', '/');
    const guide = page.locator('.sidebar a', { hasText: /^Guide$/ });
    await expect(guide).toHaveAttribute('href', '/guide/');
    const sameFoot = await page.evaluate(() => {
      const links = [...document.querySelectorAll('.sidebar a')];
      const g = links.find((a) => a.textContent === 'Guide');
      const add = links.find((a) => a.textContent === 'Add repos');
      return g !== undefined && add !== undefined && g.parentElement === add.parentElement;
    });
    expect(sameFoot).toBe(true);
  });

  // req:web/links-to-site@110ad07e
  test('phone top bar: begins with a homepage link before the repo switcher', async ({ page, context }) => {
    await mockApi(context);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=a]')).toBeVisible();
    const first = page.locator('.mtop > *').first();
    await expect(first).toHaveAttribute('href', '/');
    await expect(first).toHaveAttribute('aria-label', /CLM homepage/);
    await expect(first).toBeVisible();
    expect(await page.locator('.mtop > *').evaluateAll((els) => els.findIndex((e) => e.tagName === 'SELECT'))).toBe(1);
  });
});

// req:web/sign-in-early-access@c4497420
test('the sign-in page says CLM works without the web app and links to early access', async ({ page, context }) => {
  await context.route('**/api/**', (route) => route.fulfill({ status: 401, contentType: 'application/json', body: '{}' }));
  await page.goto('/app/');
  const card = page.locator('.signin');
  await expect(card).toBeVisible();
  await expect(card.locator('a.logo')).toHaveAttribute('href', '/');
  await expect(card).toContainText('CLM works fully without the web app');
  await expect(card).toContainText('optional extra');
  await expect(card).toContainText('early access');
  await expect(card.locator('a[href="/guide/quickstart"]')).toBeVisible();
  await expect(card.locator('a', { hasText: 'Request early access' })).toHaveAttribute('href', 'https://github.com/clunny/clm/issues/new?template=early-access.yml');
  await expect(card.locator('a', { hasText: 'Sign in with GitHub' })).toHaveAttribute('href', '/auth/login');
  await expect(card.locator('a', { hasText: 'Install the GitHub App' })).toHaveAttribute('href', 'https://github.com/apps/clm-spec-review/installations/new');
});

test.describe('a backlog item opens its page', () => {
  // req:web/backlog-in-the-browser@5ad99bbe
  test('tapping an item in the Backlog and in the inspector shows its page', async ({ page, context }) => {
    await mockApi(context);
    await page.goto('/app/#/o/a/backlog');
    await page.locator('.bi[data-slug=later] a.ttl').click();
    await expect(page).toHaveURL(/#\/o\/a\/backlog\/later$/);
    await expect(page.locator('.bipage h1')).toHaveText('Do it later');
    await page.goto('/app/#/o/a/backlog/gone');
    await expect(page.locator('.bipage')).toContainText("isn't in the backlog");
    await page.locator('.bipage .empty a').click();
    await expect(page).toHaveURL(/#\/o\/a\/backlog$/);
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto('/app/#/o/a/graph');
    await page.locator('.n[data-node=b]').click();
    await page.locator('.insp .backlog a.mini').click();
    await expect(page).toHaveURL(/#\/o\/a\/backlog\/later$/);
  });
});

test.describe('Home first', () => {
  test.use({ viewport: { width: 1280, height: 800 } });

  // req:web/home-first@fb20db87
  test('a repo opens on Home, the sidebar\'s first view, badged with what waits', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos'] = [
      { owner: 'o', name: 'a', default_branch: 'main' },
      { owner: 'o', name: 'b', default_branch: 'main' },
    ];
    mock.routes['GET /api/repos/o/b/snapshot'] = snapshot({ repo: 'o/b' });
    mock.routes['GET /api/repos/o/b/branches'] = [];
    mock.routes['GET /api/repos/o/b/implementation'] = implementation();
    await page.goto('/app/#/o/b/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await page.locator('.sidebar select.repo').selectOption('o/a');
    await expect(page).toHaveURL(/#\/o\/a$/);
    await expect(page.locator('.home h1')).toHaveText('Home');
    await expect(page.locator('.sidebar .nv[data-view]').first()).toHaveAttribute('data-view', 'home');
    await expect(page.locator('.sidebar .nv[data-view=home]')).toHaveClass(/\bon\b/);
    // The human task `setup` waits on the person, and so does spec/c's review.
    await expect(page.locator('.sidebar .nv[data-view=home] .c')).toHaveText('2');
    await expect(page.locator('[data-section=review] a.hent')).toHaveAttribute('href', '#/o/a/prs/spec%2Fc');
    await expect(page.locator('[data-section=answer] a.hent')).toHaveAttribute('href', '#/o/a/backlog/setup');
  });

  // req:web/home-first@fb20db87
  test('Home is drawn from the snapshot at once and adds each section as its response arrives', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.hold('GET /api/repos/o/a/implementation');
    mock.hold(CAPABILITIES);
    mock.hold(ARCHITECTURE);
    await page.goto('/app/#/o/a');
    await expect(page.locator('.home h1')).toBeVisible();
    await expect(page.locator('[data-section=answer] button.answer')).toHaveCount(1);
    await expect(page.locator('[data-section=flagged]')).toHaveCount(0);
    mock.release(CAPABILITIES);
    await expect(page.locator('[data-section=flagged]')).toBeVisible();
    mock.release(ARCHITECTURE);
    mock.release('GET /api/repos/o/a/implementation');
  });

  // req:web/home-first@fb20db87
  test('a backlog item\'s page opens at its route', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos/o/a/snapshot'] = withQuestion();
    await page.goto('/app/#/o/a/backlog/q');
    await expect(page.locator('h1')).toContainText('Ship it?');
    await expect(page.locator('strong')).toHaveText('ship');
    await expect(page.locator('button.answer[data-answer=yes]')).toBeVisible();
  });

  // req:web/answer-from-app@ba9bce6f
  test('answered on the item\'s page: one post, the implementation state asked again with fresh, and yes drawn chosen', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos/o/a/snapshot'] = withQuestion();
    mock.onPost = () => {
      const impl = implementation();
      impl.answers = [{ id: 'x1', writer: 'alice/web', at: '2026-11-13T00:00:00Z', item: 'q', answer: 'yes', mine: true }];
      mock.routes['GET /api/repos/o/a/implementation'] = impl;
    };
    await page.goto('/app/#/o/a/backlog/q');
    await page.locator('button.answer[data-answer=yes]').dblclick();
    await expect(page.locator('button.answer[data-answer=yes]')).toHaveClass(/chosen/);
    expect(mock.posts).toEqual([{ key: ANSWER, body: '{"answer":"yes"}' }]);
    // The implementation state is asked for again, from the Worker anew, so the answer shows from its `answers`.
    const asked = mock.urls.filter((k) => k.startsWith('GET /api/repos/o/a/implementation'));
    expect(asked.length).toBeGreaterThan(1);
    expect(asked.at(-1)).toMatch(/\?fresh=/);
  });

  // req:web/answer-from-app@ba9bce6f
  test('an answer the Worker refuses says why and keeps the buttons enabled', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos/o/a/snapshot'] = withQuestion();
    mock.fail(ANSWER, 422, { error: 'answer' });
    await page.goto('/app/#/o/a');
    await expect(page.locator('button.answer[data-answer=no]')).toBeVisible();
    const before = mock.requests.filter((k) => k === 'GET /api/repos/o/a/implementation').length;
    await page.locator('button.answer[data-answer=no]').click();
    await expect(page.locator('.hent-row', { hasText: 'Ship it?' }).locator('.err')).toContainText('The answer was refused.');
    await expect(page.locator('button.answer[data-answer=no]')).toBeEnabled();
    // A refused answer asks for nothing again.
    expect(mock.requests.filter((k) => k === 'GET /api/repos/o/a/implementation').length).toBe(before);
  });

  // req:web/answer-from-app@ba9bce6f
  test('answered on Home: one post to q\'s route, and Home draws yes chosen', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos/o/a/snapshot'] = withQuestion();
    mock.onPost = () => {
      const impl = implementation();
      impl.answers = [{ id: 'x1', writer: 'alice/web', at: '2026-11-13T00:00:00Z', item: 'q', answer: 'yes', mine: true }];
      mock.routes['GET /api/repos/o/a/implementation'] = impl;
    };
    await page.goto('/app/#/o/a');
    await page.locator('button.answer[data-answer=yes]').click();
    await expect(page.locator('.home button.answer[data-answer=yes]')).toHaveClass(/chosen/);
    expect(mock.posts).toEqual([{ key: ANSWER, body: '{"answer":"yes"}' }]);
  });

  // req:web/answer-from-app@ba9bce6f
  test('refused on the item\'s page: a 422, a 404 and a 500 each say why under the buttons, which stay enabled, and nothing is asked again', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes['GET /api/repos/o/a/snapshot'] = withQuestion();
    await page.goto('/app/#/o/a/backlog/q');
    const yes = page.locator('button.answer[data-answer=yes]');
    await expect(yes).toBeEnabled();
    const asked = () => mock.requests.filter((k) => k === 'GET /api/repos/o/a/implementation').length;
    await expect.poll(asked).toBeGreaterThan(0);
    const before = asked();
    const cases: [number, unknown, string][] = [
      [422, { error: 'answer' }, 'The answer was refused.'],
      [404, undefined, 'That item is no longer in the backlog.'],
      [500, undefined, "The answer wasn't recorded (500)."],
    ];
    for (const [status, body, says] of cases) {
      mock.fail(ANSWER, status, body);
      await yes.click();
      await expect(page.locator('.answers .err')).toHaveText(says);
      await expect(yes).toBeEnabled();
    }
    expect(mock.posts).toHaveLength(3);
    expect(asked()).toBe(before);
  });

  // req:web/home-first@fb20db87
  test('the phone tab bar badges Home with what waits', async ({ page, context }) => {
    await mockApi(context);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/app/#/o/a');
    await expect(page.locator('.tabbar a[data-view=home] .bdg')).toHaveText('2');
  });
});
