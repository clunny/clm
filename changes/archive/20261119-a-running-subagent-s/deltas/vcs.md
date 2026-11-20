---
component: vcs
base: { contract: "7242f9bf956a3ac1" }
---

## MODIFIED Contract
The one place CLM's Rust crates start `git`, GitHub's CLI `gh` or the Azure CLI `az` (`git-plumbing-layer`, `reuse-first`). No other crate of the workspace starts one of them, and only the crates that run a program the project names start any process at all; a scan of every crate fails when one does (`one-git-layer`). The panel's TypeScript and the repository's shell scripts are outside it and run `git` themselves.

Crate `clm-git` offers:
- `run(root, program, args, Options { stdin, env })`, the raw form the others are built on: `program` is whatever program the caller names (`git`, `gh`, `az` or any other), resolved on `PATH`, `stdin` optional bytes to feed it, `env` variables to set (`GIT_INDEX_FILE`, `GIT_AUTHOR_DATE`, a `GH_TOKEN`). It returns stdout as bytes, untrimmed (`raw-runs`). `run_within(root, program, args, options, limit)` is `run` with a time limit: a command still running when `limit` runs out is killed and the call returns a timed-out error naming the command and the limit (`bounded-runs`).
- `git(root, args)` and `gh(root, args)`, which run the command in `root` and return its stdout with trailing whitespace trimmed (`run-git`); `git_isolated(dir, args)`, `git` run as though the environment named no repository (`git-isolated`). Every other git command the crate runs is run that way when its `root` lies outside the work tree the calling process's environment names, and with the environment as it is inside it (`git-follows-its-clone`).
- `Error`, what each of these returns when it fails: `Failed { command, status, stderr }` for a command that exited non-zero, `status` its exit status and `stderr` its standard error trimmed; `Spawn { command, source }`, an I/O error with what met it: a program that couldn't be started, waited for or read, a path `extract_paths` refuses or finds nothing for (`extract-paths`), or a temporary directory `extract_tree` or `extract_paths` couldn't make; `TimedOut { command, limit }`; `Unreadable { command, why }` for an answer of git's that can't be read (`tree-blobs`); and `StillOnDisk { path }` (`remove-worktree`). It displays as `<command>: <stderr>`, or `<command>: <exit status>` when stderr is empty, and for the others as their requirements say. Callers match `Failed`'s `status` and `stderr`, and `TimedOut`, by name.
- `fetch(root)`, `git fetch origin` tried again after a failure, returning the last run's error (`fetch-tries-again`); `fetch_retrying(root, args)`, a fetch of `origin` tried again the same way, which returns a `FetchError`: `Offline`, displayed `origin can't be reached`, when `origin` can't be reached, and `Git(error)`, displayed as the error, when git refused the fetch (`fetch-retries`).
- `is_ancestor(root, a, b)`; `current_branch(root)` (`None` on a detached `HEAD`); `worktrees(root)`, each worktree's path, branch, whether it is locked and whether its lock is *held*, by a process its reason names that still runs (`held-lock`), listed again after a failure (`worktrees-tries-again`); `common_dir(root)`, the clone's common git directory as an absolute path, shared by every worktree (`common-dir`).
- `extract_tree(root, rev)`, which checks a revision's tree out into a temporary directory, leaving the working tree and the index alone, and `extract_paths(root, rev, paths)`, which does the same for only the files under `paths` (`repo-queries`, `extract-paths`). Both write each file as `git checkout` of `rev` would write it into this repository's working tree, so hashes read from an extracted tree equal those read from a checkout of the same revision (`extract-as-checkout`). `tree_blobs(root, commit, keep)` reads a commit's files from the local objects in memory, as stored, writing nothing (`tree-blobs`).
- `tracked_files(root)` and `tracked_matching(root, globs)`, the files the index holds under `root`, all of them or those a component's `paths` globs match (`tracked-files`); `files_to_commit(root)`, those and the untracked files git doesn't ignore (`files-to-commit`).
- `remove_worktree(root, path)`, which removes a linked worktree whole, ignored files included, also when a build left a directory in it its owner can't write (`remove-worktree`); whether the worktree is clean enough to remove is the caller's question.
- `REMOTE`, the name of the remote every command fetches from and pushes to (`"origin"`); `now()`, the current time in whole seconds since the Unix epoch; and `Failure`, the error of a command that either refused (`Refused(why)`, exit 1) or failed in git (`Git(message)`, exit 2). `Failure` converts from `Error` with `?` (as `Git`), displays its message alone, and gives its exit code with `exit_code()` (`shared-plumbing-in-git`). A crate whose errors are exactly these two uses `Failure` as its `Error`; one that needs a third kind, or `?` from another crate's error type, keeps its own type.

Every program is resolved on `PATH`, so a caller's test puts a stand-in `gh`, `az` or `git` first on `PATH` and the layer runs it.

## ADDED Requirements

### Requirement: held-lock
`worktrees(root)` SHALL give each worktree whether its lock is *held*: it is held when the worktree is locked, the lock's reason, as `git worktree list --porcelain` gives it, holds `pid`, at its start or after a character that is neither a letter nor a digit, then one space and a run of decimal digits ending at its end or at a character that isn't a digit, the first such in the reason, whose value is above 0 and fits the platform's process ID type, and a process with that ID runs on this machine, which a signal 0 sent to it tells, a refusal for want of permission counting as running. A worktree that isn't locked, whose lock has no reason or a reason that names no process ID so, or whose named process doesn't run, SHALL NOT be held; a value of 0 or one too large for that type names no process, and SHALL never be passed to the signal call. On a platform where CLM can't tell whether a process runs, a lock whose reason names a process ID SHALL be held.

#### Scenario: a running process's lock
- GIVEN a linked worktree `w` locked with the reason `claude agent agent-1 (pid <p> start 1)`, `<p>` the ID of the test's own process
- WHEN `worktrees(root)` runs
- THEN its entry for `w` is locked and held

#### Scenario: an ended process's lock
- GIVEN a linked worktree `w` locked with the reason `claude agent agent-1 (pid <p> start 1)`, `<p>` the ID of a child process the test started and has waited for
- WHEN `worktrees(root)` runs
- THEN its entry for `w` is locked and not held

#### Scenario: another user's process
- GIVEN a linked worktree `w` locked with the reason `claude agent agent-1 (pid 1 start 1)`, on Unix
- WHEN `worktrees(root)` runs
- THEN its entry for `w` is locked and held

#### Scenario: a lock naming no process
- GIVEN linked worktrees locked by `git worktree lock` with no reason, with the reason `kept by hand`, with `rapid 1`, with `pid 0`, with `pid 99999999999999999999` and with `pid x1`
- WHEN `worktrees(root)` runs
- THEN each entry is locked and not held

#### Serves
- capability/work-concurrently
