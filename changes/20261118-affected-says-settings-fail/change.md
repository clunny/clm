---
status: applied
from: backlog/affected-settings-load-failure-has-no-requirement
---

## Intent

Two small pieces of tidying the human asked for on 2026-11-18, after phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md` and before phase 5.

`check.affected/affected-base` states that `clm affected` exits 1 when the `[affected]` settings can't be loaded, writing its caller's line to standard error and nothing to standard output. The root loads the settings and words the line, which `clm/routes-only` already states and `crates/clm/tests/routes.rs` proves through the binary, so the clause says only what `clm-affected` does. The Contract and Design said so and the code does it, but no requirement did, so the spec-auditor had the test's marker taken off on `impl/check.affected` (`code-fix-needs-a-written-rule`). The question item that asked is answered by this change and deleted.

`orchestrator.triage`, `orchestrator.run` and `view` no longer name `check` in `depends_on`: none of their crates depends on `clm-check` since the twelve crates moved their imports to `clm-coverage`. `view`'s Design and `view/status-report` name `check.coverage` for the rows of `clm status`, which they still gave to `check`. `land` keeps `check`: its crate has no `clm-check` dependency either, but it runs `clm check --no-detectors` as a process and its spec relies on that command line, so its builds should be given `check`'s Contract (`ancestor-scoped-context`); the architecture audit found this.

## Approaches

For the clause: add it to `affected-base`, which already states the command's other exits of 1 (taken, as the item proposed); add it to `affected-all`, which names the setting but states only what a glob selects; or accept the behaviour as stated by the Contract alone, the freeze's default, which leaves a test that cites nothing and code a prune could remove unseen.

## Outcome

- `check.affected`'s build cites `settings_that_cant_load_exit_1` in `crates/clm-affected/tests/command.rs` at `affected-base`, takes out the comment that names the deleted item, and re-cites the requirement's other tests. No code changes.
- The `depends_on` edits build nothing; `view`'s build re-cites `status-report` and rewords the doc comment in `crates/clm-view/src/spec.rs` that names `clm_check::size`.
- `clm affected --readers` never loads the settings and exits 0 whatever they hold, which the builder check found by running it; the clause says so.

## Open questions

- The line the root prints for the failure carries the rule `check.affected/affected-all`, though `affected-base` now states the exit. It is left: `clm/routes-only`'s scenario quotes that label, the failing key is `affected.all`, and changing it costs a build of the root and of `check.affected` for a label.

## Constraints

No file, key or format an adopter holds changes.
