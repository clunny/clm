---
status: applied
priority: P1
resolves: [builds-pre-land-checks-queue-behind-lands-and-run-again-at-land]
---

## Intent

Stop a build's pre-land check from queueing behind every other check on the clone, and stop its land from checking the same tree again. `clm land --check-only` in a build place (`CLM_SLOT` not 0) runs outside the clone's check lock, still running every test, and, when it passes on a clean working tree whose committed `HEAD` contains `origin/main`, appends a `checked` record for `HEAD`'s code key. A land of that branch made before `main` moves then finds its code key checked and runs only `clm check --no-detectors` and the readers of checked code, taking no lock; when `main` has moved, the land runs its usual selected check on the merge, which stays the safety net. The build's prompt says why it checks after its last commit and rebase and commits nothing between the check and the land, and has it run the formatter, the linter and its own tests by hand first. Decision: `worktree-check-counts`; `check-runs-one-at-a-time`, `land-check-command` and `fixed-land-tree` are amended, and the guide's `ci-and-deployment` page says a build's check counts for its land. Resolves `backlog/builds-pre-land-checks-queue-behind-lands-and-run-again-at-land`.

## The human's answers

> do we need to run so many land checks? and could we catch issues for lands in the dev worktree before doind a bottlenekced land check? eg linting, formatting, tests etc.

> could pre-land check be not serialized?

Asked whether to design a change that takes builds' pre-land worktree check out of the clone lock and lets a passing worktree check on an up-to-date, committed HEAD count as the land's check:

> Both parts as a P1 item, this will make a big difference

## Problem

Each build checks its code twice through one serialised queue. In its worktree, `clm land --check-only` runs the whole `[land] check` with no `CLM_SELECTION`, under `.clm/check.lock`, often twice (after the review's fixes and before landing). Its land then runs `[land] check` again on the merge, under the same lock, even when `main` hasn't moved and the merge is the tree the build just checked. With `/clm:build` running several builds, lands wait behind a queue of full checks (388 s each in this repository, measured 2026-11-07). `check-runs-one-at-a-time` deferred turning the lock off until the serialised checks were shown to be the bottleneck; this is that evidence, and `tests-run-concurrently` with `CLM_SLOT` (`build-slots`) has since removed the lock's reason for runs in a build place.

## Approaches

- **Chosen: both parts, keyed on the slot and the code key.** The pre-land check in a build place takes no lock, since its slot keeps its tests apart from every other run's; runs outside a place share slot 0 and keep the lock, as do lands and `clm verify`. A passing check on a clean, committed, up-to-date `HEAD` writes the record a land would, matched by exact code key as every `checked` record is (`legacy-code-key`). The pre-land check stays a full run, so it proves more than a land's selected run and keeps the full runs in builds that `graph-test-selection` relies on; outside the lock it costs the build's own time, not the queue's.
- **A setting to turn the lock off.** Rejected (`yagni`): the slot already tells which runs can overlap, and a project whose tests can't overlap breaks `tests-run-concurrently` whatever a setting says.
- **Turn the lock off for every `--check-only` run.** Rejected: runs outside a place all have slot 0, so two of them (a human's checkout and a draft's code fix) could collide on a port keyed on it; keeping the lock for them costs one condition.
- **Hand `--check-only` the land's selection.** Rejected after the architecture audit: faster, but it would drop the full suite's runs in builds, the safeguard `graph-test-selection` counts on for a test the selection skips.
- **A dry-run land (fetch, merge, check, no push).** Rejected as `land-check-command` rejected it: with `HEAD` containing `origin/main`, the worktree already is the merge.

## Outcome

- `land/check-only` (modified): still runs every test (`CLM_SELECTION` unset); after both commands pass, appends `checked` for `HEAD`'s code key with a land's command when the tree was clean before and after, `HEAD` didn't move and `origin/main` is an ancestor of `HEAD`, printing `recorded <HEAD> as checked`, or else `not recorded: <reason>`; a failed append only warns. `checks pass` stays its last line.
- `land/check-lock` (modified): a `--check-only` run whose `CLM_SLOT` isn't 0 neither takes nor waits for the check lock; every land's check and every slot-0 `--check-only` run still does. It needs `orchestrator/place-slots`, which `land/check-slot`, the slot it reads, waits on.
- `land`'s Contract and Design say so; `land`, `land-target-lock` and `selected-tests` are unchanged, since a land already counts any `checked` record for its merged tree's code key.
- `ledger`'s Contract says who writes a `checked` record; no record kind or field changes.
- `orchestrator.run/prompt-recorded-check` (added): the prompt says why the check runs after the last commit and `rebase ok` and nothing is committed before the land, what `not recorded:` costs, and to run the formatter, the linter and the changed code's tests by hand first.
- Decision `worktree-check-counts`; amendments to `check-runs-one-at-a-time`, `land-check-command` and `fixed-land-tree`; the guide's `ci-and-deployment` page says builds' pre-land checks run outside the check lock, count for the land, and so must pass from a clean checkout.

## Constraints

- `deterministic-enforcement`: the land still decides; a stale record can't match, since a moved `main` changes the merged tree's code key whenever the code differs.
- `tests-run-concurrently`, `build-slots`: what makes overlapping builds' checks safe.
- `land-target-lock`: unchanged; `--check-only` never takes the land lock.
- `legacy-code-key`: a land accepts only an exact code-key match.

## Edge cases and workarounds

- Ignored state: a worktree check runs beside the build's own `target/` and other ignored files, while a land checks a tree `git clean -fdx` emptied of them. A check that passes only through an ignored file it doesn't make itself is the project check's defect; the next land after `main` moves, or `clm verify`, shows it. Accepted.
- The readers of checked code (`clm affected --readers`) still run at the land when the branch changes a doc-only path a test reads, though the worktree check already ran them on the same tree: the code key can't tell. Accepted: they are few, and the check runs only them.
- A `[land] check` that deploys to a shared test environment now runs from several builds at once; such a project already breaks `tests-run-concurrently` and must keep the environment out of what builds run, as the guide says.

## Open questions

- `land/check-slot` still ends "The check lock (`check-lock`) and `CLM_SELECTION` SHALL be unchanged by it" (architecture audit). Left as is: setting the slot changes neither; it is `check-lock` that now reads the slot to decide on the lock, and its text says so.
