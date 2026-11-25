---
status: applied
resolves: []
---

## Intent

This change corrects `backlog/unjudged-check.coverage-88470edf`'s `components` from `check.coverage` to `ci` and records in it what kept the mutation pass from its verdict; it designs nothing.

The pass after `88470edf` left `check.coverage/pending-report` and `check.coverage/status-report` unjudged because it never started their blocks: `.github/scripts/mutate.sh` exited 1 when StrykerJS ran no test for the first five `web` blocks it started (`No tests were executed`, after `Vitest failed to find test files related to mutated files`), and a tool failure ends the pass. The configuration `mutate.sh` writes leaves StrykerJS's vitest runner relating tests to the mutated files, where `ci/mutate-script` as written has it run the block's vitest `test` lines alone. So the fix is code-only and lies in `mutate.sh`, a file of `ci`, which the item didn't name: `clm claim --fix` takes only a component the item names, so the next `clm triage` groups it under `ci`, beside `backlog/unjudged-ci-88470edf`, which the same mend resolves.

It also files the CLM defect met while drafting: `backlog/an-unjudged-item-doesn-t-say-which-failure-ended-the-pass-that` (component `verify`), since the item named neither the failure nor a log that still held it, and one tool failure filed items on seven components.
