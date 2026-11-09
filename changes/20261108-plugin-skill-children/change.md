---
status: applied
---

## Intent

Split `plugin`, at 45,580 spec tokens nearly four times its 12,000 budget and the largest spec in the graph, into children along its skill files, so a build of one skill loads that skill's rules and not every skill's (`slim-components`, `nested-splits`). The requirements that concern one skill move to that skill's child word for word; the rules several skills share stay in `plugin`. Nothing a skill says changes: only where its rules live and which build owns its file.

## Problem

`plugin` holds 79 requirements, and most of them each constrain a single file: `plugin/skills/propose/SKILL.md`, `plugin/skills/triage/SKILL.md`, `plugin/skills/build/SKILL.md`, `plugin/skills/init/SKILL.md`, `plugin/skills/upgrade/SKILL.md`, or one of the four subagents under `plugin/agents/`. Every build that touches any skill loads all 79, plus a 7,700-token Design that walks through each skill in turn, and `plugin`'s Contract and Design also ride in the bundle of every child it already has (`plugin.hooks`, `plugin.panel`). Each skill file changes for its own reasons, at its own pace, and imports nothing: a real seam, not a number to hit (`clm survey` on the cut exits 0).

## Approaches

- **A. Trim the eight largest Designs** across the graph, leaving every requirement alone. No build, lands on checks, about 20–25k tokens cut, but `plugin` stays at about 40k. Not chosen for this pass.
- **B. Split `plugin` along its skills (chosen).** The single largest flag; the skills are separate files that change for separate reasons. Costs a review (the change edits `ARCHITECTURE.md`) and a mechanical rebuild that re-cites the moved requirements' tests.
- **C. Merge duplicate scenarios** in the hotspots (`push-guard`, `checkpoint`, `build-rows`, `parse-backlog`). Every touched requirement goes `behind`, for less text saved than A. Not chosen.

Within B, for the rules that name two or more skills:

- **1. Keep them in `plugin` as shared rules (chosen).** They hold today and rarely change; a later change to one adds a requirement to each affected child, as `dependency-work-is-a-requirement` asks of work in another component's `paths`. The move stays word for word and nothing is duplicated.
- **2. Split each into one copy per skill.** Each copy would be owned outright, but the text grows by about 2k tokens and every copy is reworded and reviewed. Not chosen: it works against the slimming.

## Outcome

Six children of `plugin`, each with the `paths` of its files, `serves: [architecture/method-in-the-plugin]`, a short Contract naming the file it owns, and a Design taking over `plugin`'s paragraphs about that file:

| Child | `paths` | Requirements moved from `plugin` |
|---|---|---|
| `plugin.propose` | `plugin/skills/propose/**` | `propose-brainstorms`, `propose-claims`, `propose-priority`, `propose-nests-splits`, `propose-code-fixes`, `propose-archives-design-only`, `code-fix-needs-a-written-rule`, `propose-elaborates-ideas`, `propose-clears-workarounds`, `propose-in-a-fork`, `propose-adds-glossary-terms`, `propose-replacing-constraints`, `builder-check-asks-migrations`, `builder-check-capability-tests`, `propose-adds-settings-sections`, `propose-harness-only-work` |
| `plugin.triage` | `plugin/skills/triage/**` | `triage-skill`, `triage-caps-drafts`, `triage-answers`, `triage-menu`, `triage-checks-splits`, `triage-folds-renames-alone`, `triage-enters-branch-worktree`, `triage-waiting-drafts`, `triage-revises-declined-fold`, `triage-contributions` |
| `plugin.build` | `plugin/skills/build/**` | `skill-named-build`, `build-subagents`, `build-archive-branch`, `build-reads-origin`, `build-finishes-builds`, `build-watches`, `build-reports-workarounds`, `build-waiting-builds`, `build-stops-landed-builds`, `build-says-what-is-ready`, `build-unclosed-builds`, `build-resumes-sound-builds` |
| `plugin.init` | `plugin/skills/init/**` | `init-skill`, `method-pointers`, `init-skill-asks-verify`, `init-skill-concurrent-tests` |
| `plugin.upgrade` | `plugin/skills/upgrade/**` | `upgrade-skill`, `upgrade-skill-offers-verify`, `upgrade-skill-settings-by-hand`, `upgrade-skill-notes-without-edit` |
| `plugin.agents` | `plugin/agents/**` | `spec-auditor`, `spec-auditor-keeps-checkout`, `auditor-flags-unstated-fixes`, `spec-slimmer`, `architecture-auditor`, `issue-verifier` |

