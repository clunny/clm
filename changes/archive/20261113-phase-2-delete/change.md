---
status: done
resolves:
  - moving-the-builder-check-rule-builds-tests-the-plan-deletes
  - runner-loop-on-the-stepper
  - runner-per-agent-harness
  - workaround-gate-classify-1
  - workaround-web.views-home-waiting-on-you-1
  - workaround-init-survey-components-1
  - workaround-plugin-propose-adds-glossary-terms-1
---

## Intent

Phase 2 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims`: delete. It removes `clm runner` and its component, the requirements that are satisfied only by a sentence existing in a skill, an agent definition, a prompt or an instruction file, the superseded decisions, and the stale workaround flags. Nothing is added but one decision that records the rule, one requirement of the root that states the runner is gone, so that its build removes the crate and the command, and the backlog items that name what a program could hold instead.

## Problem

The audit of 2026-11-13 found about 130 requirements whose tests check for string fragments in a prompt or skill. Such a test fails only when someone edits the sentence, and proves nothing about what a session does, which contradicts `deterministic-enforcement`. The plan also cut `clm runner` from scope: unattended builds run through `/clm:build` in a session. Fifty decisions were superseded yet still on disk, nine of them still cited from live specs and skills, and 86 accepted decisions carried amendments a reader had to reconcile. Four workaround flags named blockers that are built.

## Approaches

- **One change on one branch, approved once** (chosen). The branch edits `ARCHITECTURE.md` (one line of `land-locally`), so it needs review, and the approval's fingerprint covers every removal. The human approves over the list under "What is removed" and reads no pass over what is kept.
- **A change per kind of removal** (runner, prompt text, decisions). Rejected: only the branch that edits `ARCHITECTURE.md` would need review, so the other removals would land on checks, against the plan's safeguard that a removal of behaviour has the human's approval.
- **Deleting every requirement about a written prompt.** Rejected with the human: that a prompt names the right branch, model or check command is behaviour a test can prove. The line is in `prompt-text-is-source`.
- **Keeping the five skill components as empty owners.** Rejected with the human: a component with no requirement has nothing to build or prove. Their skill files return to `plugin`.

## Outcome

- 120 requirements are removed and 24 are split: each split one keeps its ID and the part a program computes, and loses its wording clauses and the scenarios that state only wording.
- Six components are removed: `orchestrator.runner`, and `plugin.propose`, `plugin.triage`, `plugin.init`, `plugin.build` and `plugin.upgrade`, whose skill files `plugin` owns again. 57 components become 51.
- `plugin/skills` is the one requirement that says the skills ship: exactly the ten, each with a description. It takes that over from nine removed requirements.
- `clm/no-runner-crate` is added, as `clm/no-watch-crate` was for the last removed crate: no crate `clm-runner` and no `runner` subcommand. Without it nothing but obsolete markers would make the removal pending, and a build could satisfy those by deleting the tests alone.
- Ten more requirements change only to drop a `#### Needs` line or a citation that named something removed; no scenario of theirs changes.
- `ARCHITECTURE.md`: `land-locally` no longer lists `orchestrator.runner/runner-needs-guard` under `#### Enforced by`. No other entry names a removed requirement.
- 51 decisions are deleted (the 50 superseded ones and `machine-runners`). What seven of them still held is restated in the decision that replaced each. 86 amended decisions are folded into one current text each. Citations of the deleted names are repointed in the specs and the decisions.
- Names in live specs that resolved to nothing are fixed or dropped: `trunk-named-main` (now `trunk-is-main`), `panel-inspector-contract`, `regenerate-applied-live-changes`, `shared-build-target`, and `humans-cut-releases` where it was a citation.
- Four workaround flags are deleted. Three named blockers that are built; the test gap behind each is a `bug` item. The fourth goes with its requirement.
- The Worker question is answered by the human and recorded in its item.

## What is removed

120 requirements, in 12 components. Six components go whole.

