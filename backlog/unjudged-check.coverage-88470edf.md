---
title: The mutation pass can't judge requirements of check.coverage
kind: bug
components: [ci]
---

Each of the mutation passes after the lands named left these requirements of `check.coverage` unjudged, having stopped, reached its limit or failed before its verdict:

- `check.coverage/pending-report`: `bf0d9028`, `88470edf`
- `check.coverage/status-report`: `bf0d9028`, `88470edf`

Each pass after the first tried them first, so one that left them was stopped, reached `mutate_minutes`, or met a mutation command that fails on their code: `clm verify`'s stderr for those passes says which. Mend what keeps the pass from its verdict, then delete this item.

What kept it (draft `20261124-unjudged-check.coverage-88470edf`, from the stderr of the pass after `88470edf`): `.github/scripts/mutate.sh failed with status 1` with `0 killed, 0 unproven, 0 not judged, 29 left`, after `mutate.sh: StrykerJS failed for requirement web/<slug>` for the first five blocks it started (`web/no-install-scripts`, `web/graph-fills-height`, `web/invited-users`, `web/sign-in-early-access`, `web/site-served-directly`), each StrykerJS ending `No tests were executed` after `Vitest failed to find test files related to mutated files`. The configuration `mutate.sh` writes gives `"vitest": { "configFile": ... }` with no `"related": false`, so StrykerJS's vitest runner keeps only the block's tests that import a mutated file, while `ci/mutate-script` says StrykerJS runs "with the block's vitest `test` lines alone"; a block whose tests read files or reach the code another way then runs no test, and that tool failure ends the pass before the `check.coverage` blocks start. The fix lies in `mutate.sh` (component `ci`), the same mend `backlog/unjudged-ci-88470edf` needs: a code branch can resolve both.
