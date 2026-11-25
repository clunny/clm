---
status: done
resolves: []
---

## Intent

Corrects `backlog/unjudged-web.worker-88470edf`'s `components` from `web.worker` to `ci` and reports the CLM defect behind it; no requirement changes.

`web.worker`'s six requirements were left unjudged because `.github/scripts/mutate.sh` stopped at five `web` blocks whose StrykerJS run executed no test, and `ci/mutate-script` starts no further block once one fails, so the pass recorded no verdict at all (`0 killed, 0 unproven, 0 not judged, 29 left`). Nothing in `web.worker`'s code or tests keeps the pass from its verdict; the fix lies in `ci`'s `mutate.sh`, so the item goes to `ci` for the next triage to group with `unjudged-ci-88470edf`. The new item `one-block-s-strykerjs-failure-leaves-every-other-component-s` reports how one block's failure starves every other requirement and is filed under each left requirement's own component.
