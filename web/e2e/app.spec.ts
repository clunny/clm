import { expect, test } from '@playwright/test';
import { APPROVE, ARCHITECTURE, ARCH_DECIDE, CAPABILITIES, COMMENTS, DECIDE, PROPOSALS, REQUEST, REVIEW, mockApi, reviewOfB } from './mock';
import { architecture, capabilities, cost, implementation, review, snapshot } from '../test/fixtures';

// req:web/mobile-layout@b86f52c6
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
    await expect(page.locator('.tabbar a[data-view=capabilities]')).toBeVisible();
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

  test('architecture on a phone: six tabs ending Capabilities and Arch., cards, nothing scrolls sideways, and the decision buttons can be reached and tapped', async ({ page, context }) => {
    const mock = await mockApi(context);
    const fits = () => page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth);
    await page.goto('/app/#/o/a/architecture');
    await expect(page.locator('.ccard')).toHaveCount(2);
    await expect(page.locator('.ccard').first()).toBeVisible();
    await expect(page.locator('table.caps')).toBeHidden();
    const tabs = page.locator('.tabbar a');
    await expect(tabs).toHaveCount(6);
    expect((await tabs.allInnerTexts()).slice(-2).map((t) => t.replace(/\s+/g, ' ').trim())).toEqual(['⚖ Capabilities', '▦ Arch.']);
    for (const tab of await tabs.all()) await expect(tab).toBeInViewport({ ratio: 1 });
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

// req:web/capabilities-list@fde4e68b
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
  await expect(page.locator('section[data-sec=why]')).toContainText('Without a, nothing works.');
  await expect(page.locator('.sidebar .nv[data-view=capabilities]')).toHaveClass(/on/);
});

// req:web/architecture-list@d644a9d4
test.describe('architecture list', () => {
  test.use({ viewport: { width: 1280, height: 800 } });

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
    await expect(page.locator('section[data-sec=held]')).toContainText('Partly judgement.');
  });

  test('indexing: the proof reads ? / 2 and the note says coverage is still indexing', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes[ARCHITECTURE] = architecture({ indexing: true });
    await page.goto('/app/#/o/a/architecture');
    await expect(page.locator('tr.cap[data-slug=e] .proof')).toHaveText('? / 2 + untested');
    await expect(page.locator('.cost-note')).toContainText('still indexing');
  });
});

// req:web/architecture-decide-buttons@41e20cf5
test.describe('deciding an architectural constraint', () => {
  test.use({ viewport: { width: 1280, height: 800 } });

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

  test('moved: a 409 says the entry changed and draws it as it is now', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(ARCH_DECIDE, 409);
    mock.onPost = () => {
      const a = architecture();
      a.architecture[0] = { ...a.architecture[0], version: 'feedbeef', statement: 'CLM SHALL e, reworded.' };
      mock.routes[ARCHITECTURE] = a;
    };
    await page.goto('/app/#/o/a/architecture/e');
    await page.locator('button.value', { hasText: 'high' }).click();
    await page.locator('button.decide', { hasText: 'Keep' }).click();
    await expect(page.locator('.decide-note')).toContainText('The entry changed since it was shown');
    await expect(page.locator('.capage')).toContainText('CLM SHALL e, reworded.');
  });

  test('signed: the buttons are disabled and the note gives clm architecture decide', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.routes[ARCHITECTURE] = architecture({ signed: true });
    await page.goto('/app/#/o/a/architecture/e');
    await page.locator('button.value', { hasText: 'high' }).click();
    await expect(page.locator('button.decide').first()).toBeDisabled();
    await expect(page.locator('.signed-note code')).toHaveText('clm architecture decide e --value high|medium|low --decision keep|cut|redesign|accept');
  });
});

