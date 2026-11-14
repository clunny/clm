---
status: done
---

## Intent

`check.affected` depends on `check.coverage`, so test selection can read every marker of a file with `marker_spans`, as `check.affected/affected-files` has said since `20261114-proof-is-real`. It is the same edit `20261114-health-reads-coverage` made for `check.health`, under the same rule (`check-coverage-split`), and it lets the code fix of `backlog/three-readers-of-every-marker-still-read-only-the-ones-that-count` reach the call. Part of phase 3 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`.

## Problem

`affected-files` selects a file for the component its markers cite, "inside a test or outside one". Its code reads markers with `clm_check::markers_in`, which will hold only the markers inside tests. `clm-affected` reads file texts, from a checkout or from two commits, so it can't use `scan`'s `outside`, and `clm-check` passes on no call that returns every marker of a text. `check.affected` depended on `check` alone.

## Approaches

- **Taken: `check.coverage` in `depends_on`**, and `marker_spans` imported from `clm-coverage`. `check-coverage-split` says a call `clm-check` doesn't pass on is reached this way.
- **Not taken: a re-export from `check`.** The one that would serve, `check/coverage-lists`, waits on the marker rule, which waits on this fix.

## Outcome

`check.affected`'s frontmatter names `check.coverage`. No requirement is added, modified or removed, so the change is archived on its branch. The code is the bug item's.

## Constraints

No behaviour changes with this change alone.
