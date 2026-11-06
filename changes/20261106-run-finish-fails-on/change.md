---
status: applied
resolves: [run-finish-fails-on-unwritable-worktree-dirs]
---

# clm run --finish removes a landed build's worktree whole

## Intent

`clm run --finish <component>` removes a landed build's worktree under `.claude/worktrees/` whole on its first run, ignored files included, even when the build left a directory in it without its owner's write permission: once the worktree is judged clean, it gives the owner the permissions removal needs on everything under the worktree, never following a symbolic link, and then runs `git worktree remove`. When a worktree still can't be removed, `--finish` prints `left <path>: <error>; delete the directory by hand, then run git worktree prune`, goes on, and still prints its `freed` and `landed` lines and exits 0, instead of dropping them and exiting 2. Resolves issue #205 (`backlog/run-finish-fails-on-unwritable-worktree-dirs`).

## Problem

A build can leave an ignored directory without its owner's write bit, for instance by unpacking an archive. `git status` still reads the worktree as clean, so `--finish` runs `git worktree remove`, which unregisters the worktree and then fails to delete those files. `finish` has already freed the place, but the `?` on that failure drops the `freed` and `landed` lines it had gathered and exits 2, so `build` reads no `landed` line for the build. On the next run the worktree is no longer listed, so `finish` prints `landed` and says nothing of the directory still on disk, which a person has to find, make writable and delete by hand. `run-finish` states neither what it removes nor what it does when removal fails.

## Approaches

- **Make the worktree removable, then let git remove it; report what's left (chosen).** The worktree is about to be deleted, so changing permissions inside it costs nothing, and git stays the one that removes a worktree (`reuse-first`). The walk uses `std::fs` and never follows a symbolic link, so nothing outside the worktree changes. What still fails (another user's file, a file in use on Windows, a worktree git refuses) is reported on its own line with the steps that clear it, which work whether or not git already unregistered the worktree, and doesn't cost the other lines.
- **`git worktree remove --force`.** Force only overrides git's dirty and locked checks; it doesn't delete a file in a directory the owner can't write. Rejected: it doesn't fix the failure, and it would remove a dirty worktree, which `run-finish` keeps.
- **Delete the directory with `std::fs::remove_dir_all` after git fails, then `git worktree prune`.** Same permission failure, and a second removal path beside git's. Rejected.
- **Only report the leftover, don't fix permissions.** Meets half the item: a person still has to clear every such worktree by hand, though `--finish` can do it safely. Rejected.

## Outcome

- `orchestrator.run`'s Design bullet *Finishing a build* says how `--finish` makes a clean worktree removable and what it prints when removal still fails.
- `orchestrator.run/run-finish` adds both behaviours and two scenarios: *unwritable directory removed* and *worktree git can't remove*. A `left` line takes the place of `removed`, with git's error on one line.
- `plugin/build-finishes-builds` has the `build` skill pass each `left <path>: ...` line to the human and go on, never running `git worktree remove` or `git worktree prune` itself (`method-in-the-plugin`); it already needs `orchestrator.run/run-finish`. No capability test changes: `build_from_spec` finishes only clean worktrees and asserts success, which still holds.
- `backlog/wrap-up-and-runner-trip-on-unwritable-worktree-dirs` records the same failure in `clm wrap-up` (`orchestrator.checkpoint`) and the runner (`orchestrator.runner`), which remove build worktrees too (`fold-small-defer-large`).

## Constraints

- Permissions change only under the worktree being removed, after it is judged clean, and never through a symbolic link.
- A removal failure is the only git failure `--finish` reports and goes past; a failed fetch still exits 2.
- The unwritable-directory scenario is a Unix test (directory permission bits); on Windows the walk clears read-only attributes, which the scenario doesn't cover.

## Edge cases

- The scenario *worktree git can't remove* needs a worktree git deterministically can't remove without running as root; a worktree holding an initialised submodule, which `git worktree remove` refuses without `--force`, is one way to build it.

## Open questions

- `clm draft --finish` (`orchestrator.triage`) runs `git worktree remove` too, but it already reports a failure as `kept <path>: <error>` and goes on, and drafts don't unpack archives, so it is left as it is (`yagni`). `clm wrap-up` and the runner, which remove build worktrees, are filed as a backlog item rather than designed here, to keep this change to the item it resolves.
