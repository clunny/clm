---
status: applied
resolves: []
---

## Intent

Reports a CLM defect met while drafting: `clm draft`'s prompt has no route for a bug that only an architecture entry states (`backlog/draft-prompt-has-no-route-for-a-bug-only-an-architecture-entry`).

The item this draft was given, `web-suites-take-a-preview-port-two-runs-can-share`, was fixed as code on `impl/web` (`8029e80`): `web/playwright.config.ts` no longer reuses a preview server already on its port (`tests-run-concurrently`). Its `web.site` half needed no fix: `vitepress preview` exits with `EADDRINUSE` on a taken port rather than moving, and that config already sets `reuseExistingServer: false`.
