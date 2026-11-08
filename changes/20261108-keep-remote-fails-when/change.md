---
status: applied
resolves: [keep-remote-fails-when-git-work-tree-is-set]
---

## Intent

Make `keep_remote` (`host/none-keeps-a-remote`) work in a clone the environment names with `GIT_DIR` and `GIT_WORK_TREE`, a work tree whose git directory is elsewhere, so `clm init` on `host = "none"` succeeds there. `clm-git` gains `git_isolated(dir, args)` (`vcs/git-isolated`), which runs git with the variables `git rev-parse --local-env-vars` lists removed, and `keep_remote` makes its bare repository through it. This resolves `keep-remote-fails-when-git-work-tree-is-set`.

## Problem

`keep_remote` runs `git init --quiet --bare <common>/clm/remote.git` through `clm_git::git`, which passes the process's environment on. With `GIT_DIR` and `GIT_WORK_TREE` set, git refuses it: `git init --bare <path>` drops `GIT_DIR` for the new repository but still reads `GIT_WORK_TREE`, and fails with `GIT_WORK_TREE (or --work-tree=<directory>) not allowed without specifying GIT_DIR`. Checked by hand: `GIT_DIR=<new>`, `--git-dir=<new>`, `--work-tree=""` and an empty `GIT_WORK_TREE` are all refused the same way, so only removing the variables works. `clm-git`'s `Options` can set variables but not remove them, and `clm-host` may run no git itself (`host/git-through-layer`), so the fix needs `clm-git`.

`host/none-keeps-a-remote` as written says `keep_remote` makes the bare repository but nothing about the environment, so the case becomes a scenario (`code-fix-needs-a-written-rule`), and the `clm-git` function it needs becomes a `vcs` requirement (`dependency-work-is-a-requirement`).

## Approaches

1. **A function beside `git` that removes git's local variables (chosen).** `git_isolated(dir, args)`; the list comes from `git rev-parse --local-env-vars`, git's own list of the variables that bind a command to a repository, the one git itself clears before running a command in a submodule (`reuse-first`). It is additive: no caller changes.
2. **An `env_remove` field on `Options`.** Rejected: `Options` is built by struct literal in other crates (the reason `run_within` is a function, `vcs` Design), so a new field breaks every one of them for one caller.
3. **A function named for the job, `init_bare(path)`.** Rejected: the same fault sits in other scratch repositories (below), which run `init`, `add`, `fetch` and `checkout`, so a general form serves them all.
4. **Accept it.** Rejected: `GIT_DIR` is set in git hooks and by tools that keep the git directory apart from the work tree, and `clm init` is a first-run command that should not fail there.

## Outcome

- `vcs`: the Contract names `git_isolated`; `vcs/git-isolated` is added.
- `host`: `host/none-keeps-a-remote` makes the bare repository through `git_isolated`, gains a scenario for a clone the environment names, and needs `vcs/git-isolated`; `host/git-through-layer` lists `git_isolated` among the `clm-git` functions `clm-host` runs git through (from the architecture audit). Reading the remotes, `common_dir` and `git remote add` keep running with the variables, since they work on the clone the variables name.
- `backlog/scratch-repositories-follow-git-dir-to-the-clone.md` (bug, `sync` and `orchestrator.run`): `clm-sync`'s rename replay and `clm-run`'s superpowers fetch make a repository of their own in a temporary directory through `git` and, with `GIT_DIR` set, would reinitialize, stage in, fetch into or check out the clone instead. Out of this change's components, so filed (`fold-small-defer-large`).

## Edge cases and workarounds

- A test that sets `GIT_DIR` and `GIT_WORK_TREE` must not set them in the test process with `std::env::set_var`, since the environment is shared by every test thread: the scenarios run the code in a child process that has them set (a test binary re-running itself with the variables, or a small helper), as the build chooses.
- The root's `clm init` test `search_stops_at_the_top_without_a_git_entry` passes `--host git` to step around this fault. It stays valid as it is; moving it back to `host = "none"` is optional and outside these components.

## Constraints

- `git-plumbing-layer`, `host/git-through-layer`: git runs only through `clm-git`.
- `reuse-first`: the variables are the ones git lists, not a list kept by hand.
- `expand-contract`: the change only adds to `clm-git`'s Contract.

## Open questions

None.
