---
status: proposed
resolves: [a-template-and-its-migration-live-in-different-components, a-mutation-pass-that-fails-before-any-has-been-recorded-loses]
---

## Intent

Two things phase 3 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md` left blocking. The migration of a file `clm init` writes comes to live with that file in `init`, so the build that changes the file writes its migration in its own component. And a mutation pass that fails before any pass has been recorded no longer loses the requirements it was to judge.
