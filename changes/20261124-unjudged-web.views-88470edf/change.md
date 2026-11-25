---
status: applied
resolves: []
---

## Intent

Corrects `backlog/unjudged-web.views-88470edf.md`'s `components` from `web.views` to `ci`; no requirement changes.

`web.views/answer-buttons`, `architecture-decide-buttons`, `decide-buttons`, `home-waiting-on-you` and `propose-from-backlog` were left unjudged because `.github/scripts/mutate.sh` stopped at five `web` blocks whose StrykerJS run executed no test ("Vitest failed to find test files related to mutated files"), and `ci/mutate-script` starts no further block once one fails, so the pass after `88470edf` recorded no verdict at all (`0 killed, 0 unproven, 0 not judged, 29 left`). Nothing in `web.views`' code or tests keeps the pass from its verdict, and all five requirements are current; the fix lies in `ci`'s `mutate.sh`, so the item goes to `ci` for the next triage to group with `unjudged-ci-88470edf`. The CLM defect behind it, one block's failure starving every other requirement and being filed under each left requirement's own component, is reported by `20261124-unjudged-web.worker-88470edf` as `one-block-s-strykerjs-failure-leaves-every-other-component-s`, so this draft reports no duplicate.
