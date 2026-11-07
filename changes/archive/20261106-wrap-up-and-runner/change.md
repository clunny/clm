---
status: done
resolves: [wrap-up-and-runner-trip-on-unwritable-worktree-dirs]
---

# clm wrap-up and the runner remove a build worktree whole

## Intent

`clm wrap-up` and `clm runner` remove a build's worktree whole, as `clm run --finish` already does, even when the build left a directory in it without its owner's write permission: all three remove a worktree through one function, `vcs`'s new `remove_worktree`, which unlocks the worktree, gives the owner the permissions removal needs on every directory under it without following a symbolic link, and runs `git worktree remove`. When a worktree still can't be removed, each prints `left <path>: <error>; delete the directory by hand, then run git worktree prune` and goes on instead of failing: `clm wrap-up` keeps that worktree's local branch and doesn't count the line against the wrap-up, and the runner starts no session for the component and runs `clm run --finish <component>`, as it does for a dirty leftover. Resolves `backlog/wrap-up-and-runner-trip-on-unwritable-worktree-dirs`.

## Problem

`20261106-run-finish-fails-on` fixed `clm run --finish`, and filed the same failure in the two other commands that remove build worktrees. `clm wrap-up` (`orchestrator.checkpoint`) removes each landed `agent-*` worktree under `.claude/worktrees/` with a bare `git worktree remove`, and the runner (`orchestrator.runner`) removes its clean `agent-runner-<component>` leftover with `git worktree remove --force` before adding a new one. When a build left an ignored directory its owner can't write (an unpacked archive, say), `git status` still reads the worktree as clean, git unregisters it and then fails to delete those files: `clm wrap-up` exits 2 with its report half printed, and the runner prints `git failed: ...` and leaves the directory on disk with nothing saying what to do. Neither component's rules say what they do with such a directory, and the code that handles it lives privately in `clm-run`, which neither depends on.

## Approaches

- **Move `clm-run`'s removal into `clm-git` as `remove_worktree` and have all three commands call it (chosen).** `clm-git` is the one crate all three already depend on, a worktree is git's, and the function is a git call plus the permission walk that makes it succeed (`reuse-first`). `clm-run`'s private copy goes when `orchestrator.run` moves to it (`no-dead-code`). Each command keeps its own `left` line, since the words, and what happens next, are the command's.
- **Copy `make_writable` into `clm-checkpoint` and `clm-runner`.** Three copies of a permission walk that must never follow a symbolic link. Rejected (`reuse-first`).
- **Put it in `orchestrator` (`clm-orchestrator`).** `clm-runner` deliberately depends on `clm-git` alone and drives the other commands as subprocesses; adding `clm-orchestrator` for one function would widen it. Rejected.
- **`git worktree remove --force` alone.** Force overrides git's dirty and locked checks, not a directory its owner can't write: the runner already passes it and still fails. Rejected.

## Outcome

- `vcs` adds `remove-worktree`: `remove_worktree(root, path)`, unlock, the permission walk, `git worktree remove` without `--force`, and `Error::StillOnDisk` when git succeeded but the directory remains. Its Contract names the function and its Design takes the walk's description from `orchestrator.run`'s.
- `orchestrator.run/run-finish` removes through `remove_worktree` and holds no removal of its own; its behaviour and scenarios are unchanged. Its build moves `clm-run` onto the function and deletes `remove_worktree` and `make_writable` there.
- `orchestrator.checkpoint/checkpoint` removes through `remove_worktree` and prints the `left` line for a worktree it can't remove, keeping that worktree's local branch, with two scenarios: *unwritable directory in a build's worktree* and *build's worktree git can't remove*. A `left` line doesn't make the wrap-up incomplete: a build's worktree is never the session's own, and its branch has landed.
- `plugin` adds `wrap-up-left-lines`: every skill that runs `clm wrap-up` tells Claude to pass its `left <path>: ...` lines to the human as they stand and leave the directory to them, as `build-finishes-builds` already does for `clm run --finish` (`method-in-the-plugin`).
- `orchestrator.runner/runner-worktree-leftover` removes through `remove_worktree` and, when that fails, starts no session, prints the `left` line with the absolute path and runs `clm run --finish <component>`, with two scenarios: *unwritable leftover replaced* and *leftover git can't remove*. The runner no longer passes `--force`: the leftover was already judged clean, and `remove_worktree` unlocks it.
- Each of the three modified requirements has `#### Needs` `vcs/remove-worktree`; `plugin/wrap-up-left-lines` needs `orchestrator.checkpoint/checkpoint`.
- No capability test sees a difference: `wrap_up_session`, `unattended_runner` and `build_from_spec` remove only ordinary worktrees.

## Constraints

- Permissions change only under the worktree being removed, after its caller judged it clean, and never through a symbolic link.
- A worktree's removal is the only failure each command reports and goes past; every other git failure keeps its exit code.
- `clm-git` keeps no dependency on the rest of the workspace and stays out of the wasm builds.

## Edge cases

- The *git can't remove* scenarios need a worktree git deterministically can't delete without running as root: `orchestrator.run`'s test makes the worktree's parent directory unwritable for the run, and the same works for `.claude/worktrees/` in `clm wrap-up`'s and the runner's tests, and for `vcs`'s *git refuses* scenario an untracked file that isn't ignored does it.
- The unwritable-directory and symbolic-link scenarios are Unix tests (permission bits). CLM supports Linux and macOS only (`no-windows-support`), so `remove-worktree` states no Windows behaviour; `orchestrator.run`'s build drops `clm-run`'s non-Unix branch of the walk when it moves to the function.
- After a failed removal, what is left of the directory may hold a `.git` file pointing at a gitdir git has deleted, where `git status` fails: `clm wrap-up` skips a path it printed as `left` when it reports dirty work, so that failure can't exit 2.
- Dropping the runner's `--force` means a leftover holding an initialised submodule, which git refuses without it, is now reported as `left` rather than removed. Runner worktrees are detached at `origin/main`, and a clean one holds only what a build put in ignored paths; a human clears the rare case by hand, as the line says.

## Open questions

- None. The architecture audit's two findings (no skill told sessions what to do with `clm wrap-up`'s `left` line; a Windows-only clause against `no-windows-support`) are fixed in the change.