// req:web/decide-buttons@c1039d9e
test.describe('deciding a capability', () => {
  test.use({ viewport: { width: 1280, height: 800 } });

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

  test('moved: a 409 says the capability changed and draws it as it is now', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(DECIDE, 409);
    mock.onPost = () => {
      const c = capabilities();
      c.report.capabilities[1] = cost('a', 2, { version: 'feedbeef', statement: 'A person can a, reworded.' });
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
});

test('wide screens get the sidebar and no tab bar', async ({ page, context }) => {
  await mockApi(context);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/app/#/o/a/backlog');
  await expect(page.locator('.sidebar')).toBeVisible();
  await expect(page.locator('.tabbar')).toBeHidden();
  await expect(page.locator('.sidebar .nv[data-view=backlog] .c')).toHaveText('1');
});

// req:web/full-document@fcfa5b34
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

// req:web/graph-view@7bdf4eb6
test.describe('graph', () => {
  test('queue states: once the implementation state arrives, b is ringed amber and named claimed, a rose and blocked', async ({ page, context }) => {
    await mockApi(context);
    await page.goto('/app/#/o/a/graph');
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

  test("a PR's Graph tab marks only the nodes it touches", async ({ page, context }) => {
    await mockApi(context, reviewOfB());
    await page.goto('/app/#/o/a/prs/spec%2Fc/graph');
    await expect(page.locator('.n.hit')).toHaveCount(1);
    await expect(page.locator('.n.hit')).toHaveAttribute('data-node', 'b');
    await expect(page.locator('path.e.eh[data-from=b][data-to=a]')).toHaveCount(1);
  });

  test("a selected node's inspector lists the open PRs that change it", async ({ page, context }) => {
    await mockApi(context, reviewOfB());
    await page.goto('/app/#/o/a/graph');
    await page.locator('.n[data-node=b]').click();
    await expect(page.locator('.insp .inpr')).toContainText('spec/c');
    await page.locator('.n[data-node=a]').click();
    await expect(page.locator('.insp .inpr')).toContainText('No open PR changes it.');
  });
});

// req:web/spec-branch-review@a7d7ac89
test.describe('spec branches', () => {
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

  test('approved by: the header names clunny/web with its time and says not checked, and no request went to /pulls', async ({ page, context }) => {
    const mock = await mockApi(context, review({ approvals: [{ writer: 'clunny/web', at: '2026-10-27T10:00:00Z' }], checked: false }));
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    const head = page.locator('.review > .head');
    await expect(head).toContainText('Approved by clunny/web · 2026-10-27 10:00 UTC');
    await expect(head).toContainText('not checked');
    await page.goto('/app/#/o/a/graph');
    await page.locator('.n[data-node=a]').click();
    await page.goto('/app/#/o/a/build');
    await expect(page.locator('.impl')).toBeVisible();
    expect(mock.requests.filter((r) => r.includes('/pulls'))).toEqual([]);
  });
});

// req:web/approve-by-branch@971e758a
test.describe('Approve by branch', () => {
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

  test("a 409 whose new review can't be loaded says to reload before approving", async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(APPROVE, 409);
    mock.onPost = () => mock.fail(REVIEW, 502);
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('button.approve').click();
    await expect(page.locator('.review')).toContainText('could not be loaded. Reload before approving.');
  });

  test('any other failure says Not approved with the status', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(APPROVE, 422);
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await page.locator('button.approve').click();
    await expect(page.locator('.review')).toContainText('Not approved (422).');
    await expect(page.locator('button.approve')).toBeEnabled();
  });
});

// req:web/signed-approvals@9bdf6473
test.describe('signed repositories', () => {
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

// req:web/approve-button@ff59f9d1
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

// req:web/backlog-taken@7225a6a8
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

// req:web/oversized-component@550a1fc1
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

// req:web.client-live/notify-switch@59a9bf95
test.describe('Notify me', () => {
  test('no key: a Worker answering 404 for the push key shows no switch', async ({ page, context }) => {
    const mock = await mockApi(context);
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=b]')).toBeVisible();
    await expect.poll(() => mock.requests.includes('GET /api/push/key')).toBe(true);
    await expect(page.locator('.sidebar .notify')).toBeHidden();
  });

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
  });

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
});

// req:web.client-live/push-shown@28764ae8
test('the service worker imports the push handlers and shows a push', async ({ page, context, baseURL }) => {
  await mockApi(context);
  // The view, not the `load` event: the graph's large chunk can hold `load` back.
  await page.goto('/app/#/o/a/graph', { waitUntil: 'commit' });
  await expect(page.locator('.n[data-node=b]')).toBeVisible();
  const sw = await page.evaluate(async () => {
    const reg = await navigator.serviceWorker.ready;
    return reg.active?.scriptURL ?? null;
  });
  expect(sw).toMatch(/\/sw\.js$/);
  const worker = context.serviceWorkers().find((w) => w.url().endsWith('/sw.js')) ?? (await context.waitForEvent('serviceworker'));
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

// req:web/installable@e711892d
test.describe('PWA', () => {
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

  test('offline launch: shows the last data with its age, and Approve is disabled', async ({ page, context }) => {
    await mockApi(context);
    await page.goto('/app/#/o/a/prs/spec%2Fc');
    await expect(page.locator('button.approve')).toBeEnabled();
    // The service worker has installed and controls the page.
    await page.evaluate(async () => {
      await navigator.serviceWorker.ready;
      if (!navigator.serviceWorker.controller) {
        await new Promise((r) => navigator.serviceWorker.addEventListener('controllerchange', r, { once: true }));
      }
    });
    // Visited once online; then the network goes.
    await context.unroute('**/api/**');
    await context.setOffline(true);
    await page.reload();
    await expect(page.locator('.offline')).toHaveText(/^Offline · showing data from (just now|\d+ min ago)$/);
    await expect(page.locator('.review > .head h1')).toHaveText('spec/c');
    await expect(page.locator('.repo-meta')).toContainText('3 components');
    await expect(page.locator('button.approve')).toBeDisabled();
  });
});

// req:web/comment-on-review@1279617c
test.describe('comments', () => {
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
  });
});

// req:web/request-changes@8ce19ec8
test.describe('Request changes', () => {
  const requested = { id: 'q1', writer: 'alice/web', at: '2026-10-27T12:00:00Z', fingerprint: 'f1', text: 'Split it', current: true, held: true };

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
});

// req:web/propose-from-backlog@9b8191c8
test.describe('proposals', () => {
  const IMPL = 'GET /api/repos/o/a/implementation';
  const proposal = (over = {}) => ({ id: 'p1', writer: 'alice/web', at: '2026-10-27T10:00:00Z', slug: 'export-csv', title: 'Export CSV', body: 'Let me export.', nodes: ['a'], mine: true, ...over });

  test('proposed: two quick submits post once, the form closes, and Proposed lists it by alice with the note', async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.onPost = () => (mock.routes[IMPL] = implementation({ proposals: [proposal()] }));
    await page.goto('/app/#/o/a/backlog');
    await page.locator('button.propose').click();
    const form = page.locator('.fbform.proposal');
    await form.locator('input[name=title]').fill('Export CSV');
    await form.locator('textarea').fill('Let me export.');
    await form.locator('.picker input[value="a"]').check();
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
    await expect(row.locator('.pill')).toHaveText(['a']);
    expect(mock.posts).toEqual([{ key: PROPOSALS, body: '{"title":"Export CSV","body":"Let me export.","nodes":["a"]}' }]);
    expect(mock.requests.filter((r) => r === IMPL).length).toBeGreaterThanOrEqual(2);
  });

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

  test("a 422 names the Worker's field and keeps the form", async ({ page, context }) => {
    const mock = await mockApi(context);
    mock.fail(PROPOSALS, 422, { error: 'nodes' });
    await page.goto('/app/#/o/a/backlog');
    await page.locator('button.propose').click();
    await page.locator('.fbform.proposal input[name=title]').fill('Export CSV');
    await page.locator('.fbform.proposal button.send').click();
    await expect(page.locator('.fbform.proposal .err')).toHaveText('Not proposed: the Worker refused the nodes.');
  });

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
});

// req:web/latest-navigation-wins@d93cb8bd
test.describe('latest navigation wins', () => {
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
});

// req:web/links-to-site@110ad07e
test.describe('links to the site', () => {
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

  test('phone top bar: begins with a homepage link before the repo switcher', async ({ page, context }) => {
    await mockApi(context);
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/app/#/o/a/graph');
    await expect(page.locator('.n[data-node=a]')).toBeVisible();
    const first = page.locator('.mtop > *').first();
    await expect(first).toHaveAttribute('href', '/');
    await expect(first).toHaveAttribute('aria-label', /CLM homepage/);
    expect(await page.locator('.mtop > *').evaluateAll((els) => els.findIndex((e) => e.tagName === 'SELECT'))).toBe(1);
  });
});

// req:web/sign-in-early-access@50c354b1
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
  await expect(card.locator('a', { hasText: 'Install the GitHub App' })).toBeVisible();
});
