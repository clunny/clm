---
component: rename
---

## ADDED Requirements

### Requirement: undo-on-failure
When a step of `clm rename` fails after it has begun writing, up to and including the archive (a write or a removal, the apply, a refusal, git, or the archive itself), it SHALL put every file it wrote or removed back as it was before the run, those its apply wrote and removed and the `status: done` line `finish` writes among them, and exit as that failure exits (`refusals`, `finish`), so the tree is as the run found it. It SHALL perform its own files and the apply's through one `apply/journal`, the apply's through `apply_into`, and undo that journal once. A file it can't put back SHALL be named on standard error on a line of its own holding `not restored`.

#### Needs
- apply/journal

#### Scenario: a write after the apply fails
- GIVEN change `r`, `proposed`, renaming `a/x` to `a/y`, a test file citing `a/x` that can't be written, and `backlog/q.md` with `kind: question` that `r`'s `resolves` names
- WHEN `clm rename r` runs
- THEN it exits 1, `specs/a/SPEC.md` and `changes/r/change.md` hold what they held before, `changes/r/deltas/` holds what it held before, and `backlog/q.md` still exists
