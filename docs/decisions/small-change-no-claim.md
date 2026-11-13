---
title: "A small code change with no spec change takes no claim"
date: 2026-11-03
status: accepted
---

A session with a small code improvement on a converged component had no claim to take: `clm claim <component>` refuses a converged component, and `--fix` wants a `kind: bug` or `kind: deferred` item. It landed from a plain branch, which `clm land` accepted, without anything saying that was allowed.

That is the right route, and now it is stated. A claim holds a component's place in the build queue, so two builds of it don't race. A change with no requirement behind it isn't in that queue, and two branches that touch the same files already meet at `clm land`, which merges `main` in and checks the result. The gate classifies a branch by the paths it changes, not by its name.

So such a change goes on a branch of its own from `origin/main`, named outside `impl/`, `spec/`, `propose/` and `autosync/`, in a worktree of the session's own. `clm gate --base origin/main` must print `code`. The session pushes it, opens the pull request and runs `clm land <branch>`, then deletes the pushed branch, since `clm land` deletes only claim branches (`land/landed-branch-cleanup`). A session already on an `impl/*` branch whose components hold the change folds it there instead (`fold-small-defer-large`).

It fits only when every requirement and scenario as written stays true and the change adds nothing a requirement would have to state: a refactor, a clearer message or hint, a test, a comment. A behaviour someone would rely on starts on a spec branch, since code no rule states can be removed unseen (`code-fix-needs-a-written-rule`). A `kind: bug` or `kind: deferred` item takes `clm claim <component> --fix`, whose trailer deletes it (`code-fix-resolves-bug`). Telling these apart is reading, not a program's, so nothing enforces it (`deterministic-enforcement`).

`CLAUDE.md` and the guide state the route for people and sessions in this repository. Other repositories have no such line in their `CLAUDE.md`, so `clm claim`'s refusal of a converged component names it as well (`orchestrator/claim`).

Rejected:
- **A `clm claim` form that takes no item.** `code-fix-resolves-bug` limits `--fix` to a named item so that it can't become a general way around the queue's refusals. This would be that way around, and it would hold the component out of the queue for nothing.
- **Filing a `kind: bug` item to get a `--fix` claim.** It records an improvement as a defect, for a claim that protects nothing here.
- **`clm land` deleting every landed branch from `origin`.** `land/landed-branch-cleanup` leaves other branches on purpose: land owns claim branches, not a branch someone may have pushed for another reason.

**Amended 2026-11-04** (`20261104-no-stated-route-for`). The same route takes an edit that changes only `docs/` or `CLAUDE.md` and no requirement, scenario or design: rewording or adding a guide page, or recording a decision for a choice already made. Those are spec paths, so its gate prints `spec`, not `code`, and it lands on checks (`yes-no-review`); everything else above holds, including deleting the pushed branch after the land. It needs no change, since a change exists to carry deltas and burdens, and such an edit has neither: a `change.md` with only an intent would be a record to archive for nothing. It isn't named `spec/*`, which names a change by its ID and which `clm proposals` reads for claims. When its gate prints `spec review` (an edit to `ARCHITECTURE.md` or `CAPABILITIES.md`), or the edit needs `specs/` or `changes/`, it is design work and starts on a spec branch. `clm land` and the gate already allow all of this (`land/spec-branch-holds-spec-paths` binds only `spec/*` branches), so only the text that states the route changes.

**Amended 2026-11-07** (`20261107-harness-only-capability-work`). Harness-only work on the capability suite takes this route even when a `kind: bug` or `kind: deferred` item names it: the commit that deletes the item holds `Resolves: backlog/<slug>`, which `code-fix-resolves-bug` accepts without a claim, and no `clm claim --fix` is taken, since no requirement states the harness (`human-layer-proof`).

**Amended 2026-11-08** (`20261108-draft-prompt-has-no`). The harness exception above covers all test plumbing: work that changes only how a component's own test suite runs (its runner settings, the ports and servers its tests start, the paths they write) and nothing any of its tests asserts takes this route too, its item deleted by a `Resolves: backlog/<slug>` trailer with no `clm claim --fix` (`code-fix-needs-a-written-rule`).

**Amended 2026-11-13** (`20261113-a-test-plumbing-fix`). The trailer deletes the item only when the branch's code belongs to a component the item names (`gate/backlog-additions`): `human-layer-proof`'s "on any branch" speaks of the branch's name, not of the item's `components`. One plumbing fix lay under `capabilities/panel/`, which is `capability`'s, for an item that named `plugin.panel`, and its gate printed `mixed`. A draft that meets such an item corrects its `components` on its spec branch and leaves the fix to the next draft, as it does for a code-only bug filed on the wrong component (`orchestrator.triage/draft-corrects-bug-components`). The gate keeps its bound, since it can't tell test plumbing from other code (`deterministic-enforcement`), and without the bound any code branch could delete any bug.
