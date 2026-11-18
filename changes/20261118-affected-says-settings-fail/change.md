---
status: applied
from: backlog/affected-settings-load-failure-has-no-requirement
---

## Intent

Two small pieces of tidying the human asked for on 2026-11-18, after phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md` and before phase 5.

`check.affected/affected-base` states that `clm affected` exits 1, naming the file and key, when the `[affected]` settings can't be loaded. The Contract and Design said so and the code does it, but no requirement did, so the spec-auditor had the test's marker taken off on `impl/check.affected` (`code-fix-needs-a-written-rule`). The question item that asked is answered by this change and deleted.

`orchestrator.triage`, `orchestrator.run`, `land` and `view` no longer name `check` in `depends_on`. None of their crates depends on `clm-check` since the twelve crates moved their imports to `clm-coverage`: `clm-land` runs `clm check` as a process, and the others call only the coverage library or nothing of either.

## Approaches

For the clause: add it to `affected-base`, which already states the command's other exits of 1 (taken, as the item proposed); add it to `affected-all`, which names the setting but states only what a glob selects; or accept the behaviour as stated by the Contract alone, the freeze's default, which leaves a test that cites nothing and code a prune could remove unseen.

## Outcome

- `check.affected`'s build cites `settings_that_cant_load_exit_1` in `crates/clm-affected/tests/command.rs` at `affected-base`, takes out the comment that names the deleted item, and re-cites the requirement's other tests. No code changes.
- The four `depends_on` edits build nothing. A build of one of those components is no longer given `check`'s Contract as a dependency's; `orchestrator.run`, `land` and `view` still depend on `check.coverage`, whose builds read `check`'s Contract as an ancestor's.

## Constraints

No file, key or format an adopter holds changes.
