---
status: done
resolves: [clm-land-leaves-its-tmp-land-bin-copy-behind]
---

## Intent

Accept the behaviour the bug `clm-land-leaves-its-tmp-land-bin-copy-behind` reports, delete the item, and design nothing. No spec, code or skill text changes.

The item reports `land-bin-*` directories left under `/tmp` after lands ended, one two days old, each holding about 20 MB, and guesses that a failing land, such as one on a full disk, leaves them. It asks that the land remove its binary copy on every exit path.

It already does, on every exit path the process has. `land/land-binary-copy` says the copy is removed on every exit path, and its scenario `copy removed` proves it. The copy lives in a `tempfile::TempDir` held by `Exe` (`crates/clm-land/src/exe.rs`), which `land` and `--check-only` hold until they return; every failure is an `Err` returned up to `main`, which returns an `ExitCode` and never calls `std::process::exit`, and the workspace sets no `panic = "abort"`, so the directory is removed on success, on failure and on a panic alike. A copy that fails for a full disk fails inside `Exe::new`, which drops the half-made directory as it returns the error.

What leaves a copy is a land that never reaches an exit: one killed by a signal. The directory the item found, `land-bin-VXf2zW` of 2026-11-13, holds the whole 21.8 MB binary, so the copy was finished and the land was killed later, on a day sessions were stopping lands that waited on a loaded machine. `land`'s Design already accepts this for the land's other temporary directory: a `land-*` worktree is "removed on every exit path" and "a killed land's tree stays until the system clears it". The binary copy is the same case and gets the same answer.

The behaviour is accepted rather than changed because:

- **A killed process removes nothing.** `SIGKILL` can't be caught, so no handler makes the promise whole; only a later land sweeping the copies no live land holds could, which needs a new requirement, a lock per copy (as the queue's tickets have) and its tests, for 20 MB per killed land under a directory the system owns.
- **The freeze is on.** The plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md` gives a defect the default answer "accept" until it is done, and an exception is the human's to make.

## Approaches

- **Taken: accept, and delete the item.** Every exit path removes the copy; a killed land's copy stays until the system clears its temporary directory, as a killed land's worktree does.
- **Not taken: catch `SIGINT`, `SIGTERM` and `SIGHUP` and remove the copy.** It adds a signal dependency and handler for part of the case, and still leaves the `SIGKILL` copies.
- **Not taken: sweep stale `land-bin-*` directories at the start of each land.** Telling a stale copy from a live land's needs a lock held on each copy for the land's life, as the lock queue's tickets do; it is a new requirement of `land`, outside the freeze, and saves only what a killed land leaves.

## Edge cases and workarounds

None.
