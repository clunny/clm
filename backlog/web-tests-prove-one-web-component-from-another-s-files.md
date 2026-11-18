---
title: "Web tests prove one web component from another's files"
kind: deferred
components: [web, web.views, web.worker]
origin: 20261117-tests-sit-with-the-component-they-cite
blocks: [check/tests-cite-their-owner]
---

Seven test files of the web app hold 40 markers that cite a component other than their owner: `web/e2e/app.spec.ts` and `layout.spec.ts` (`web`'s, citing `web.views` and `web.client-live`), `web/src/views/api.test.ts` and `pr.test.ts` (`web.views`', citing `web` and `web.client-live`), and `crates/clm-web/tests/deploy.rs`, `npmrc.rs` and `sign_in.rs` (`web.worker`'s, citing `web`).

`20261117-tests-sit-with-the-component-they-cite` designed where each goes: `changes/20261117-tests-sit-with-the-component-they-cite/moved-tests.txt` lists every test with the file it ends in and the requirement it cites there, and the Designs of `web`, `web.views` and `web.worker` say why. The builds of those three components carry it out.

This item is kept only to order the work. The web components are optional, so `check/tests-cite-their-owner`, a required component's requirement, can't name theirs under `#### Needs`; this item blocks it instead, and `gate/own-files` needs it. Once the root names its `wiring`, that rule reports every marker that cites a component other than its file's owner, so it can't be built while one of these is left.

**Delete this item** in the build of `web`, `web.views` or `web.worker` after which no file under `web/` or `crates/clm-web/` holds a marker citing a component other than the file's owner: the last of the three to land. The change names it in `resolves`, so a build may delete it.
