---
status: done
resolves: [scratch-repositories-follow-git-dir-to-the-clone]
---

## Intent

Make the two repositories CLM makes of its own in a temporary directory, the copy `clm rebase` replays a rename in (`sync/regenerate-renames`) and the superpowers fetch (`orchestrator.run/superpowers-path`), follow no `GIT_DIR` or `GIT_WORK_TREE` the environment sets. The rename copy stops being a repository at all, as `rename/replay`'s copy already isn't; the superpowers fetch runs its git through `clm-git`'s `git_isolated` (`vcs/git-isolated`). This resolves `scratch-repositories-follow-git-dir-to-the-clone`.

## Problem

`clm-git`'s `git` passes the process's environment on. With `GIT_DIR` and `GIT_WORK_TREE` set, as in a git hook or a work tree whose git directory is elsewhere, git run in any directory works on the repository those variables name:

- `clm-sync`'s `replayed` (`crates/clm-sync/src/renames.rs`) extracts a revision into a temporary directory and runs `git init -q` and `git add -A` there, so `clm rename`'s report can list the files git tracks. With the variables set, `git init` reinitializes the clone's git directory (writing `core.worktree` into its config when the work tree isn't its parent) and `git add -A` stages the clone's work tree into the clone's index.
- `clm-run`'s `superpowers_dir` (`crates/clm-run/src/lib.rs`) runs `git init -q`, `git fetch --depth 1 <url> <commit>` and `git checkout -q FETCH_HEAD` in a temporary directory. With the variables set, it reinitializes the clone, fetches into it and checks the pin out over the clone's work tree.

Neither requirement as written says anything about the environment, so each gains a sentence and a scenario (`code-fix-needs-a-written-rule`).

## Approaches

For the rename copy:

1. **No repository (chosen).** `clm rename` already reads a root without `.git` by walking it (its `text_files`), which is how `rename/replay` reads the copy the gate checks. Dropping `git init` and `git add -A` makes `clm rebase` produce the tree exactly as the replay it trusts produces it, runs no git in the copy, and removes code (`yagni`, `reuse-first`). The copy is then read by the same walk as the replay's, so the two agree.
2. **`git_isolated` for `init` and `add`.** Rejected: it keeps a repository whose only use is a report line `clm rebase` discards, and keeps the copy read differently from the replay's.

For the superpowers fetch:

1. **`git_isolated` for `init`, `fetch` and `checkout` (chosen).** The fetch needs a repository; `vcs/git-isolated` exists for exactly this (`reuse-first`).
2. **`git clone`.** Rejected: a clone can't take a bare commit ID, and still needs the variables removed.

Across CLM:

- **`clm-git`'s `git` drops the variables whenever `root` lies outside the work tree they name.** Not taken here: it changes `vcs`'s Contract for every caller and needs a rule for `GIT_DIR` set without `GIT_WORK_TREE` (as in hooks, where it is often relative). It is named in the new item below for the human to weigh.

## Outcome

- `sync`: `sync/regenerate-renames` makes its trees in a directory that is no repository and runs no git there, with a scenario for a clone the environment names. The `.git` skip in `tree_ids` goes with it (`no-dead-code`).
- `orchestrator.run`: `orchestrator.run/superpowers-path` fetches through `git_isolated`, with a scenario for a clone the environment names, and needs `vcs/git-isolated`.
- `backlog/scans-of-copies-follow-git-dir-to-the-clone.md` (bug, `check.coverage` and `sync`): `clm-coverage`'s `scan` runs `git ls-files` in the copies `rename` scans, and `clm rebase`'s step 8 runs git in its temporary worktree, both following the variables to the clone. Out of this change's scope (`fold-small-defer-large`).

## Edge cases and workarounds

- A test sets `GIT_DIR` and `GIT_WORK_TREE` on the child process it runs (`clm rebase`, `clm superpowers`), never on the test process with `std::env::set_var`, since every test thread shares it. The sync scenario therefore runs the `clm rebase` binary (a test in `crates/clm/tests`, or one in `clm-sync` that runs a child), not `sync()` in process, on a fixture whose git directory sits outside its work tree.
- `rename`'s walk of a copy runs no git, but `clm-coverage`'s `scan`, which `rename` calls, still runs `git ls-files` there; so the copy holds no repository and `clm-sync` runs no git there itself, while the scan's git is the new item's.
- Files `git add -A` would have left out can't occur, since `extract_tree` writes only tracked files; the one difference left is that the walk skips any directory named `.git` or `target`, as `rename/replay`'s does, so `clm rebase` and the gate now read the copy alike.
- The superpowers scenario can't fetch the real pin without the network, so it uses the failing fetch `fetch_fails` uses (`GIT_ALLOW_PROTOCOL=file`) and checks the clone's git directory is untouched: under the fault, `git init` rewrites its config. The fixture puts `g` outside `w` so the reinitialization writes `core.worktree` and shows.
- In the sync scenario, `clm-coverage`'s scan of the copy still follows the variables (the new item). With an ID-only rename the versions don't move, so its outcome doesn't depend on that scan.

## Constraints

- `git-plumbing-layer`: git runs only through `clm-git`.
- `reuse-first`: `git_isolated` and `rename`'s walk of a non-repository, both already there.
- `no-dead-code`: the scratch repository's code and the `.git` skip it needed go.

## Open questions

- Whether `clm-git`'s `git` should drop the variables itself for any `root` outside the work tree they name, instead of fixing each site; see `backlog/scans-of-copies-follow-git-dir-to-the-clone.md`.
