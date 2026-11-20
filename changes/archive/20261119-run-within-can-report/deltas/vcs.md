---
component: vcs
base: { design: "c9eed0b593860ca3" }
---

## MODIFIED Design
Thin wrappers over `std::process::Command`, with no git library. Callers map `Error` into their own errors, `Failure` or a type of their own; none matches every variant by name, so a new variant breaks none of them. The crate has no dependency on the rest of the workspace, so every component can depend on it, and it stays out of the wasm builds (`web.worker` never shells out), which is why a crate that builds for wasm (`orchestrator`) uses `REMOTE` and `now()` only behind the feature that brings in `clm-git`.

`REMOTE` is a `pub const REMOTE: &str`, and `fetch` passes it rather than spelling the name. `now()` reads `SystemTime::now()`; callers that need finer time, such as `orchestrator`'s claim IDs, or a formatted date, such as `ledger`'s records, keep their own. `Failure` implements `std::error::Error`.

`run` and `run_within` share one body that takes an optional limit; `run` passes none and waits. With a limit, stdout and stderr are each drained on a detached thread (not a scoped one) and stdin written on a third, from an owned copy of the bytes since the thread may outlive the call, and the caller polls `try_wait` every few milliseconds until the command exits or the limit passes. When the limit passes it calls `kill` and `wait` on the child, so it is reaped, without joining the readers: a process the command started, such as a credential helper, may still hold the pipes open, and the readers end when it closes them. The readers send what they read as it comes, a chunk at a time, so the call can take a stream's bytes without its end. A command that exits in time may also leave a process holding its pipes, as an `ssh` control master that a `git` command over SSH starts with `ControlPersist` holds its stderr: the call then waits for the streams to close for at most 100 ms after the command exits, which is time enough for the bytes the command left in a pipe's buffer, and then returns the command's own outcome with the chunks received by then. It doesn't wait until the limit: both callers pass what remains of a budget shared by several commands (the guard's push check, its release lookup), so a command that exited at once and waited out the limit would leave the commands after it no time and turn them into timeouts. A read error and a stream that sent nothing map as they do when the stream closes, to `Error::Spawn` and to no bytes. A timed-out error there would report a command that did its work as one that ran out of time, which the guard's push check reads as a question it couldn't answer. Only the child is killed, not a process group, since `git` and `gh` are run directly and the portable `std` API kills one process. A limit is a `Duration`, not a deadline, so callers sharing a budget pass what remains of it. `run_within` is a function beside `run` rather than a `timeout` field of `Options`, since `Options` is built by struct literal in other crates (`bounded-runs`).

**The scan** (`one-git-layer`) lists the crates from the directories under `crates/` that hold a `Cargo.toml`, reads each one's `build.rs` and the `.rs` files under its `src/`, removes the whitespace of each file and looks for the strings the requirement names, keeping for each character the line it came from so a match is reported where it starts. It is one check here rather than one in each component, since a crate that didn't hold its own copy was read by nothing.

**Isolation.** The variables removed are the ones `git rev-parse --local-env-vars` prints, not a list written in the code, so a variable a later git adds is removed too. The work tree the environment names is found by running `git rev-parse --show-toplevel` and kept, with the two values it was found for, until either value changes; one answer is kept, not one for each pair, and a change of the process's current directory alone doesn't find it again.

`extract_tree` and `extract_paths` both read the whole of `rev` into a temporary index (`git read-tree <rev>` with `GIT_INDEX_FILE` set), so checkout reads `rev`'s `.gitattributes` from that index even when the paths asked for don't include them, then check entries out with `checkout-index` and the temporary directory as the work tree: `extract_tree` every entry (`--all`), `extract_paths` only the entries under `paths`, listed from the temporary index (`ls-files -z --full-name` with literal, top-relative pathspecs) and fed to `checkout-index --stdin -z`. Not `git checkout <rev> -- <paths>`: it runs the repository's `post-checkout` hook, and `checkout-index` runs none. `ls-files` matching nothing is not a git error, so `extract_paths` compares the listing with the paths asked for itself.

**Tracked files.** `tracked_files` runs `git ls-files -z --cached --deduplicate` in `root`, and `tracked_matching` the same with `--` and one pathspec `:(glob)<glob>` per glob, so a component's `paths` (`**`, `*`, `?` as in its frontmatter) match by git's own glob rules; with no glob it runs nothing, since a component with no `paths` owns no file, not every file. `--deduplicate` lists a file the index holds at several merge stages, during a conflict, once, and needs git 2.31 or later. On Unix the path is built from git's bytes with `OsStr::from_bytes`, so a caller can hand it back to git (`check-attr --stdin -z`); elsewhere it is converted lossily, which CLM accepts since it runs on Linux and macOS. `files_to_commit` sorts what git lists, which names the index's files and then the untracked ones.

**Reading a commit** (`tree-blobs`). `tree_blobs` runs `git ls-tree -r -z --full-tree <commit>` and then, with the IDs of the blobs whose path `keep` accepts on stdin, one `git cat-file --batch`, both with `GIT_NO_LAZY_FETCH=1` so a partial clone never fetches a missing object, and both through `run`. Unlike `extract_paths` it reads objects, not a checkout, so no `core.autocrlf`, `ident` or filter driver is applied.

**Removing a worktree** (`remove-worktree`). A build can leave an ignored directory without its owner's write permission, for instance by unpacking an archive; `git status` still reads the worktree as clean, but `git worktree remove` unregisters the worktree and then can't delete what that directory holds. So the owner is given access first, walking with `std::fs::symlink_metadata`; the walk is best effort, and what it can't change is left for git to report. Claude Code locks the worktrees it makes, which is why the worktree is unlocked first. **What git leaves behind.** `git worktree remove` deletes the directory first and unregisters the worktree whether or not that worked, so when a process adds a file while git is deleting (a build's subagent that the agent harness hasn't ended yet, when the build loop finishes the run on its report), git fails with `Directory not empty`, the worktree is no longer one, and no later `git worktree remove` or `clm wrap-up` finds the directory again: left there, it would be the human's to delete by hand, which isn't theirs to do (`build-loop-in-clm`). Git has by then judged the worktree removable and deleted what it tracked, so what is left is what git itself was deleting, and nothing is discarded that git wasn't already discarding. The two listings are the guard: a worktree git refused is still listed, so it is never touched, and a path git never listed as a worktree is never removed, whatever the caller passed. `git worktree list` prints resolved paths and a caller may pass one that isn't, which is why both are resolved, the caller's before git runs, while it still exists. 5 seconds is a constant, not a setting (`yagni`): it has to outlast a process that is ending, not one that is working, and it bounds how long a caller waits on a directory nothing can remove. Not taken: `--force`, which removes a worktree with uncommitted work and still fails on a directory being written into; the loop keeping the paths that were left in its file and trying again at each step, which is state and a second removal for a case the retry covers; and `clm wrap-up` sweeping `.claude/worktrees/` for directories git doesn't list, which could remove a directory the agent harness is still making. It lives here, not in `clm-run`, so every caller removes a worktree the same way (`reuse-first`); it is file-system work beside the git call, and lives in this crate because a worktree is git's.

## MODIFIED Requirements

### Requirement: bounded-runs <!-- base: 9e2c536eb64257c7 -->
`run_within(root, program, args, options, limit)` SHALL behave as `run(root, program, args, options)` when the command exits within `limit` and its stdout and stderr have closed once `limit` has passed. When the command exits within `limit` and a process it started still holds its stdout or stderr open once `limit` has passed, `run_within` SHALL return, within one second after the command exits and never later than one second after `limit` passes, what `run` would return were those streams closed then: `Ok` with the stdout bytes when the command exited with status 0, and otherwise `Error::Failed` with its status and its trimmed stderr, each stream holding every byte the command wrote to it and what the process it started wrote to it before then, and never `Error::TimedOut`. When it is still running once `limit` has passed, it SHALL kill the command's process, wait for it to exit, and return `Error::TimedOut { command, limit }`, whose display is `<command>: timed out after <limit>`, with the limit in `Duration`'s debug form. It SHALL return within one second after `limit` passes even when a process the command started keeps the command's stdout or stderr open. A zero `limit` SHALL return the timed-out error without starting the command, so a caller whose budget is spent passes what remains and starts nothing. `run` SHALL keep waiting with no limit.

#### Scenario: a command that finishes in time
- GIVEN a directory first on `PATH` holding an executable `gh` that prints `{"state":"MERGED"}`
- WHEN `run_within(root, "gh", &["pr", "view", "1"], Options::default(), 5s)` runs
- THEN it returns the bytes `{"state":"MERGED"}` followed by the newline the script printed

#### Scenario: a command that stalls
- GIVEN an executable `gh` first on `PATH` that writes its process ID to a file and then `exec`s `sleep 30`, so the sleep keeps that ID
- WHEN `run_within(root, "gh", &["api", "x"], Options::default(), 1s)` runs
- THEN it returns within 3 seconds an error whose display is `gh api x: timed out after 1s`, and the file holds an ID that `kill -0` reports no process for

#### Scenario: a child keeps the pipes open
- GIVEN an executable `gh` first on `PATH` that starts `sleep 30` in the background, inheriting its stdout, and then sleeps for 30 seconds itself
- WHEN `run_within(root, "gh", &["api", "x"], Options::default(), 200ms)` runs
- THEN it returns the timed-out error within 2 seconds

#### Scenario: a child keeps the pipes open after the command exits
- GIVEN an executable `gh` first on `PATH` that prints `done`, starts `sleep 30` in the background, inheriting its stdout and stderr, and exits 0
- WHEN `run_within(root, "gh", &["api", "x"], Options::default(), 10s)` runs
- THEN it returns within 2 seconds the bytes `done` followed by a newline

#### Scenario: a child keeps the pipes open after the command fails
- GIVEN an executable `gh` first on `PATH` that writes `nope` to its stderr, starts `sleep 30` in the background, inheriting its stdout and stderr, and exits 1
- WHEN `run_within(root, "gh", &["api", "x"], Options::default(), 10s)` runs
- THEN it returns within 2 seconds an `Error::Failed` whose status is 1 and whose stderr is `nope`

#### Scenario: no time left
- GIVEN an executable `gh` first on `PATH` that writes a file when it runs
- WHEN `run_within(root, "gh", &["api", "x"], Options::default(), Duration::ZERO)` runs
- THEN it returns an error whose display is `gh api x: timed out after 0ns`, and the file doesn't exist

#### Scenario: failures within the limit
- GIVEN a directory that isn't a git repository
- WHEN `run_within(root, "git", &["status"], Options::default(), 5s)` runs
- THEN it returns the same `Failed` error `run` returns, whose display holds `git status` and `not a git repository`

#### Serves
- capability/guard-sessions
