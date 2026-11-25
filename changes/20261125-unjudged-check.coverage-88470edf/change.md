---
status: proposed
resolves: [unjudged-check.coverage-88470edf, unjudged-web.views-88470edf, unjudged-web.worker-88470edf]
---

## Intent

Deletes three backlog items that `dcee2c44` already fixed; no requirement changes.

`unjudged-check.coverage-88470edf`, `unjudged-web.views-88470edf` and `unjudged-web.worker-88470edf` (filed by `5e311d57`) each say the pass after `88470edf` never reached their component's blocks because `.github/scripts/mutate.sh` left StrykerJS's `vitest.related` at true: five `web` blocks whose tests read files ran no test, StrykerJS failed, and the pass stopped. `dcee2c44` ("ci: StrykerJS runs a block's vitest tests whether or not they import its code", `Resolves: backlog/unjudged-ci-88470edf`, the item each of these names as needing the same mend) landed after them and sets `"related": false` in the configuration `mutate.sh` writes. `20261124-mutate-script-s-strykerjs` then wrote the rule into `ci/mutate-script` and made its StrykerJS fixture fail when the tool runs only the tests that import a mutated file. Nothing in `check.coverage`, `web.views` or `web.worker` kept the pass from its verdict, so with that mend the next pass reaches their blocks; a pass that again leaves them unjudged files a fresh item.