- **`clm`** (1): `claude-md-pushes-with-clm`.
- **`init`** (2): `claude-md-points-at-guide`, `claude-md-names-the-skills`.
- **`orchestrator.run`** (18): `prompt-commands`, `ask-by-backlog`, `delete-fixed-items`, `no-shared-install`, `background-work-stopped`, `check-tree-still`, `scratch-files`, `prompt-spot-checks`, `review-every-diff`, `capability-tests-follow-rule`, `prompt-pushes-through-clm`, `prompt-recorded-check`, `bounded-test-runs`, `template-migration-fix`, `prompt-unopened-pr`, `prompt-doc-readers`, `prompt-question-choices`, `stays-in-its-component`.
- **`orchestrator.runner`** (9, the whole component): `runner-launch`, `runner-refusals`, `runner-finish`, `runner-worktree-leftover`, `runner-prerequisites`, `runner-once-and-stop`, `runner-needs-guard`, `paragraph-says-pull-request`, `says-component`.
- **`orchestrator.triage`** (17): `background-work-stopped`, `draft-code-fix-written-rule`, `draft-restraint`, `draft-backlog-only-change`, `builder-check-model`, `draft-scratch-files`, `builder-check-order`, `draft-contract-step-deferred`, `draft-corrects-bug-components`, `draft-hand-edit-check`, `draft-adds-glossary-terms`, `draft-architecture-audit`, `draft-capability-test-fixes`, `prompts-push-through-clm`, `bounded-test-runs`, `draft-records-promised-code-work`, `draft-question-choices`.
- **`plugin`** (21): `defect-check-before-pr`, `session-worktrees`, `view-fallback-wording`, `health-skill`, `skills-write-serves`, `skills-write-enforcement`, `recommend-the-right-design`, `renamed-commands`, `says-architectural-constraint`, `skills-say-new-words`, `skills-say-component`, `contribution-skill`, `architecture-audit`, `help-skill`, `skills-push-with-clm`, `unattended-permissions-check`, `wrap-up-left-lines`, `skills-run-verify`, `configure-skill`, `verify-skill`, `skills-follow-agent-harness`.
- **`plugin.agents`** (2): `spec-auditor-keeps-checkout`, `auditor-flags-unstated-fixes`.
- **`plugin.build`** (5, the whole component): `skill-named-build`, `build-runs-the-loop`, `build-reads-the-cause`, `build-approves-in-session`, `build-keeps-the-human-in-sight`.
- **`plugin.init`** (8, the whole component): `init-skill`, `init-skill-asks-verify`, `init-skill-concurrent-tests`, `init-skill-build-setup`, `init-skill-marks-lockfiles`, `init-skill-marks-doc-readers`, `init-follows-agent-harness`, `init-skill-gives-tests-their-component`.
- **`plugin.propose`** (19, the whole component): `propose-brainstorms`, `propose-claims`, `propose-priority`, `propose-nests-splits`, `propose-code-fixes`, `propose-archives-design-only`, `code-fix-needs-a-written-rule`, `propose-elaborates-ideas`, `propose-clears-workarounds`, `propose-in-a-fork`, `propose-adds-glossary-terms`, `propose-replacing-constraints`, `builder-check-asks-migrations`, `builder-check-capability-tests`, `propose-adds-settings-sections`, `propose-harness-only-work`, `propose-fix-runs-setup`, `propose-follows-agent-harness`, `builder-check`.
- **`plugin.triage`** (14, the whole component): `triage-skill`, `triage-caps-drafts`, `triage-answers`, `triage-menu`, `triage-checks-splits`, `triage-folds-renames-alone`, `triage-enters-branch-worktree`, `triage-waiting-drafts`, `triage-revises-declined-fold`, `triage-contributions`, `triage-archives-complete`, `triage-follows-agent-harness`, `triage-verify-log`, `triage-web-answers`.
- **`plugin.upgrade`** (4, the whole component): `upgrade-skill`, `upgrade-skill-offers-verify`, `upgrade-skill-settings-by-hand`, `upgrade-skill-notes-without-edit`.

## Requirements split

Each keeps its ID and loses the clauses that state fixed wording.

- `plugin/skills`: the plugin ships exactly its ten skills, each with a description.
- `plugin/method-pointers`: every `clm guide <page>` a skill or agent names is a page `clm guide` prints.
- `plugin/launch-models`: the three agents set `model: sonnet`.
- `plugin.agents/spec-auditor`, `spec-slimmer`, `architecture-auditor`, `issue-verifier`: each ships with the frontmatter the harness reads and a tool list that can't edit; the verifier's tools are exactly `Read`, `Glob` and `Grep`.
- `plugin.hooks/defect-rule`: what the session-start paragraph holds for each `defects` setting.
- `orchestrator.run/run`: the claim, the resume, the prompt file, the launcher and the exit codes; the prompt names its branch, the skill paths and the plan path, says a resumed branch exists, and ends with the bundle.
- `orchestrator.run/pending-prompt`: the pending list with its diffs and marker locations, blocked requirements left out, read from the merged tree.
- `orchestrator.run/prompt-check-command`: the prompt names `clm land --check-only` and the configured command only when one is set, and never `CARGO_TARGET_DIR`.
- `orchestrator.run/prompt-setup`, `prompt-slot`, `prompt-serves`, `review-model`, `capability-waits-for-changed-rule`, `prompt-pr-body`, `prompt-workaround`: the computed value or condition each puts in the prompt.
- `orchestrator.triage/draft`, `draft-answers`, `draft-prompt-setup`, `draft-workaround-deferred`, `fold`: the claims, the IDs, the item lines, the answers block and where it sits, the fold's branch names and order.
- `orchestrator.loop/plan-archive`: when the archive run is launched, and on which branch.

## Tests

`deleted-tests.txt` in this directory lists the 187 tests that go whole and the one that only loses a marker. A script produced it from this change's deltas and the markers on the branch: a test is listed when every requirement it cites is removed, or when it is a text-scan test of a split requirement, named by hand after reading it. A build deletes no test that isn't on the list. A test that mixes computed assertions with wording needles is not on it: it keeps the computed assertions, loses the needles and is re-cited at the requirement's new version.

