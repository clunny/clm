---
title: "The fresh-after-write browser test reads the cache before the page has written it"
kind: bug
components: [web]
---

`web/e2e/fresh.spec.ts` › `fresh after write` › `approve: the review is requested again with a UUID, and the offline copy has no fresh` failed in `clm land impl/replica`'s check on the merged tree, after passing in the same branch's `clm land --check-only` minutes before. The branch changed no web file. It failed with `kept` = `["/api/repos", "/api/repos/o/a/branches", "/api/repos/o/a/snapshot"]`: no review entry yet.

Cause: the test polls until the mock has seen the second review request, then reads `caches.open('clm-api')` at once (`fresh.spec.ts:28-30`). The page stores its copy only after the response arrives, so the read can come before the write. Expected: the test waits for the copy, e.g. with `expect.poll` over the cache's keys until the review entry shows, and then asserts that no key holds `fresh`.
