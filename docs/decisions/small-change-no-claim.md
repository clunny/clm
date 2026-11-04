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
