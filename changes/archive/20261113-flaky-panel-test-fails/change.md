---
status: done
resolves: []
---

## Intent

Correct the component of `backlog/flaky-panel-test-fails-a-land-removing-its-temp-directory` from `plugin.panel` to `capability`, and report a CLM defect. The change has no delta.

The failing test, `capabilities/panel/watch-machine-load.test.ts`, and the fake engine it runs the pane in are `capability`'s (`paths: capabilities/**`), not `plugin.panel`'s. The cause is in that harness: the fake engine runs the pane's `clm` with nothing waiting for it, so a test can end and remove its temporary directory while a `clm` still writes there, and the removal fails with ENOTEMPTY. The fix is test plumbing: the engine stops its timers and waits for its processes before a test removes its directory, which changes nothing a capability test observes and leaves `capability`'s Design true as written. So it is no requirement of any component (`human-layer-proof`) and is fixed on the code branch `fix/panel-suite-awaits-pane-processes`, whose commit deletes the item with `Resolves: backlog/flaky-panel-test-fails-a-land-removing-its-temp-directory` (`small-change-no-claim`).

The gate counts that deletion only for an item on a component the branch's code touches, so the item must name `capability` on `main` before the fix can land. That gap in the test-plumbing route is reported as `backlog/a-test-plumbing-fix-can-t-delete-a-bug-item-that-names-another`.