Four removed requirements of `orchestrator.run` had one computed part each, that their text appears only when `[land] check` is set: `background-work-stopped`, `bounded-test-runs`, `check-tree-still` and `prompt-recorded-check`. `prompt-check-command` states that condition and `scenario_no_check_command` proves it over the whole prompt, so their two condition tests are duplicates and are on the list.

## For the builds

- **Each build removes or cuts the tests that cite its own requirements, wherever they sit.** Most of them are in the root's files under `crates/clm/tests/`, since the CLI's tests prove other components through the binary (`backlog/clm-s-cli-tests-prove-other-components-through-the-binary`, the plan's phase 4). `clm pending <component>` names each obsolete marker with its file, and a build removes the test it marks (`removed-requirements-build-out`) and re-cites or cuts a test of a requirement that changed (`stale-markers-keep-behind`).
- **The root** deletes `crates/clm-runner/`, the `clm runner` dispatch (the `Runner` command in `crates/clm/src/main.rs` and its arm in `crates/clm/src/upgrade_gate.rs`) and its `Cargo` entries, and `crates/clm/tests/runner.rs` (`clm/no-runner-crate`). The crate is under the root's `paths` from this change on, so no file is unowned; the glob matches nothing once the crate is gone. The root's test files also hold the tests of the removed skill requirements (`plugin_propose.rs`, `plugin_triage.rs`, `plugin_init.rs`, `plugin_build.rs`, `plugin_upgrade.rs`, `claude_md.rs`), which go whole.
- **`orchestrator.run`** and **`orchestrator.triage`** delete the listed tests and cut the mixed ones. The prompts' text stays as it is, but for the removed requirement names it cites (`backlog/dead-decision-names-in-code-comments-and-skills`).
- **`plugin`** cuts `skills`, `skills_point_at_the_guide` and `launch_models` to their computed assertions, and `skills` gains the check that `plugin/skills/` holds exactly the ten.
- The glossary keeps its `clm runner` lines, and the root's `paths` the crate's glob, until the command and the crate are gone (`backlog/the-glossary-still-lists-clm-runner`).

## Constraints

- Only deleting. No surviving requirement is reworded beyond the cut; that is the plan's phase 5. The `says-component` family, the migration notes of `upgrade`, and the retired-word lints on `init`'s templates and on help text scan source or templates, not prompts, and wait for phase 5. Two requirements of `init` do go here: they pin sentences of the `AGENTS.md` that `clm init` writes, which is an instruction file.
- No migration: `clm runner` has no settings key and `clm init` writes nothing for it, and no file a project holds changes.
- The live changes `20261113-land-starvation` and `20261112-components-own-their-tests` are untouched; their deltas still cite two of the dead names, in text this change repoints in `specs/`.
- `20261113-builder-check-text-lives` is live too. The requirement it added, `plugin.propose/builder-check`, is removed here, so nothing of it is left to build but the removal of `plugin/builder-check`'s two obsolete tests; it is archived once they are gone.

## Decisions

- `prompt-text-is-source` (new): skill and prompt text is source, and a requirement states only what a program computes.
- 51 decisions deleted and 86 folded, as under Outcome.

## What a person loses

- `clm runner`: running builds headless from a plain terminal, cron or a service with no session open. Unattended builds need a session running `/clm:build`.
- The tests that failed when a sentence was edited out of a skill, an agent, a prompt or an instruction file. The sentences stay. Where a program already refuses the outcome a sentence warns of, nothing is lost; where none does and one could, a backlog item names it; the rest waits for the skill evals of the plan's phase 6.
- The history of how 86 decisions changed, which git keeps.
- `machine-runners` was the one deleted decision still accepted. Its record that hosted sandboxes were declined, and what a headless runner would start from, is kept in `builds-in-subagents`.

## Open questions

From the builder check and the architecture audit; none holds the change.

- **Most of the tests to delete are in files the root owns.** A build of `plugin`, `plugin.agents`, `init` or `orchestrator.run` deletes or cuts tests under `crates/clm/tests/`. The pending list hands each build those markers and `removed-requirements-build-out` says the build removes what they mark, but `path-ownership`'s written exception names only re-citing. A build that reads it strictly files a `question` and stops; the answer is that it may. Phase 4 moves the tests to their components.
- **A capability test still reads fixed prompt text.** `crates/clm/tests/capability/right_model_per_task.rs` asserts three sentences of the build prompt about naming a subagent's model, which `orchestrator.run/review-model` no longer states. The test passes while the prompt keeps them. Whether the capability's proof should rest on prompt wording is phase 3's question.
- **Two split requirements keep a fixed sentence under a computed condition.** `orchestrator.run/prompt-serves` and `capability-waits-for-changed-rule` say what the capability component's prompt tells the session. What a test can prove is that the sentence is there for that component and for no other; the wording itself is unpinned in effect. Left as borderline for phase 5.
- **The build budget's number is unpinned.** `scenario_budget_in_land_terms` asserted the prompt's "at most 6 requirements", a constant. It goes with the wording.
- **`clm/claude-md-pushes-with-clm` was a lint a program ran**, on this repository's `AGENTS.md`. It is removed as a scan of instruction text; `backlog/a-session-s-raw-git-push-is-refused-by-nothing` names the guard as the program that would hold the rule.
