---
status: done
resolves: [clm-pending-describes-the-checkout-without-naming-its-commit]
---

## Intent

Make `clm pending` say which commit it describes and warn when that commit is behind `origin/main`, as `clm status` and `clm view` do, and let it read a commit's tree with `--at <rev>`, as `clm view` can. `/clm:build`'s closing summary then takes the requirements each `question` and `human-task` item blocks from `clm pending --at origin/main`, which agrees with `clm next`, rather than from the session's checkout, which the skill may not sync.

## Problem

`views-name-their-commit` says every view names the commit it describes and warns when it is behind `origin/main`, but only `clm status` and `clm view` do. `clm pending` reads the working tree too, names no commit and has no way to read `origin/main`. In a `/clm:build` session whose checkout was behind, the closing summary listed a requirement as `behind` and others as blocked after builds had landed them or their blockers, while `clm next`, which reads `origin/main`, disagreed. The skill forbids syncing the checkout, which may hold the human's work, so the session had no correct source for the summary.

## Approaches

- **A. `clm pending` names its commit, and takes `--at <rev>` (chosen).** It follows the constraint every view follows, and the build skill reads `origin/main` without touching the checkout. `--at` reuses `view.repo`'s reading of a commit's blobs and its first line (`view-at-rev`), and `clm_check::pending` already works on a graph and markers loaded without a checkout, so little is new.
- **B. Only the warning.** The summary would at least say it may be stale, but the session still couldn't give the right list, since it may not sync. Doesn't fix the summary.
- **C. Only a new source for the summary,** such as `clm view blockers --at origin/main`. That view lists only components that can't start at all, not the requirements an item blocks on a component that has other work, so it isn't the list the summary needs; and `clm pending` would still break the constraint for every other reader.

## Design

- **`view.repo/pending-names-its-commit`.** `clm pending` prints the checkout line (`commit <hash>`, then `⚠ <n> behind origin/main` when behind) on **stderr**, without colour, so its stdout stays the list `check.coverage/pending-report` defines: one line per pending requirement, empty when nothing is pending, read a line at a time by sessions and tests. `--at <rev>` reads that commit's tree as `clm view --at` does, without fetching or writing, and its stderr line is `view-at-rev`'s first line. The command's reading moves into `clm-view-repo`, beside `status` and `view`, which already read the checkout and a commit's blobs; the narrowing to one component is shared with `clm_check::pending_in`, not copied.
- **`check/pending-narrows-on-a-graph`.** That narrowing lives in `crates/clm-check`, which `view.repo`'s build may not edit (`path-ownership`), so `check` exports it as `pending_for(graph, markers, component)`, which `pending_in` calls, and `view.repo`'s rule needs it (`dependency-work-is-a-requirement`).
- **`plugin/build-reads-origin`.** The build skill's summary takes the requirements each `question` and `human-task` item blocks from `clm pending --at origin/main`, run after `clm next` has fetched, never from `clm pending` alone.

Edge cases and workarounds known now: none. The warning compares with `origin/main` as of the last fetch, as the views' does; `--at origin/main` is only as fresh as that fetch, which `clm next` makes just before.

## Open questions

- **`views-name-their-commit`'s `#### Enforced by`.** The architecture audit noted that the entry lists only `view/status-report` and `view/view-command`, not the new `view.repo/pending-names-its-commit`. Left as it is: the new rule's `#### Serves` names the entry, so the trace exists, as it does for `view.repo/view-at-rev`, which the list doesn't name either, and the entry's scenario (`clm status`) stays true. Adding the line is a hand edit of `ARCHITECTURE.md` the human may want.
