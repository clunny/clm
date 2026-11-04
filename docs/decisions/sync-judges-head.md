---
title: "clm sync loads the settings and runs clm check on HEAD's commit, in a temporary worktree"
date: 2026-10-27
status: accepted
---

`sync-carries-unrelated-edits` let `clm sync` carry an uncommitted edit through the rebase on any branch other than `impl/*`, when the edit is outside `specs/`, `changes/` and `backlog/` and `origin/main` didn't change the file. It named three ways a tracked edit can hurt a sync: the rebase writes over it, sync's own commits sweep it in, or a step that judges `HEAD` reads it in `HEAD`'s place. It guarded the third only for the archive and split-item steps on `impl/*`, and missed the last step: the settings load and `clm check` read the working tree on every branch. So an uncommitted edit to `clm.toml`, to a test file's `req:` marker or to a decision under `docs/decisions/`, and any untracked non-Markdown file the marker scan reads, was judged as if it were `HEAD`'s, and `sync ok` (or a failure) could describe a tree that `clm land` never merges.

`clm sync` now checks `HEAD` out detached into a fresh temporary worktree under the system's temporary directory, loads the settings and runs `clm check` there, and removes it on every exit path, as `clm land` does with its merge commit (`scratch-outside-git`). What sync judges is then exactly what land judges, whatever the working tree holds, and carrying an edit through stays safe on every branch.

Alternatives considered:
- Refusing more edits: a modified `clm.toml`, and any modified file holding a marker. The check reads far more than that: every non-Markdown file for markers (a marker added or removed changes the outcome, so "holding a marker" would have to be judged at both `HEAD` and the working tree), `docs/decisions/`, and untracked files, which the refusal can't cover without stopping every sync over a stray test output. Each new rule in `clm check` that reads a file would widen the list again, and every widening brings back the hand `git stash` that `sync-carries-unrelated-edits` removed.
- Judging in place when the working tree is clean, and in a temporary worktree only otherwise. It saves a checkout, but gives sync two paths whose results can differ through ignored files, which the model reads under `docs/decisions/` and a checkout of `HEAD` doesn't have. One path is simpler to test and to trust; a checkout of `HEAD` writes only the tracked files.
- Keeping the carried edit out of the working tree until the check has run (holding the autostash, or a `git stash create` commit, past the rebase). The edit would then live only in git's internal state while the check runs and, on a crash, in an unreferenced commit; and an untracked file would still be read.
- Loading the model and markers from git objects instead of a checkout. `clm-model` can load an in-memory file set, but the marker scan and the settings read files from disk, and building a second reading path for each is more code than one checkout.
