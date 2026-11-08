---
status: applied
resolves: [scans-of-copies-follow-git-dir-to-the-clone]
---

## Intent

Make every git command CLM runs outside the clone that `GIT_DIR` and `GIT_WORK_TREE` name ignore those variables, in `clm-git` itself (`vcs/git-follows-its-clone`), so a scan of a plain copy walks the copy and `clm rebase`'s and `clm land`'s temporary worktrees are their own checkouts. This resolves `scans-of-copies-follow-git-dir-to-the-clone`.

## Problem

`clm-git`'s `git` and `run` pass the process's environment on. With `GIT_DIR` and `GIT_WORK_TREE` set, git run in a directory outside the work tree they name still works on that clone:

- `clm-coverage`'s `scan(root)` runs `git ls-files --cached --others --exclude-standard` in `root` and walks only when it fails. In the plain copies `rename/replay` and `sync/regenerate-renames` make, the listing succeeds and lists the clone's work tree, files the copy lacks dropped, so a marker only the copy holds is missed. `clm-check`'s file listing (`tracked_files`, then `in_work_tree`) has the same fault.
- `clm rebase`'s temporary worktree (`judged-by-check`) runs `git checkout --detach <rev>` there after a reconsider item is committed, which with the variables set moves the clone's own `HEAD` and work tree, and the check's scans there list the clone's work tree, untracked files included, which `judged-by-check` says never decide the outcome. `clm land`'s temporary worktrees run git there the same way.

No requirement says how `clm-git` treats these variables outside the clone, so the fix is a requirement (`code-fix-needs-a-written-rule`).

## Approaches

1. **`clm-git` drops the variables for a `root` outside the work tree they name (chosen).** One rule in the layer every git command already passes through (`git-plumbing-layer`, `reuse-first`): every directory CLM runs git in outside the clone is one it made for itself, so none is meant to follow them. It covers the sites the item lists, `clm-check`'s listing found while drafting, and sites not yet written. Inside the work tree the environment is honoured as before, so a clone whose work tree has no `.git` still works. See `docs/decisions/git-follows-its-clone.md`.
2. **Fix each site.** `clm-coverage` and `clm-check` would check `rev-parse --show-toplevel` against `root` before listing, `clm-sync` and `clm-land` would route their worktrees' git through `git_isolated`. Rejected: the same question in four crates, and the next site is found only when it misfires.
3. **`clm` removes the variables at start-up.** Rejected: a clone whose work tree has no `.git` is then no repository to CLM at all.

## Outcome

- `vcs`: the Contract states the rule; `git-follows-its-clone` added, with scenarios for a plain copy, a linked worktree and a caller's own `GIT_INDEX_FILE`.
- `sync`: `judged-by-check` gains *a clone the environment names*, which needs `vcs/git-follows-its-clone`: with the variables set, an untracked file in the clone's work tree doesn't decide the outcome and the clone's `HEAD` is untouched.
- `check.coverage`, `check`, `land` and `rename` change no text: their scans and worktrees follow from the layer. `check.coverage`'s Design already says a root outside a git work tree is walked.
- Decision `git-follows-its-clone`.

## Edge cases and workarounds

- A test sets `GIT_DIR` and `GIT_WORK_TREE` on a child process, never on the test process with `std::env::set_var`, since every test thread shares it and the work tree is found once per process. `clm-git`'s scenarios therefore run the layer in a child: the test binary re-run with a filter, or a small helper binary or example in `clm-git`; `sync`'s runs the `clm rebase` binary, as `regenerate-renames`' *a clone the environment names* does.
- `git rev-parse --show-toplevel` with `GIT_WORK_TREE` set prints that directory even when the process's directory lies outside it, and with only `GIT_DIR` set prints the process's directory, as git takes it for the work tree; both are what git itself would act on.
- `root` and the work tree are compared with symbolic links resolved (`/tmp` under `/private` on macOS). A `root` that can't be resolved is outside.
- A variable that names a path relative to the process's directory, kept for a `root` inside the work tree but other than the process's directory, resolves against `root` as it does today; unchanged here.
- `regenerate-renames`' scenario *a clone the environment names* noted that the copy's coverage scan still followed the variables; after this change it walks the copy, and the scenario's outcome doesn't change.
- `git_isolated` stays: `clm-host` runs `git init --bare <path>` with `root` inside the clone.
- A worktree CLM keeps inside the work tree, such as `clm land`'s kept tree `<main worktree>/.clm/land-tree`, lies under it by path, so the rule also counts a `root` as outside when it, or a directory between it and the work tree, holds a `.git` entry; otherwise its `whole(top)` check would read the clone's top level and rebuild it every land.
- `run_bounded` sets `Options::env` and then removes the listed variables; the build removes first, or skips a name the caller set, so a caller's `GIT_INDEX_FILE` survives (*the caller's variables kept outside*).
- The work tree is found again when `GIT_DIR` or `GIT_WORK_TREE` changes, so the existing single-test binaries that call `set_var` (`clm-git`'s `isolated_env.rs`, `clm-host`'s `keep_remote_env.rs`) see the value they set; the lookup runs its `rev-parse` directly, not through the rule, so it doesn't recurse.
- `plugin.hooks.guard`'s push query passes a pushed command's own `GIT_DIR` and `GIT_WORK_TREE` through `Options::env`, which are kept; only the process's own are dropped for a checkout outside its work tree.
- The capability suite's `isolated()` helper removes the variables from every capability test, so no capability test sees a difference.

## Constraints

- `git-plumbing-layer`: every git command already runs through `clm-git`, which is what lets one rule cover them.
- `reuse-first`: `git_isolated`'s removal of the variables git lists is reused, not rewritten.
- `tests-run-concurrently`: no test changes the process's environment.

## Open questions

- With only `GIT_DIR` set, the named work tree is the directory `clm` was started in, as git takes it; whether a hook that sets `GIT_DIR` alone and starts `clm` in a subdirectory needs more is left until one does.
