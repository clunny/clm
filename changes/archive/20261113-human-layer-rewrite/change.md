---
status: done
---

## Intent

Phase 1 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`: rewrite the human layer by hand, with the human in the conversation. `ARCHITECTURE.md` states each constraint in one or two sentences, labelled checked, measured or principle, with its mechanics in the decision it names; `CAPABILITIES.md` folds the runner into `build-from-spec`, moves `pin-working-method` to the constraints, adds the capability for measuring by evals and rewords scenarios the product's boundary can't observe. The contradictions the plan lists under Problem are removed, and `docs/the-clm-project.md` and the opening of `AGENTS.md` are brought up to date with the settled layer.

The plan item stays in the backlog: this change names it here only, never in `from` or `resolves`.

## Problem

The audit of 2026-11-13 found mechanics in the human layer and rules called enforced that no program decides. `ARCHITECTURE.md` had grown to 8,636 words in 56 entries and is prepended to every context bundle; entries such as `path-ownership` (828 words) named files and exceptions the human had to approve; several entries listed, under `#### Enforced by`, requirements whose tests only scan a prompt for a sentence. The documents also contradicted themselves: the release workflow pushes `main` against `land-locally`; `[verify] mode = "pipeline"` runs tests on the host against `platform`; the guide's FAQ said Claude Code only; `ai-writes-code` said code changes only through the build flow, though a small fix takes none; the workaround limit was per "thing" in the entry and per capability in the code and the guide; and the decisions `node-reconciliation` and `node-size-budgets` said the opposite of constraints citing them.

## Approaches

- **Rewrite both files by hand in one branch, settled with the human in two tables first** (chosen). Only the human can settle the human layer, and one branch costs one approval.
- **Let triage draft it entry by entry.** Rejected: 56 reviews, and no view of the merges across entries.
- **Shorten the text and leave the entries as they are.** Rejected: 15 entries were mechanisms of another entry or restated a capability, and keeping them keeps the file long.

## Settled with the human (2026-11-13)


Pass 1, the constraints: 56 become 43. No enforcing requirement is deleted; each is pointed at the surviving entry or a capability.

- Merged: `regenerate-dont-merge` into `hash-based-staleness`; `trunk-named-main`, `no-local-land-with-remote` and `guard-blocks-push-to-main` into `land-locally`; `land-policy-from-base` and `approve-pins-the-review` into `approval-policy`; `web-approve-off-when-signed` into `signed-ledger-records`; `needs-alone-order` into `component-reconciliation`; `settings-before-use` into `checks-relax-before-use`.
- Moved down to their decisions: `per-writer-ledgers` (rules serve `git-is-truth`), `lease-claims` and `autosync-policy` (rules serve `work-concurrently`), `health-advises` (`know-health`), `say-guard-is-off` (`guard-sessions`), `views-name-their-commit` (`see-project-state`).
- Added: `pin-working-method` (from `CAPABILITIES.md`, not carried, checked) and `removal-needs-a-yes` (an entry of its own, carried as `method`, principle until the gate can tell a removal from a rewording).
- Labels: no entry is measured until phase 6. `path-ownership` is principle until phase 3's land check; `tests-run-concurrently`, `tests-run-in-their-own-tree`, `land-checks-stay-fast` and `expand-contract` are principle with their checked parts named. `#### Enforced by` lists programs only: the prompt-scan rules leave it.
- The label is a word in each entry's statement; `#### Enforced by` and `#### Untested` stay as the parser reads them, and a block for measured waits for phase 6.
- Mechanics an adopter is held to go to a guide page, since `clm guide constraints` prints only an entry's statement and adopters can't read this repository's decisions (`method-in-the-plugin`); CLM-internal mechanics go to decisions.

Pass 2, the capabilities: 33 become 32.

- `unattended-runner` folds into `build-from-spec`: builds run unattended through `/clm:build` in a session, and "with no session open" goes, as settled in the plan. The runner's requirements serve `build-from-spec` until the plan's phase 2 deletes them.
- `pin-working-method` moves to the constraints (`pin-working-method`).
- `measure-by-evals` is added, and the plan item blocks it until the eval method is built, so no build tries to prove it before there is anything to prove.
- Reworded to what the product's boundary can observe: `adopt-clm` (a component per agreed part), `propose-intent` (the builder check leaves the statement; it still runs, and becomes a measured claim), `right-model-per-task` (CLM names the model; "a cheaper one than the main session's" goes from the scenario), `guard-sessions` (the guard prevents accidents and an audit finds what got through, `approval-gap-accepted`), `report-defects` and `work-from-any-agent-harness`.
- `catch-drift` and `no-dead-weight` restate the carried constraints `no-silent-drift` and `no-dead-code`, which are what reaches an adopter's project. They stay in this phase, as proposed cuts.

## Outcome

