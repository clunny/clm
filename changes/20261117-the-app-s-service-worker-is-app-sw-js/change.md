---
status: proposed
from: backlog/push-in-the-browser-names-the-app-s-service-worker-sw-js
---

## Intent

`web/push-in-the-browser`, added by `20261117-tests-sit-with-the-component-they-cite`, names the service worker the app registers as `/sw.js`. The app registers `/app/sw.js` with scope `/app/` (`web/installable`); `/sw.js` is the retiring root worker (`web/retire-root-service-worker`). The human answered the question that reported it on 2026-11-17: reword it. The requirement and its scenario name `/app/sw.js`, and `web`'s build has the browser test match that path whole.

## Approaches

Reword the text to what the app does (taken), or keep `/sw.js` as written: the test matches either path, so the text would stay wrong about which worker shows a push.

## Also on this branch

Two more questions the builds of that change filed, answered by the human the same day, and both from one cause: `orchestrator.run/pending-prompt` was built before `gate/own-files`, so for a while the prompt says `clm land` refuses a build's edit to another component's file that cites it, while the gate's code still lets it through.

- `web-s-tests-ported-from-the-worker-crate-stay-there-since-web-s-build-can-t-delete-them` is deleted: no spec change is needed. The land accepts the deletion today, since `crates/clm-web/tests/npmrc.rs` and `deploy.rs` cite `web`, so the deletion is `web`'s build's, as `web`'s Design says.
- `run-prompt-says-land-refuses-an-edit-it-accepts` is deleted as accepted: the prompt, the check and the land agree once `gate/own-files` and `check/tests-cite-their-owner` are built, which is the plan's next step. Its side point is kept as `backlog/clm-run-keeps-its-own-copy-of-the-test-for-which-files-a-build-may-change`.