Each requirement moves as `nested-splits` says: REMOVED from `plugin`'s delta and ADDED, text, scenarios, `#### Needs` and `#### Serves` unchanged, to the child's. A child is a leaf: it imports nothing, so its `depends_on` is empty, and its rules' `#### Needs` lines order its builds as they did in `plugin` (`needs-alone-order`).

`plugin` keeps the shim and its targets, the manifest, the hooks wiring, the `/clm:view` fallback, the small skills (`help`, `health`, `configure`, `contribution`) and the rules shared by several skills: `skills`, `builder-check` (propose and build), `session-worktrees` (propose, build and init), `architecture-audit` (propose and contribution), `defect-check-before-pr`, `recommend-the-right-design`, `skills-write-serves`, `skills-write-enforcement`, `launch-models`, `renamed-commands`, `says-architectural-constraint`, `skills-say-new-words`, `skills-say-component`, `skills-push-with-clm`, `unattended-permissions-check`, `wrap-up-left-lines` and `skills-run-verify`. Its `paths` shrink to the files no child owns, its `depends_on` gains the six children, its Contract says which child owns which file, and its Design keeps only what is the plugin's as a whole: the shim, the hooks, the change flow every skill follows, the session worktrees, the method's place in the plugin and the absence of evals. Each per-skill paragraph moves to its child's Design, with the sentences that restate a requirement word for word cut.

References re-pointed to the moved IDs:

- `ARCHITECTURE.md`: `tests-run-concurrently`'s `#### Enforced by` names `plugin.init/init-skill-concurrent-tests`.
- `#### Needs` lines in other components: the root's requirement needing `plugin/init-skill`, `init`'s needing `plugin/skill-named-build`, `orchestrator.run`'s needing `plugin/build-waiting-builds`, and `orchestrator.triage`'s needing `plugin/triage-waiting-drafts` and `plugin/architecture-auditor`.
- Spec text: `model/carried-constraints` cites `plugin/architecture-auditor`, `upgrade/migration-offer-verify` cites `plugin/upgrade-skill-offers-verify`, and `orchestrator.triage`'s Design cites `plugin/triage-waiting-drafts`.
- Backlog: `workaround-plugin-propose-adds-glossary-terms-1` moves to `plugin.propose` at the requirement's version after apply, and `sessions-sharing-a-checkout` keeps citing `session-worktrees`, which stays in `plugin`.

Decisions under `docs/decisions/` that cite the old IDs record history and are left as written.

Expected sizes, in spec tokens: `plugin` from 45,580 to about 16,000, still over its budget, since its shared rules alone come to about 12,700; `plugin.propose` about 9,000, `plugin.triage` about 7,700, `plugin.build` about 6,000, `plugin.init` about 3,500, `plugin.agents` about 3,100, `plugin.upgrade` about 1,400. Code tokens move with the files and with the tests' citations (`tests-count-where-cited`).

## Constraints

- The skill files, the subagents and the tests' assertions don't change. The builds re-cite the moved requirements' markers in `crates/clm/tests/plugin.rs` under the new IDs (`moved-markers-wait`): each child's build re-points its own, and `plugin`'s obsolete lines wait for them.
- The requirements whose `#### Needs` line or text is re-pointed (in the root, `init`, `orchestrator.run`, `orchestrator.triage`, `model` and `upgrade`) change version, so each of those components re-cites one or two markers; nothing they do changes.
- `paths` stay disjoint: the children take only files that were `plugin`'s.
- No new `clm` behaviour, no new term, no settings key, no migration: nothing a project set up by a release holds changes.

## Left for later

`plugin` stays oversized at about 16k. What remains is the shared rules: the next pass is Approach A's Design trims across the graph, or merging the shared rules' overlapping scenarios.
