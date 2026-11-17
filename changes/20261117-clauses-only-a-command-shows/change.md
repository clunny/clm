---
status: proposed
from: [twenty-view-tests-show-what-only-the-commands-do, six-check-coverage-tests-assert-what-other-crates-do, view-repo-s-commit-line-is-rendered-and-proven-in-view]
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, step 4 of "Phase 4, what is next", which waits on this change. Step 3 moved every test it could into the crate of the component it cites and left 27 that can't move as they stand: 20 of `view`'s and six of `check.coverage`'s assert what only the `clm status`, `clm view` and `clm pending` commands, or `clm-check`'s narrowing of the pending list, do, and one in `view`'s file proves a line `view.repo` states and `view`'s code draws. This change gives each such clause to the component whose crate can show it, so the tests move under their own names and `clm/routes-only` waits on nothing else.
