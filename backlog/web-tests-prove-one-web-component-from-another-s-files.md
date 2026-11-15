---
title: "Web tests prove one web component from another's files"
kind: deferred
components: [web, web.views, web.client-live, web.worker]
origin: 20261115-tests-in-their-own-files
---

Seven test files of the web app hold 40 markers that cite a component other than their owner, which only the clause "a file that cites the component" lets a build change (`own-files-at-the-gate`). Measured on `main` at `4f96740b`:

- `web/e2e/app.spec.ts` (`web`'s) cites `web.views` 15 times and `web.client-live` twice, and `web/e2e/layout.spec.ts` cites `web.views` once: browser tests of the whole app that prove a view's or the live socket's requirement.
- `web/src/views/api.test.ts` (`web.views`'s) cites `web` 7 times and `web.client-live` once, and none of its owner's; `web/src/views/pr.test.ts` cites `web` once.
- `crates/clm-web/tests/deploy.rs` (4), `npmrc.rs` (1) and `sign_in.rs` (8) are `web.worker`'s and cite `web`.

This needs a design, not only a move, so it is not due until one is proposed. For each file, one of three holds: the test moves to the files of the component it cites; the requirement is the file owner's and a spec change moves it; or the file is given to the cited component with a `tests` glob (`components-own-their-tests`), which fits a browser test that imports none of the app's code. A test of the whole product stays with `web`, which holds the entry point.
