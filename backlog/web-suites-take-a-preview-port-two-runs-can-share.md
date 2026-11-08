---
title: "Web suites take a preview port two runs can share"
kind: bug
components: [web, web.site]
---

Two checks running at the same time on one host can collide in the browser suites (`tests-run-concurrently`). Found while taking the `20261107-projects-upgrading-past-the` upgrade note.

- `web/playwright.config.ts` serves on `CLM_WEB_PORT` when the environment already sets it (`web/test/preview-port.ts`), and sets `reuseExistingServer: !process.env.CI`. Outside CI, two runs that inherit the same `CLM_WEB_PORT` don't fail: the second finds the first's `vite preview` answering and tests the other checkout's build. The same reuse happens if another run takes the free port between the probe and the bind. Fix: `reuseExistingServer: false`, as `web/site/playwright.config.ts` has.
- `web/site/playwright.config.ts` serves on `CLM_SITE_PORT` when the environment sets it, and runs `vitepress preview` with no strict-port option. On a port another run holds, it can move to a different port while Playwright waits on the original URL. Fix: make the preview fail on a taken port, or check that the server answering is this run's.

Neither suite keys its port on `CLM_SLOT`. Each picks a free port, which is fine as long as the environment doesn't already carry the variable.
