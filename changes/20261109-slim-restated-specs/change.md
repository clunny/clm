---
status: applied
---

## Intent

Slim the first batch of the components `clm health` flags as outliers: `gate.fingerprint`, `orchestrator.checkpoint`, `plugin.hooks.guard` and `check.coverage`, plus the two narrating requirements the backlog item names (`check.coverage/owner`, `ci/nextest-runner`). Each Design is cut down to its reasons, where it restated its requirements sentence by sentence. Requirements that duplicate another are folded in, requirement text that narrates a past build or a removed field is cut, scenarios another one fully covers are dropped or merged, and the requirements that bundled several rules are split. This is the first batch of `backlog/specs-restate-their-requirements-and-narrate-history`, which stays open for the rest.

## Problem

`clm health` on 2026-11-09 flagged these components: `gate.fingerprint` (spec 2118 tokens per requirement against a median of 608), `orchestrator.checkpoint` (1327), `plugin.hooks.guard` (1499) and `check.coverage` (code 6055 per requirement against 2159). A `spec-slimmer` pass on each found the same causes: Designs that restate their requirements almost word for word, one requirement carrying most of a component's rules, requirements that only repeat another (`orchestrator.checkpoint/outcome-name`, `checkpoint-verifies-review`, `plugin.hooks.guard/defect-ask-says-pull-request`), and text that narrates history (`needs_pending`, "as it does now", how a past build was to proceed). `orchestrator.checkpoint`'s Contract still said "Its report ends with the name it was run by (`outcome-name`)", which contradicts that requirement's "SHALL take no name". `check.coverage/status-report`'s scenario "node renamed later, built" names a scenario, "component renamed later, not yet built", that doesn't exist.

## Approaches

- **A. Cut and merge only.** Remove the duplicates, trim the narration, drop covered scenarios and trim the Designs, keeping every requirement whole. Removes the most words for the least rebuild, but leaves the bundled requirements as the size outliers.
- **B. A, plus split the bundled requirements (chosen).** As A, and also split `push-guard`, `release-guard`, `fingerprint` and `checkpoint` into one requirement per rule. A split moves text rather than removing it and re-cites those rules' tests, but each rule then changes, and churns, on its own, and the outlier flags clear. The human chose B.

Each split keeps the existing ID on the core rule and adds the carved-out parts under new IDs, so every reference from other components (`land`, `gate`, `web.worker.review`, `plugin`, `host`) and their `#### Needs` stay valid:

- `plugin.hooks.guard/push-guard` keeps the local `git push` cases, the queries and their budget; `push-guard-scoped-to-clm` takes the "not under CLM" exemption (the Design already used that name), and `host-routes-guard` the `gh` routes to `main`.
- `plugin.hooks.guard/release-guard` keeps dispatch, `gh release`, `gh api` and what counts as a release run; `release-run-lookup` takes the REST lookup, the repository and host it resolves, and its budget.
- `gate.fingerprint/fingerprint` keeps the hash, the "adds a change" test, `adds_change`, `empty` and the no-process rule; `fingerprint-ignores-config` and `fingerprint-stable` take the rest.
- `orchestrator.checkpoint/checkpoint` keeps the landed test, the removals, `left` and how the report ends; `checkpoint-reports-unsafe-work` takes the `dirty`, `unpushed`, `elsewhere`, `open` and `complete` lines and the violations.

## Outcome

- `orchestrator.checkpoint/outcome-name` is removed; the root's `renamed-commands` and `old-command-names-retired` need `orchestrator.checkpoint/checkpoint` in its place, which states the ending.
- `orchestrator.checkpoint/checkpoint-verifies-review` is folded into `checkpoint-review-mark`, and `plugin.hooks.guard/defect-ask-says-pull-request` into `defect-approval`.
- `plugin.hooks.guard/git-through-layer` loses its migration clause about `clm-hooks` and that scenario.
- `check.coverage/pending-report` names `owner` for an orphaned obsolete marker's component instead of restating it, and loses `needs_pending`; `owner` loses "as it does now"; `ci/nextest-runner` loses the instruction to the build that moved the script to `nextest`, which has happened.
- `ARCHITECTURE.md`: `guard-blocks-push-to-main` is enforced by `push-guard-scoped-to-clm` and `host-routes-guard` too, `land-locally` by `host-routes-guard`, and `approve-pins-the-review` by `fingerprint-stable`.

The builds afterwards delete the tests of the removed requirements' own scenarios that no rule keeps (`crates/clm-checkpoint/tests/outcome_name.rs`, the guard's "nothing of the guard left behind" check) and re-cite the rest at the new versions and IDs.

## Constraints

- No behaviour changes: every scenario dropped is covered by one kept, and every rule removed is stated by another.
- Left alone on purpose: `plugin.hooks.guard/hook-budgets`, which a `workaround` item names; `check.coverage/change-progress`, the library contract `view`, `clm rebase` and `clm wrap-up` rely on; the guard's scenarios that prove its explicit "SHALL NOT"s; `checkpoint-review-mark`'s "no lands yet", a rule of its own.
