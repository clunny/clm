---
title: "Web's tests ported from the Worker's crate stay there, since web's build can't delete them"
kind: question
components: [web, web.worker]
blocks: [web/no-install-scripts, web/invited-users, web/site-served-directly]
choices:
  - "web.worker's build deletes them"
  - "Exempt web's build to delete them"
---

`web`'s Design (**Tests**) and `changes/20261117-tests-sit-with-the-component-they-cite/moved-tests.txt` tell `web`'s build to port the Worker crate's tests of `web`'s files to `web/src/` and delete them there: `crates/clm-web/tests/npmrc.rs` whole, and `deploys_the_clm_worker_on_the_apps_routes` and `sends_only_the_api_and_its_paths_to_the_worker` in `crates/clm-web/tests/deploy.rs`. Those files are `web.worker`'s, and `clm land` refuses a `web` build that changes them (`land/build-holds-own-files`), so the deletion can't be made by the build the spec gives it to.

`web`'s build of `no-install-scripts@d8da6553` ported `npmrc.rs`'s two tests to `web/src/npmrc.test.ts`, which proves both scenarios at the current version. The requirement still reads `behind`, because `npmrc.rs` cites it at `0d751f57` and `clm pending` counts the stale marker. It is current once `npmrc.rs` is deleted. `deploy.rs`'s two tests cite `web/site-served-directly` and `web/invited-users`, which are blocked by `web.worker/invited-sign-in` today and will meet the same wall when `web` builds them.

A spec change is needed: either a requirement of `web.worker` (or its Design) whose build deletes `npmrc.rs` and the two `deploy.rs` tests, once `web/src/` holds their assertions, or an exemption letting `web`'s build delete them.
