---
component: vcs
base: { contract: "3906887304c2f34a" }
---

## MODIFIED Contract

The one place CLM's Rust crates start `git`, GitHub's CLI `gh` or the Azure CLI `az` (`git-plumbing-layer`, `reuse-first`). No other crate of the workspace starts one of them, and only the crates that run a program the project names start any process at all; a scan of every crate fails when one does (`one-git-layer`). The panel's TypeScript and the repository's shell scripts are outside it and run `git` themselves.

Crate `clm-git` offers:
- `run(root, program, args, Options { stdin, env })`, the raw form the others are built on: `program` is whatever program the caller names (`git`, `gh`, `az` or any other), resolved on `PATH`, `stdin` optional bytes to feed it, `env` variables to set (`GIT_INDEX_FILE`, `GIT_AUTHOR_DATE`, a `GH_TOKEN`). It returns stdout as bytes, untrimmed (`raw-runs`). `run_within(root, program, args, options, limit)` is `run` with a time limit: a command still running when `limit` runs out is killed and the call returns a timed-out error naming the command and the limit (`bounded-runs`).
- `git(root, args)` and `gh(root, args)`, which run the command in `root` and return its stdout with trailing whitespace trimmed (`run-git`); `git_isolated(dir, args)`, `git` run as though the environment named no repository (`git-isolated`). Every other git command the crate runs is run that way when its `root` lies outside the work tree the calling process's environment names, and with the environment as it is inside it (`git-follows-its-clone`).
- `Error`, what each of these returns when it fails: `Failed { command, status, stderr }` for a command that exited non-zero, `status` its exit status and `stderr` its standard error trimmed; `Spawn { command, source }`, an I/O error with what met it: a program that couldn't be started, waited for or read, a path `extract_paths` refuses or finds nothing for (`extract-paths`), or a temporary directory `extract_tree` or `extract_paths` couldn't make; `TimedOut { command, limit }`; `Unreadable { command, why }` for an answer of git's that can't be read (`tree-blobs`); and `StillOnDisk { path }` (`remove-worktree`). It displays as `<command>: <stderr>`, or `<command>: <exit status>` when stderr is empty, and for the others as their requirements say. Callers match `Failed`'s `status` and `stderr`, and `TimedOut`, by name.
- `fetch(root)`, `git fetch origin` tried again after a failure, returning the last run's error (`fetch-tries-again`); `fetch_retrying(root, args)`, a fetch of `origin` tried again the same way, which returns a `FetchError`: `Offline`, displayed `origin can't be reached`, when `origin` can't be reached, and `Git(error)`, displayed as the error, when git refused the fetch or couldn't be started in `root`, the error then naming `root` (`fetch-retries`).
- `is_ancestor(root, a, b)`; `current_branch(root)` (`None` on a detached `HEAD`); `worktrees(root)`, each worktree's path, branch, whether it is locked and whether its lock is *held*, by a process its reason names that still runs (`held-lock`), listed again after a failure (`worktrees-tries-again`); `common_dir(root)`, the clone's common git directory as an absolute path, shared by every worktree (`common-dir`).
- `extract_tree(root, rev)`, which checks a revision's tree out into a temporary directory, leaving the working tree and the index alone, and `extract_paths(root, rev, paths)`, which does the same for only the files under `paths` (`repo-queries`, `extract-paths`). Both write each file as `git checkout` of `rev` would write it into this repository's working tree, so hashes read from an extracted tree equal those read from a checkout of the same revision (`extract-as-checkout`). `tree_blobs(root, commit, keep)` reads a commit's files from the local objects in memory, as stored, writing nothing (`tree-blobs`).
- `tracked_files(root)` and `tracked_matching(root, globs)`, the files the index holds under `root`, all of them or those a component's `paths` globs match (`tracked-files`); `files_to_commit(root)`, those and the untracked files git doesn't ignore (`files-to-commit`).
- `remove_worktree(root, path)`, which removes a linked worktree whole, ignored files included, also when a build left a directory in it its owner can't write (`remove-worktree`); whether the worktree is clean enough to remove is the caller's question.
- `REMOTE`, the name of the remote every command fetches from and pushes to (`"origin"`); `now()`, the current time in whole seconds since the Unix epoch; and `Failure`, the error of a command that either refused (`Refused(why)`, exit 1) or failed in git (`Git(message)`, exit 2). `Failure` converts from `Error` with `?` (as `Git`), displays its message alone, and gives its exit code with `exit_code()` (`shared-plumbing-in-git`). A crate whose errors are exactly these two uses `Failure` as its `Error`; one that needs a third kind, or `?` from another crate's error type, keeps its own type.

Every program is resolved on `PATH`, so a caller's test puts a stand-in `gh`, `az` or `git` first on `PATH` and the layer runs it.

## MODIFIED Requirements

### Requirement: fetch-retries <!-- base: 2629af1f5e3a9389 -->
`clm-git` SHALL offer `fetch_retrying(root, args)`, which runs `git` with `args`, a fetch naming `REMOTE`, in `root`, and when it fails runs it again, at most twice more, waiting 1 second before the first of these and 2 seconds before the second. It SHALL return success at the first run that succeeds. When a run returns `Error::Spawn`, as a run of `git` that can't be started in `root` does (a `root` that doesn't exist or isn't a directory), it SHALL run `git` no more, run no `git ls-remote`, and return `Git` holding an `Error::Spawn` whose display is `<command> in <root>: <the I/O error>`, `<root>` the path as given, so a caller names the directory it was given and never says `origin` can't be reached when `git` never ran. Otherwise, when the last run fails, it SHALL run `git ls-remote --quiet <REMOTE> refs/heads/main` in `root` and return a `FetchError`: `Offline`, whose display is `origin can't be reached`, when that fails too, and otherwise `Git` holding the last run's error, whose display is that error's, so a fetch that lost a race with another process of the same clone over a remote-tracking ref (git's `cannot lock ref`) goes on, and a caller says it is offline only when `origin` can't be reached.

#### Scenario: a fetch that loses a ref race
- GIVEN a clone whose `origin` is a bare repository, and an executable `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/main'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `fetch_retrying(root, &["fetch", REMOTE])` runs
- THEN it returns success after at least 1 second, and the fake ran `fetch` twice

#### Scenario: a fetch that keeps failing with origin reachable
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `fetch_retrying(root, &["fetch", REMOTE])` runs
- THEN it returns `FetchError::Git` whose display holds `cannot lock ref`, after at least 3 seconds, and the fake ran `fetch` three times and `ls-remote` once

#### Scenario: origin can't be reached
- GIVEN a clone whose `origin` URL names a directory that doesn't exist
- WHEN `fetch_retrying(root, &["fetch", REMOTE])` runs
- THEN it returns `FetchError::Offline`

#### Scenario: what being offline says
- GIVEN the same clone
- WHEN the error `fetch_retrying(root, &["fetch", REMOTE])` returns is displayed
- THEN it reads `origin can't be reached`

#### Scenario: a root that doesn't exist
- GIVEN a path `x` that doesn't exist
- WHEN `fetch_retrying(x, &["fetch", REMOTE])` runs
- THEN it returns `FetchError::Git` within 1 second, whose display starts `git fetch origin in ` and holds `x`'s path, and doesn't hold `can't be reached`

#### Serves
- capability/work-concurrently
