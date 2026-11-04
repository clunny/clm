---
title: "The land's check retries a failed web capability test once"
date: 2026-10-31
status: accepted
---

Settles `backlog/web-capability-playwright-spec-fails-intermittently-in-land`. `capabilities/web/`'s Playwright tests failed three times in one session's land checks, in both spec files, and each passed alone and on the rerun with nothing changed. The one failure whose output survives waited 30 s for the review page's approve button, which an idle run draws in about a second. Every land on this machine runs while other builds compile and run their suites (`build-places`), and each of these tests starts its own Worker in Miniflare, answers it from a fake GitHub that runs `git` per request, and lands branches with `clm land`, so it is the heaviest browser test the land runs.

So `ci-check.sh` runs the package as `npm test -- --retries=1` (`ci/capability-web-retries`): Playwright reruns a failed test once, from the start in a fresh browser context, and the check fails only when it fails twice. A retry costs one test's run, against a whole land's check for each flake. Playwright reports a test that passed only on its retry as `flaky`, with the first attempt's error, so a flake still shows in the land's output.

The retry is the check script's, not the package's config: a person or a build that runs `npm test` in the package sees every failure, which is where a real race would be chased.

Alternatives considered:
- **Longer timeouts**, as `web/`'s suite took. The assertion already waits 30 s and the test 120 s, for steps that take a second idle; a wait that long that still runs out isn't fixed by a longer one, and each raise slows every real failure.
- **Finding the cause first.** The failing step's trace was gone by the time the item was read, and the failure hasn't been reproduced. A real race in the app would fail both attempts often enough to show; until one does, the cost falls on every land.
- **One capability suite at a time per machine** (a lock like `land/land-target-lock`). The load comes mostly from other builds' compiles, not from other browser suites, so it would serialise lands without removing the cause.
- **Retries for every browser suite.** `web/`'s failures under load were fixed at their causes and `capabilities/panel/` has shown none (`yagni`).
