---
title: "Every test runs in the land check, the browser suites included, so lands run over their time target"
kind: idea
components: [ci, web, web.site, capability, verify]
---

`land-checks-stay-fast` asks that a land check run only fast, deterministic tests and that slow, flaky or exclusive ones go to `[verify]`, after the land. This repository has never moved a test there. `[land] check` and `[verify] check` both run `.github/scripts/ci-check.sh`, which has no part only `[verify]` runs: `[verify]` runs the same tests on `main` with a wider selection, as the backstop for lands that checked only an overlap. So every test is a land test, the browser suites included.

## What it costs (2026-11-13)

- The land check ran over its 120-second target on 9 of the last 20 timed lands. Over those lands the Rust tests took 918 seconds, `capabilities/web` 690 and `web` 318.
- One site test, `web/site/test/analytics.spec.ts`, holds `clm:reads docs/guide/**`. The check script turns the web suites on per directory, so any edit to the guide runs the `web`, `web/site` and `capabilities/web` suites in full: 157 of the 192 seconds of the land of `20261113-land-starvation`. Spec and docs lands that select no web test check in 18 to 20 seconds.
- In `capabilities/web`, one test (`review-capability-cost.spec.ts`, "a person sees which architectural constraint is held and decides to keep one") took 32 seconds of a 60-second run.
- `capabilities/web` runs with `--retries=1`, which suggests its tests aren't fully deterministic.

## What to weigh

- **Selecting web tests per file, not per suite.** A guide edit would run the one test that reads the guide. It loses no proof at the land and is likely the largest saving. `backlog/finer-test-selection` covers finer selection in general; this is the web suites' case of it.
- **The one 32-second test.** Fixing or moving a single slow test is a smaller step than moving a suite.
- **Whether the capability tests stay in the land check.** They prove a capability, and the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims` is about a passing land meaning the code does what the spec says. In `[verify]`, a broken web capability would land and be found up to 20 lands later, about two hours at this repository's rate.
- **A slow tier at all.** Moving the app's browser tests to `[verify]` means giving the check script, for the first time, a part only `[verify]` runs. Decide it after per-file selection, with the times that remain.
