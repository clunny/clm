---
title: "A web e2e test fails a land under no load and passes on the retry"
kind: bug
components: [web]
---

Step: `clm land` of a spec branch whose check script ran the web end-to-end suite (`ci-check: web`).

What happened: one test, `the service worker imports the push handlers and shows a push` in `web/e2e/app.spec.ts`, timed out after 20 s waiting for the graph node to be visible right after navigating with `waitUntil: 'commit'`. Many other tests in the same run took about 30 s. The machine was idle (load 0.1x of 24 cores, no builds running). Running `clm land` again on the same tree passed, and the branch landed.

Expected: a land check is deterministic (`land-checks-stay-fast`), so the same tree gives the same verdict. A flaky test there fails lands that change nothing it covers and costs a full re-run.

Rule involved: `land-checks-stay-fast`, `tests-run-concurrently`.
