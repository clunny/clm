---
title: "Every test runs in the land check, the browser suites included, so lands run over their time target"
kind: idea
components: [ci, web, web.site, capability, verify]
---

`land-checks-stay-fast` asks that a land check run only fast, deterministic tests and that slow, flaky or exclusive ones go to `[verify]`, after the land. This repository has never moved a test there. `[land] check` and `[verify] check` both run `.github/scripts/ci-check.sh`, which has no part only `[verify]` runs: `[verify]` runs the same tests on `main` with a wider selection, as the backstop for lands that checked only an overlap. So every test is a land test, the browser suites included.

## What it costs (2026-11-13)

- The land check ran over its 120-second target on 9 of the last 20 timed lands. Over those lands the Rust tests took 918 seconds, `capabilities/web` 690 and `web` 318.
- One site test, `web/site/test/analytics.spec.ts`, holds `clm:reads docs/guide/**`. (Corrected below: six site tests and the capability suite's setup file hold it.) The check script turns the web suites on per directory, so any edit to the guide runs the `web`, `web/site` and `capabilities/web` suites in full: 157 of the 192 seconds of the land of `20261113-land-starvation`. Spec and docs lands that select no web test check in 18 to 20 seconds.
- In `capabilities/web`, one test (`review-capability-cost.spec.ts`, "a person sees which architectural constraint is held and decides to keep one") took 32 seconds of a 60-second run.
- `capabilities/web` runs with `--retries=1`, which suggests its tests aren't fully deterministic.

## What to weigh

- **Selecting web tests per file, not per suite.** A guide edit would run the one test that reads the guide. It loses no proof at the land and is likely the largest saving. `backlog/finer-test-selection` covers finer selection in general; this is the web suites' case of it.
- **The one 32-second test.** Fixing or moving a single slow test is a smaller step than moving a suite.
- **Whether the capability tests stay in the land check.** They prove a capability, and the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims` is about a passing land meaning the code does what the spec says. In `[verify]`, a broken web capability would land and be found up to 20 lands later, about two hours at this repository's rate.
- **A slow tier at all.** Moving the app's browser tests to `[verify]` means giving the check script, for the first time, a part only `[verify]` runs. Decide it after per-file selection, with the times that remain.

## Found on 2026-11-13, when a change for this was drafted and withdrawn

A change, `20261113-browser-suites-run-selected-files`, was drafted for per-file selection in the browser packages and withdrawn before it landed, after its builder check and architecture audit. What they and the measurements found:

- **Seven files read the guide, not one.** Six of the seven site tests hold `clm:reads docs/guide/**` (`analytics`, `blog`, `github`, `guide`, `seo`, `site`), and so does `capabilities/web/global-setup.ts`, which builds the guide into the site for every capability test. So an edit to the guide rightly selects nearly the whole site suite and, through a file that isn't a test, the whole `capabilities/web` suite. Running "only the selected test files" would skip only `web/`'s own suite: about 40 of the 160 seconds, on about a quarter of spec lands, and nothing on code lands.
- **Where the time is, over the last 40 timed lands.** The browser suites took about 2,830 seconds and the Rust tests about 1,490; fmt, clippy, the wasm builds and the tools were a second or two a land. On the 21 code lands: Rust tests ran on all, mean 55 seconds; `capabilities/web` on 13, mean 59; `web` on 7, mean 175, most of it one 1,141-second land that looks like a hang. On the 19 spec lands: Rust tests mean 17 seconds; `web`, `web/site` and `capabilities/web` each on 5, means 39, 60 and 64.
- **What costs builds a minute is the capability suite on code lands**, which per-file selection in the script can't narrow (`backlog/finer-test-selection`).
- **Two parts of a per-file rule could never fire.** `clm affected` lists the changed files and every file of an affected component that holds no `req:` marker (`check.affected/affected-files`), so any selection naming a test under `web/` outside the site, or under `capabilities/web/`, also names a helper such as `web/e2e/mock.ts` or `capabilities/web/harness.ts`, and the suite runs whole. Only site tests are ever selected alone, as readers of a doc.
- **A hole to avoid.** The script runs `capabilities/web` after any selected file under `web/site/`. That covers a case the graph doesn't: a guide edit and a site source change landing close together, whose overlap is the site's reader tests alone, while the capability tests depend on both. A rule that stops site test files from triggering `capabilities/web` removes that cover, and only `[verify]` would then catch a break (`capability-browser-packages`).
- **The smallest safe step, if the 40 seconds are wanted.** When the only selected files under `web/` are site test files, skip `web/`'s own suite and still run the site suite and `capabilities/web`. One sentence in `ci/web-checks` and `ci/selected-tests` and one scenario. A site-only run needs the browser installed, which `web/`'s suite does today. `ci` is over its size budget (spec about 15,300 tokens of 12,000).
