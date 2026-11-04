---
title: "A draft whose spec branch hand-edits a file outside `specs/`, `changes/` and `backlog/` runs the land check before review"
date: 2026-11-02
status: accepted
---

A `clm draft` session whose spec branch changes a file outside `specs/`, `changes/` and `backlog/` (`ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `README.md`, `docs/`) runs `clm land --check-only` after its last `clm gate` run, before it opens the pull request or stops for review. A failing test that reads a file the branch changes goes under `## Open questions` in `change.md`, naming the test, its component and the hand edit, and the draft doesn't land: a change needing review stops as usual, so the fold carries the failure into the pull request the human reads before approving; a change landing on checks stops with `stopped: land check fails: <tests>`.

**Why.** Code tests read those files: this repository's root tests read `ARCHITECTURE.md`'s heading form and scan every file CLM works from for retired terms. `clm check` and the gate don't run tests, so a draft that hand-edits one, as a contract change's Outcome may ask, reported its change ready for review with a test it broke. The breakage surfaced only at `clm land`, after the human's approval, and the fix needed its own code branch first. Running the land check while drafting finds it before the human is asked. `clm land --check-only` is the land's own check (`land-check-command`), so nothing new is built (`reuse-first`).

**Why only hand edits.** The check compiles the workspace into the draft's own `target/` and runs every suite, which costs minutes and gigabytes per draft. Every spec branch changes `specs/`, `changes/` or `backlog/`, while hand edits outside them are rare. The generated `specs/` text and a change's own files are accepted as unchecked here: they reach code tests only through text scans such as the retired-terms test, and `light-ci-for-doc-prs` already accepts that a doc-only change can break a test the next code land then catches.

**Why the draft doesn't fix the test.** A spec branch carries no code (the gate classifies a branch with code `mixed`), and a `clm claim --fix` takes only an item already on `main`. The fix is a code branch that must land before the change, which the human arranges once the failure is in front of them; the draft's job is to put it there before the approval, not after.

**The fold doesn't run it.** A fold merges drafts that each ran the check when they hand-edited, changes only spec text and runs no cargo; a conflict merged in a hand-edited file is rare enough to leave to `clm land`.

**Rejected.**
- *A gate warning for hand edits.* The gate can't tell which files a test reads without running the tests, so it would warn on every hand edit and still not say whether anything broke.
- *Taking `ARCHITECTURE.md`, `CAPABILITIES.md` and `CLAUDE.md` off the ledger's doc-only list.* A land would then run the check on them, which is still after the approval the item is about.
- *Running the check on every draft.* It costs a full build per draft for a failure that needs a hand edit outside the generated text.