- `ARCHITECTURE.md`: 43 entries, each one or two sentences starting with its label, in about 5,700 words (66% of 8,636). `#### Enforced by` lists programs only, and only a checked entry holds one: a principle names the programs that check a part of it in its `#### Untested` prose, so it never reads as held.
- `CAPABILITIES.md`: 32 entries.
- 66 requirements and 10 components name a different entry under `serves`; no requirement's text or version changes, so no test falls behind for it. Seven capability entries are reworded, so their capability tests are `behind` and the `capability` build brings them up.
- The contradictions listed under Problem are gone from `ARCHITECTURE.md`, the guide and the two decisions. `land-locally` now names the release workflow's merge of its version bump as the one push to `main` that isn't a land, which is what the workflow and `clm ledger verify` already do.
- `docs/the-clm-project.md` and the opening of `AGENTS.md` state the three kinds of rule, evals and the rule on removals.
- The workaround flags `workaround-capability-report-defects-1` and `workaround-capability-right-model-per-task-1` are deleted: the reworded scenarios are the ones their tests can observe.

## What leaves the entries, and where it lives

No requirement, test or decision is deleted. These scenarios and clauses leave `ARCHITECTURE.md`; each is still stated by the rule or decision named.

- `humans-review-intent`: the trigger list and the scenario for a rename (`yes-no-review`, `rename/replay`); its exemption scenario moves to `exemptions-end`.
- `path-ownership`: the four exceptions and five scenarios (`path-ownership-by-prompt`, the guide's "What a build may change", and the `orchestrator.run` prompt requirements).
- `exemptions-end`: two scenarios (`temporary-exemptions`). `expand-contract`: the added-variant scenario and the variant and field mechanics (`expand-contract`, the guide).
- `carried-constraints`: the scenario for a constraint CLM adds (`context/bundle-carries-constraints`). `platform`: the no-host scenario, which `host-adapters` holds. `plain-terms`: the scenario for a retired term (`clm/retired-terms`).
- `guard-blocks-push-to-main`: the scenario for a repository not under CLM (`plugin.hooks.guard/push-guard-scoped-to-clm`).
- `tests-run-concurrently`, `tests-run-in-their-own-tree`, `land-checks-stay-fast`, `agent-harness-adapters`, `invite-only-web`: detail in `build-slots`, `build-setup-step`, `graph-test-selection`, `agent-harness-adapters` and `invite-only-web`.
- The six entries moved down: each decision of the same name holds the entry in full.

Two capability tests go with their entries, `crates/clm/tests/capability/unattended_runner.rs` and `crates/clm/tests/capability/pin_working_method.rs`. The same behaviour stays proven by the tests citing `orchestrator.runner`'s requirements and `orchestrator.run/superpowers-path` and `orchestrator.run/pin-file`. A spec branch can't remove an entry a test cites (`backlog/a-spec-branch-can-t-remove-a-capability-entry-a-test-cites`), so a code branch deletes the two files first.

## Decisions

- `human-layer-entries-stay-short`: each constraint is one or two sentences with its label; what merged, moved and was corrected.
- `three-kinds-of-rule`: checked, measured or principle; only a checked rule is enforced.
- `removal-needs-a-yes`: removing behaviour always needs the human's yes.
- `approval-gap-accepted`: the guard prevents accidents; signed approvals stay off here.
- `pin-working-method`: the pin is a constraint, not a capability.
- `regenerate-dont-merge`, `agent-harness-adapters`: hold the text of entries merged or cut.
- Amended: `node-reconciliation` and `node-size-budgets`, which said the opposite of the constraints citing them, and the decisions that take an entry's full text.

## Open questions

- **The plan's size target.** "Under a third" of 8,636 words is 2,880. The statements alone are about 1,800 words; the scenario and proof blocks the format requires of every entry are 3,600 more. Reaching a third needs about 25 entries, or a context bundle that carries only each entry's statement, as `clm guide constraints` already does.
- **The release workflow's push to `main`.** The contradiction is removed by naming the exception. Having the version bump land through `clm land` would remove the exception itself, and is a change to `ci` and `ledger`.
- **`catch-drift` and `no-dead-weight`** as proposed cuts.
- **`proven-by-tests` names eval proof before it exists.** The plan asks for the rewording now; the entry's `#### Untested` says CLM knows no eval proof yet. The architecture audit reads this against `spec-is-current`. The alternative is to leave the clause to the plan's phase 6.
- **Adopters' entries carry no label.** `deterministic-enforcement` is carried, and asks every rule to say its kind, but the `ARCHITECTURE.md` that `clm init` writes and the entries projects already hold have none, and nothing checks it (`backlog/adopters-architecture-entries-carry-no-label`).
- **What is carried widened by the merges.** `land-locally` and `approval-policy`, both `method`, now state what `no-local-land-with-remote`, `guard-blocks-push-to-main`, `approve-pins-the-review` and `land-policy-from-base` stated as CLM's own. Each describes what `clm` itself does in any project, so no project is held to more than before.
- **Template text under `#### Enforced by`.** The audit questioned `init/claude-md-points-at-guide` and `init/architecture-template-proof`, which test sentences in files `clm init` writes. Both were taken off the lists; the requirements stay.
