---
title: VCS
status: active
owners: ["@clunny"]
depends_on: []
paths: ["crates/clm-git/**"]
successor: ""
serves: [architecture/git-is-truth, architecture/reuse-first]
---

## Contract
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

## Design
Thin wrappers over `std::process::Command`, with no git library. Callers map `Error` into their own errors, `Failure` or a type of their own; none matches every variant by name, so a new variant breaks none of them. The crate has no dependency on the rest of the workspace, so every component can depend on it, and it stays out of the wasm builds (`web.worker` never shells out), which is why a crate that builds for wasm (`orchestrator`) uses `REMOTE` and `now()` only behind the feature that brings in `clm-git`.

`REMOTE` is a `pub const REMOTE: &str`, and `fetch` passes it rather than spelling the name. `now()` reads `SystemTime::now()`; callers that need finer time, such as `orchestrator`'s claim IDs, or a formatted date, such as `ledger`'s records, keep their own. `Failure` implements `std::error::Error`.

`run` and `run_within` share one body that takes an optional limit; `run` passes none and waits. With a limit, stdout and stderr are each drained on a detached thread (not a scoped one) and stdin written on a third, from an owned copy of the bytes since the thread may outlive the call, and the caller polls `try_wait` every few milliseconds until the command exits or the limit passes. When the limit passes it calls `kill` and `wait` on the child, so it is reaped, without joining the readers: a process the command started, such as a credential helper, may still hold the pipes open, and the readers end when it closes them. The readers send what they read as it comes, a chunk at a time, so the call can take a stream's bytes without its end. A command that exits in time may also leave a process holding its pipes, as an `ssh` control master that a `git` command over SSH starts with `ControlPersist` holds its stderr: the call then waits for the streams to close for at most 100 ms after the command exits, which is time enough for the bytes the command left in a pipe's buffer, and then returns the command's own outcome with the chunks received by then. It doesn't wait until the limit: both callers pass what remains of a budget shared by several commands (the guard's push check, its release lookup), so a command that exited at once and waited out the limit would leave the commands after it no time and turn them into timeouts. A read error and a stream that sent nothing map as they do when the stream closes, to `Error::Spawn` and to no bytes. A timed-out error there would report a command that did its work as one that ran out of time, which the guard's push check reads as a question it couldn't answer. Only the child is killed, not a process group, since `git` and `gh` are run directly and the portable `std` API kills one process. A limit is a `Duration`, not a deadline, so callers sharing a budget pass what remains of it. `run_within` is a function beside `run` rather than a `timeout` field of `Options`, since `Options` is built by struct literal in other crates (`bounded-runs`).

**The scan** (`one-git-layer`) lists the crates from the directories under `crates/` that hold a `Cargo.toml`, reads each one's `build.rs` and the `.rs` files under its `src/`, removes the whitespace of each file and looks for the strings the requirement names, keeping for each character the line it came from so a match is reported where it starts. It is one check here rather than one in each component, since a crate that didn't hold its own copy was read by nothing.

**Isolation.** The variables removed are the ones `git rev-parse --local-env-vars` prints, not a list written in the code, so a variable a later git adds is removed too. The work tree the environment names is found by running `git rev-parse --show-toplevel` and kept, with the two values it was found for, until either value changes; one answer is kept, not one for each pair, and a change of the process's current directory alone doesn't find it again.

`extract_tree` and `extract_paths` both read the whole of `rev` into a temporary index (`git read-tree <rev>` with `GIT_INDEX_FILE` set), so checkout reads `rev`'s `.gitattributes` from that index even when the paths asked for don't include them, then check entries out with `checkout-index` and the temporary directory as the work tree: `extract_tree` every entry (`--all`), `extract_paths` only the entries under `paths`, listed from the temporary index (`ls-files -z --full-name` with literal, top-relative pathspecs) and fed to `checkout-index --stdin -z`. Not `git checkout <rev> -- <paths>`: it runs the repository's `post-checkout` hook, and `checkout-index` runs none. `ls-files` matching nothing is not a git error, so `extract_paths` compares the listing with the paths asked for itself.

**Tracked files.** `tracked_files` runs `git ls-files -z --cached --deduplicate` in `root`, and `tracked_matching` the same with `--` and one pathspec `:(glob)<glob>` per glob, so a component's `paths` (`**`, `*`, `?` as in its frontmatter) match by git's own glob rules; with no glob it runs nothing, since a component with no `paths` owns no file, not every file. `--deduplicate` lists a file the index holds at several merge stages, during a conflict, once, and needs git 2.31 or later. On Unix the path is built from git's bytes with `OsStr::from_bytes`, so a caller can hand it back to git (`check-attr --stdin -z`); elsewhere it is converted lossily, which CLM accepts since it runs on Linux and macOS. `files_to_commit` sorts what git lists, which names the index's files and then the untracked ones.

**Reading a commit** (`tree-blobs`). `tree_blobs` runs `git ls-tree -r -z --full-tree <commit>` and then, with the IDs of the blobs whose path `keep` accepts on stdin, one `git cat-file --batch`, both with `GIT_NO_LAZY_FETCH=1` so a partial clone never fetches a missing object, and both through `run`. Unlike `extract_paths` it reads objects, not a checkout, so no `core.autocrlf`, `ident` or filter driver is applied.

**Removing a worktree** (`remove-worktree`). A build can leave an ignored directory without its owner's write permission, for instance by unpacking an archive; `git status` still reads the worktree as clean, but `git worktree remove` unregisters the worktree and then can't delete what that directory holds. So the owner is given access first, walking with `std::fs::symlink_metadata`; the walk is best effort, and what it can't change is left for git to report. Claude Code locks the worktrees it makes, which is why the worktree is unlocked first. **What git leaves behind.** `git worktree remove` deletes the directory first and unregisters the worktree whether or not that worked, so when a process adds a file while git is deleting (a build's subagent that the agent harness hasn't ended yet, when the build loop finishes the run on its report), git fails with `Directory not empty`, the worktree is no longer one, and no later `git worktree remove` or `clm wrap-up` finds the directory again: left there, it would be the human's to delete by hand, which isn't theirs to do (`build-loop-in-clm`). Git has by then judged the worktree removable and deleted what it tracked, so what is left is what git itself was deleting, and nothing is discarded that git wasn't already discarding. The two listings are the guard: a worktree git refused is still listed, so it is never touched, and a path git never listed as a worktree is never removed, whatever the caller passed. `git worktree list` prints resolved paths and a caller may pass one that isn't, which is why both are resolved, the caller's before git runs, while it still exists. 5 seconds is a constant, not a setting (`yagni`): it has to outlast a process that is ending, not one that is working, and it bounds how long a caller waits on a directory nothing can remove. Not taken: `--force`, which removes a worktree with uncommitted work and still fails on a directory being written into; the loop keeping the paths that were left in its file and trying again at each step, which is state and a second removal for a case the retry covers; and `clm wrap-up` sweeping `.claude/worktrees/` for directories git doesn't list, which could remove a directory the agent harness is still making. It lives here, not in `clm-run`, so every caller removes a worktree the same way (`reuse-first`); it is file-system work beside the git call, and lives in this crate because a worktree is git's.

## Requirements

### Requirement: one-git-layer
No crate of the workspace but `clm-git` SHALL start `git`, `gh` or `az` as a process itself. No crate but `clm-git`, `clm-check`, `clm-land`, `clm-loop`, `clm-run` and `clm-verify`, which run programs the project names (a detector, a land check, `clm`, `sh`), SHALL start any process itself. The scan SHALL read `build.rs` and every `.rs` file under `src/` of every directory under `crates/` that holds a `Cargo.toml`, whether or not the crate lists `clm-git`, test modules included, and SHALL compare the text of each file with every whitespace character removed against these literal strings, failing with the file and the line the match's first character is on for each one it finds:
- in any crate but `clm-git`: `Command::new("git"`, `Command::new("gh"` and `Command::new("az"`, each without its closing bracket, so a call rustfmt ends with a comma matches too;
- in any crate but the six named: also `process::Command`, a `process::{` whose braces hold `Command` before they close, and `Command::new(` not directly preceded by `clap::`. The first two match a longer name that begins `Command` too, such as `process::CommandExt`, which is meant: nothing outside the six needs one.

It SHALL fail naming the crate when it reads no `.rs` file under a crate's `src/`. A crate's `tests/` and `benches/`, and code that isn't Rust, are outside it.

#### Scenario: a wrapper comes back
- GIVEN a crate under `crates/` whose source holds `Command::new("git")`
- WHEN the workspace's tests run
- THEN the scan test fails naming that file and line

#### Scenario: a crate that hasn't migrated
- GIVEN a crate under `crates/` that doesn't list `clm-git` and whose source holds `Command::new("git")`
- WHEN the scan runs
- THEN it fails naming that file and line

#### Scenario: clean dependants
- GIVEN every crate free of such calls
- WHEN the scan runs
- THEN it passes

#### Scenario: the Azure CLI
- GIVEN a crate other than `clm-git` whose source holds `Command::new("az")`
- WHEN the scan runs
- THEN it fails naming that file and line

#### Scenario: a wrapper that takes the program
- GIVEN a crate other than the six named whose source holds `std::process::Command::new(program)`, and another whose source holds `use std::process::{Child, Command};`
- WHEN the scan runs
- THEN it fails naming each file and line

#### Scenario: a call split across lines
- GIVEN `clm-land` holding `Command::new(`, then `"gh",` on the next line and `)` on the one after
- WHEN the scan runs
- THEN it fails naming the line that holds `Command::new(`

#### Scenario: a crate that runs other programs
- GIVEN `clm-land` holding `Command::new("sh")`, and a crate outside the six holding `clap::Command::new(name)`
- WHEN the scan runs, and again with `clm-land` holding `Command::new("git")` in its `build.rs`
- THEN the first passes, and the second fails naming `crates/clm-land/build.rs` and that line

#### Scenario: the whole workspace is read
- GIVEN this repository, and a fixture workspace one of whose crates has no `.rs` file under `src/`
- WHEN the scan runs on each
- THEN it passes on this repository, and fails on the fixture naming that crate

### Requirement: raw-runs
`run(root, program, args, options)` SHALL run `program`, whatever program the caller names (`git`, `gh`, `az`), as resolved on `PATH`, in `root` with `options.stdin` written to its stdin when given and `options.env` set, and SHALL return its stdout as bytes, untrimmed, on exit 0, or the same error `git` returns otherwise.

#### Scenario: stdin and NUL-separated output
- GIVEN a repository whose `.gitattributes` marks `a.lock` `linguist-generated`
- WHEN `run(root, "git", &["check-attr", "--stdin", "-z", "linguist-generated"], stdin "a.lock\0")` runs
- THEN its stdout bytes hold `a.lock\0linguist-generated\0set\0`

#### Scenario: environment honoured
- GIVEN a repository and a path `idx` with no file
- WHEN `run(root, "git", &["read-tree", "HEAD"], env GIT_INDEX_FILE=idx)` runs
- THEN `idx` exists afterwards and the repository's own index is unchanged

#### Scenario: any program on PATH
- GIVEN a directory first on `PATH` holding an executable `az` that prints its arguments
- WHEN `run(root, "az", &["repos", "pr", "list"], options)` runs
- THEN its stdout bytes hold `repos pr list`

### Requirement: run-git
`git(root, args)` and `gh(root, args)` SHALL run the command in `root` as resolved on `PATH`, return its stdout with trailing whitespace trimmed on exit 0, and otherwise return `Error::Failed { command, status, stderr }`, `command` being the program and its arguments joined by spaces, `status` the command's exit status and `stderr` its standard error with leading and trailing whitespace removed, whose display is `<command>: <stderr>`, or `<command>: <status>` when `stderr` is empty. A program that can't be started SHALL return an error whose display begins `<command>: `.

#### Scenario: success
- GIVEN a git repository at `root`
- WHEN `git(root, &["rev-parse", "--abbrev-ref", "HEAD"])` runs on branch `main`
- THEN it returns `main`

#### Scenario: failure carries stderr
- GIVEN a directory that isn't a git repository
- WHEN `git(root, &["status"])` runs
- THEN it returns an error whose display holds `git status` and `not a git repository`

#### Scenario: the failure's fields
- GIVEN the same directory
- WHEN `git(root, &["status"])` runs
- THEN the error is `Error::Failed` whose `status` is the exit status git ended with, not 0, and whose `stderr` holds `not a git repository` with no line break at its end

#### Scenario: fake gh on PATH
- GIVEN a directory first on `PATH` holding an executable `gh` that prints `{"state":"MERGED"}`
- WHEN `gh(root, &["pr", "view", "1"])` runs
- THEN it returns `{"state":"MERGED"}`

#### Scenario: a program that can't start
- GIVEN options whose `env` sets `PATH` to a directory holding no `git`
- WHEN `run(root, "git", &["status"], options)` runs
- THEN it returns an error whose display begins `git status: `

### Requirement: repo-queries
`is_ancestor(root, a, b)` SHALL be true when `git merge-base --is-ancestor a b` exits 0, false when it exits 1, and an error for any other exit. `current_branch(root)` SHALL return the checked-out branch's name, or `None` on a detached `HEAD`. `worktrees(root)` SHALL list every worktree of the repository with its path, its branch (`None` when detached) and whether it is locked. `extract_tree(root, rev)` SHALL return a temporary directory holding `rev`'s tree, leaving the working tree and the index unchanged, and an error for a `rev` that names no tree.

#### Scenario: ancestry
- GIVEN commits `a` and `b` where `b` is a descendant of `a`
- WHEN `is_ancestor(root, a, b)` and `is_ancestor(root, b, a)` run
- THEN the first is true and the second false

#### Scenario: detached head
- GIVEN a repository with `HEAD` detached at a commit
- WHEN `current_branch(root)` runs
- THEN it returns `None`

#### Scenario: locked worktree
- GIVEN a linked worktree on branch `impl/a` that `git worktree lock` has locked
- WHEN `worktrees(root)` runs
- THEN its entry for that path has branch `impl/a` and is locked

#### Scenario: extracted tree
- GIVEN a repository whose `origin/main` holds `specs/SPEC.md` and whose working tree has an uncommitted edit to it
- WHEN `extract_tree(root, "origin/main")` runs
- THEN the returned directory's `specs/SPEC.md` is the committed text, and the working tree's edit is still there

#### Scenario: an unknown revision
- GIVEN a repository with no revision `no-such-rev`
- WHEN `is_ancestor(root, "HEAD", "no-such-rev")` and `extract_tree(root, "no-such-rev")` run
- THEN each returns an error

### Requirement: extract-paths
`extract_paths(root, rev, paths)` SHALL return a temporary directory holding exactly the files of `rev`'s tree under each of `paths`, at the same relative paths, and no other file, leaving the working tree and the index unchanged and running no hook. Each path SHALL be taken literally (no glob) and relative to the top of `rev`'s tree whatever directory `root` is, and names a file or a directory; a trailing `/` is allowed. An empty `paths` SHALL return an empty directory. A path that is empty, absolute, or has a `.` or `..` component, or that matches nothing in `rev`'s tree, SHALL make it return an error whose display names that path.

#### Scenario: only the paths asked for
- GIVEN a repository whose `HEAD` holds `.gitattributes`, `specs/SPEC.md`, `specs/a/SPEC.md` and `src/main.rs`, and whose working tree has an uncommitted edit to `specs/SPEC.md`
- WHEN `extract_paths(root, "HEAD", &["specs"])` runs
- THEN the returned directory holds `specs/SPEC.md` and `specs/a/SPEC.md` with their committed text, and no `src/` and no `.gitattributes`, and the working tree's edit and the repository's index are unchanged

#### Scenario: a missing path
- GIVEN a repository whose `HEAD` holds `src/main.rs` and no `specs/`
- WHEN `extract_paths(root, "HEAD", &["src", "specs"])` runs
- THEN it returns an error whose display holds `specs`

#### Scenario: literal, top-relative paths
- GIVEN a repository whose `HEAD` holds `sub/a.rs` and `top.rs`, with `root` its directory `sub`
- WHEN `extract_paths(root, "HEAD", &["top.rs"])` and `extract_paths(root, "HEAD", &["*.rs"])` run
- THEN the first returns a directory holding only `top.rs`, and the second an error naming `*.rs`

#### Scenario: no hook runs
- GIVEN a repository whose `HEAD` holds `specs/SPEC.md`, with an executable `post-checkout` hook that creates a file `hooked` in the repository
- WHEN `extract_paths(root, "HEAD", &["specs"])` runs
- THEN no file `hooked` exists afterwards

#### Serves
- capability/land-safely
- architecture/hash-based-staleness

### Requirement: extract-as-checkout
`extract_tree` and `extract_paths` SHALL write each file with the bytes `git checkout` of `rev` writes for it in this repository: line-ending conversion, `ident` and filter drivers applied from the attributes in `rev`'s tree and the configuration git reads in `root`, whether or not `paths` include the `.gitattributes` that set them. They SHALL ignore `export-ignore` and `export-subst`.

#### Scenario: line endings from rev's attributes
- GIVEN a repository whose `HEAD` holds a top-level `.gitattributes` with `*.md text eol=crlf` and `specs/SPEC.md` committed with LF line endings
- WHEN `extract_tree(root, "HEAD")` and `extract_paths(root, "HEAD", &["specs"])` run
- THEN `specs/SPEC.md` in each returned directory has CRLF line endings, byte for byte the file a fresh checkout of `HEAD` writes

#### Scenario: smudge filter applied
- GIVEN a repository whose `HEAD` has `.gitattributes` with `specs/SPEC.md filter=up`, and whose configuration sets `filter.up.smudge` to `tr a-z A-Z` and `filter.up.clean` to `cat`
- WHEN `extract_paths(root, "HEAD", &["specs"])` runs
- THEN the returned `specs/SPEC.md` holds the committed text upper-cased

#### Scenario: export attributes ignored
- GIVEN a repository whose `HEAD` has `.gitattributes` with `specs/** export-ignore` and `specs/SPEC.md export-subst`, and `specs/SPEC.md` holding `$Format:%H$`
- WHEN `extract_tree(root, "HEAD")` and `extract_paths(root, "HEAD", &["specs"])` run
- THEN each returned directory holds `specs/SPEC.md` with `$Format:%H$` unexpanded

#### Serves
- architecture/hash-based-staleness
- architecture/no-silent-drift

### Requirement: bounded-runs
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

### Requirement: remote-name
`clm-git` SHALL export `REMOTE`, a `&str` equal to `"origin"`.

#### Scenario: the remote's name
- GIVEN the crate
- WHEN a test reads `clm_git::REMOTE`
- THEN it is `origin`

### Requirement: unix-now
`now()` SHALL return the current time as whole seconds since the Unix epoch, as a `u64`.

#### Scenario: the clock
- GIVEN `before`, the seconds since the Unix epoch read from `SystemTime` just before the call, and `after`, read just after it
- WHEN `now()` runs
- THEN it returns a value at least `before` and at most `after`

#### Serves
- architecture/reuse-first

### Requirement: failure
`Failure` SHALL have the variants `Refused(String)` and `Git(String)`. Converting an `Error` into a `Failure` SHALL give `Git` holding the `Error`'s display. A `Failure` SHALL display as the string it holds, and `exit_code()` SHALL return 1 for `Refused` and 2 for `Git`.

#### Scenario: refused
- GIVEN `Failure::Refused("not on a branch")`
- WHEN it is displayed and its exit code read
- THEN it displays `not on a branch` and its exit code is 1

#### Scenario: git failed
- GIVEN the error `git(root, &["status"])` returns in a directory that isn't a git repository
- WHEN it is converted into a `Failure` with `?`
- THEN the `Failure` is `Git`, displays the same text as the error, which holds `git status`, and its exit code is 2

#### Serves
- architecture/reuse-first

### Requirement: tracked-files
`tracked_files(root)` SHALL return every file the index holds under `root`, as `git ls-files --cached` run in `root` names it, each path relative to `root` and in the order git lists them, each once even when the index holds it at several merge stages. `tracked_matching(root, globs)` SHALL return those of them that any glob matches as the pathspec `:(glob)<glob>`, and SHALL return nothing, running no git, when `globs` is empty. On Unix each returned path's bytes SHALL be exactly the name git lists, whether or not it is UTF-8. Both SHALL return an error when `root` is in no git repository and git is run.

#### Scenario: globs match by git's rules
- GIVEN a repository tracking `src/a/x.rs`, `src/a/b/y.rs` and `src/b/z.rs`
- WHEN `tracked_matching(root, ["src/a/**"])` runs
- THEN it returns `src/a/b/y.rs` and `src/a/x.rs`, and not `src/b/z.rs`

#### Scenario: relative to the root, in git's order
- GIVEN the same repository
- WHEN `tracked_files` runs with `root` the directory `src`
- THEN it returns `a/b/y.rs`, `a/x.rs` and `b/z.rs`, in that order

#### Scenario: no globs runs no git
- GIVEN a directory that isn't a git repository
- WHEN `tracked_matching(root, [])` runs
- THEN it returns an empty list, not an error

#### Scenario: outside a repository
- GIVEN a directory that isn't a git repository
- WHEN `tracked_files(root)` runs
- THEN it returns an error

#### Scenario: a conflicted file once
- GIVEN a repository mid-merge whose index holds `src/a/x.rs` at stages 1, 2 and 3
- WHEN `tracked_files(root)` and `tracked_matching(root, ["src/**"])` run
- THEN each lists `src/a/x.rs` exactly once

#### Scenario: a name that isn't UTF-8
- GIVEN, on Unix, a repository tracking a file whose name is the bytes `src/\xff.rs`
- WHEN `tracked_files(root)` runs
- THEN one returned path's bytes are exactly `src/\xff.rs`

#### Serves
- capability/see-project-state

### Requirement: tree-blobs
`tree_blobs(root, commit, keep)` SHALL return, in the order `git ls-tree -r` lists them, the path from the top of the repository and the bytes as stored of every blob of `commit`'s tree whose path `keep` accepts, skipping gitlinks, symbolic links and paths that aren't UTF-8, without fetching and without writing any file, the working tree and the index unchanged, whatever directory of the repository `root` is. A kept blob missing from the local objects SHALL make it return an error whose display names the blob's path. A `commit` that names no commit SHALL make it return an error.

#### Scenario: a commit's files
- GIVEN a repository whose `HEAD` holds `a.txt` reading `$Id$` under a `.gitattributes` setting `a.txt ident`, `sub/b.rs`, a symbolic link `l` to `a.txt`, and a working tree with an uncommitted edit to `a.txt`
- WHEN `tree_blobs(root, "HEAD", keep)` runs with `root` the directory `sub` and `keep` accepting every path
- THEN it returns `.gitattributes`, `a.txt` reading `$Id$` as committed, not expanded, and `sub/b.rs`, in that order, and no `l`; the working tree's edit and the index are unchanged

#### Scenario: only the kept paths
- GIVEN the same repository
- WHEN `keep` accepts only paths starting `sub/`
- THEN it returns only `sub/b.rs`

#### Scenario: a missing blob
- GIVEN a repository whose `HEAD` holds `a.txt`, with the blob's object file removed from `.git/objects`
- WHEN `tree_blobs(root, "HEAD", keep)` runs with `keep` accepting every path
- THEN it returns an error whose display holds `a.txt`

### Requirement: common-dir
`common_dir(root)` SHALL return the absolute path of the clone's common git directory, the same from the main checkout and from any linked worktree of it, or an error outside a git repository.

#### Scenario: from a linked worktree
- GIVEN a repository at `r` with a linked worktree at `w`
- WHEN `common_dir` runs with `r`, and with `w`
- THEN both return the absolute path of `r/.git`

#### Scenario: no repository
- GIVEN a directory that is in no git repository
- WHEN `common_dir` runs with it
- THEN it returns an error

### Requirement: files-to-commit
`files_to_commit(root)` SHALL return every file the index holds under `root` and every untracked file under it that git's standard excludes don't ignore (`.gitignore` files, `.git/info/exclude` and `core.excludesFile`), as `git ls-files -z --cached --others --exclude-standard --deduplicate` run in `root` names them, each path relative to `root`, each once, sorted by the bytes of the names, so a caller sees the files a `git add -A` would commit. A file deleted from the work tree that the index still holds is listed, as `tracked_files` lists it. On Unix each returned path's bytes SHALL be exactly the name git lists, whether or not it is UTF-8.

#### Scenario: tracked and untracked, not ignored
- GIVEN a repository tracking `src/a.rs` and `.gitignore` holding `build/`, with the untracked files `notes.txt` and `build/out.o`
- WHEN `files_to_commit(root)` runs
- THEN it returns `.gitignore`, `notes.txt` and `src/a.rs`, and not `build/out.o`

#### Scenario: relative to the root, sorted
- GIVEN a repository tracking `sub/b.rs`, with the untracked file `sub/a.rs`
- WHEN `files_to_commit` runs with `root` the directory `sub`
- THEN it returns `a.rs` and then `b.rs`

#### Scenario: a file both staged and changed once
- GIVEN a repository tracking `src/a.rs`, changed in the work tree since it was staged
- WHEN `files_to_commit(root)` runs
- THEN it lists `src/a.rs` exactly once

#### Serves
- capability/adopt-clm

### Requirement: remove-worktree
`remove_worktree(root, path)` SHALL remove the linked worktree at `path` whole, ignored files included: it SHALL unlock it, ignoring the failure for a worktree that isn't locked; give the owner read, write and execute permission on `path` and on every directory under it that lacks them, a directory its owner can neither write, read nor list included; and then run `git worktree remove <path>`, without `--force`. When `path` still exists after that, whether `git worktree remove` failed or succeeded, and git listed a worktree at `path` before the call and lists none there after it, it SHALL remove what is left of the directory itself, giving the owner the same permissions first, and SHALL try its own removal again while `path` still exists, for at most 5 seconds from its first try of it, without running `git worktree remove` again, so a directory a process was still writing into goes once the writing stops; it SHALL return `Ok` once its own removal leaves `path` gone, also when `git worktree remove` had failed. It SHALL compare `path` with the paths git lists once the symbolic links in both are resolved, resolving `path` before `git worktree remove` runs. It SHALL never follow or change a symbolic link under `path`, so nothing outside `path` is removed and no permission outside `path` changes. It SHALL remove nothing itself while git still lists a worktree at `path`, or when git listed none there before the call. When it removed nothing itself, or `path` still exists at the end, it SHALL return `git`'s error when `git worktree remove` failed, as for a `path` that doesn't exist, and otherwise `Error::StillOnDisk`, whose display is `<path>: still on disk`, when `path` still exists. It SHALL NOT judge whether the worktree has uncommitted changes, which is its caller's question.

#### Scenario: unwritable directory removed
- GIVEN a repository with a locked linked worktree `w` holding an ignored directory `cache/` with a file in it, `cache/` without its owner's write permission
- WHEN `remove_worktree(root, w)` runs
- THEN it returns `Ok`, `w` is gone from disk and `git worktree list` doesn't name it

#### Scenario: symbolic link not followed
- GIVEN a linked worktree `w` holding an ignored symbolic link `out` to a directory `d` outside `w` that holds a file and lacks its owner's write permission
- WHEN `remove_worktree(root, w)` runs
- THEN `w` is gone, and `d` and its file still exist with `d`'s permissions unchanged

#### Scenario: git refuses
- GIVEN a linked worktree `w` holding an untracked file that isn't ignored
- WHEN `remove_worktree(root, w)` runs
- THEN it returns an error whose display holds `git worktree remove`, and `w` and the file are still there

#### Scenario: git leaves the directory
- GIVEN a linked worktree `w`, and a `git` first on `PATH` whose `worktree remove` runs the real one, then makes `w` again holding a file and an ignored symbolic link `out` to a directory `d` outside `w` that holds a file, and exits 1 as git does when it can't delete a directory, running every other command as the real one
- WHEN `remove_worktree(root, w)` runs
- THEN it returns `Ok`, `w` is gone from disk, `git worktree list` doesn't name it, and `d` and its file still exist

#### Scenario: removable only later
- GIVEN a linked worktree `w` whose parent directory lacks its owner's write permission, so neither git nor the function can delete `w`, until 300 milliseconds after the call starts, when the permission is given back
- WHEN `remove_worktree(root, w)` runs
- THEN it returns `Ok`, and `w` is gone from disk and `git worktree list` doesn't name it

#### Scenario: never removable
- GIVEN a linked worktree `w` whose parent directory lacks its owner's write permission throughout
- WHEN `remove_worktree(root, w)` runs
- THEN it returns, after at least 5 seconds and within 8, an error whose display holds `git worktree remove`, `w` still exists, and `git worktree list` doesn't name it

#### Scenario: not a worktree
- GIVEN a repository and a directory `d` holding a file, which git lists no worktree at
- WHEN `remove_worktree(root, d)` runs
- THEN it returns an error whose display holds `git worktree remove`, and `d` and its file are still there

#### Serves
- capability/work-concurrently

### Requirement: git-isolated
`git_isolated(dir, args)` SHALL run `git` with `args` in `dir` and return what `git(dir, args)` returns there, its stdout trimmed or the same error, except that each environment variable `git rev-parse --local-env-vars` prints SHALL be removed from the command's environment, whatever the calling process has set.

#### Scenario: a bare repository beside a clone the environment names
- GIVEN a work tree `w` whose git directory `g` is elsewhere (`git init --separate-git-dir g w`), and `git` started with `GIT_DIR=g` and `GIT_WORK_TREE=w` in its environment, as from a process that has them set
- WHEN `git_isolated(w, &["init", "--quiet", "--bare", b])` runs, `b` an absolute path with nothing there
- THEN it succeeds, `b` is a bare repository (`git --git-dir b rev-parse --is-bare-repository` prints `true`), and `g` is unchanged; `git(w, ...)` with the same arguments in that environment fails with git's `GIT_WORK_TREE ... not allowed without specifying GIT_DIR`

#### Scenario: the same as git otherwise
- GIVEN a git repository at `root` on branch `main`, and no git variable set
- WHEN `git_isolated(root, &["rev-parse", "--abbrev-ref", "HEAD"])` and `git_isolated(root, &["no-such-command"])` run
- THEN the first returns `main`, and the second an error whose display holds `git no-such-command`

### Requirement: git-follows-its-clone
Every `git` command `clm-git` runs, through `git`, `run` or `run_within` with the program `git`, or any function built on them, SHALL run as `git_isolated` runs it, with each variable `git rev-parse --local-env-vars` prints removed from its environment, when its `root` lies outside the work tree the calling process's environment names; a variable the caller sets in the `env` of the `Options` it gives SHALL be kept, even one git lists. The work tree the environment names SHALL be the directory `git rev-parse --show-toplevel` prints when run, not through this rule, with the process's environment in the process's current directory, found for the values `GIT_DIR` and `GIT_WORK_TREE` hold at the call and found again when either value has changed since it was last found. `root` SHALL lie outside it when `root`, with symbolic links resolved, is neither that directory nor under it, or when `root` or a directory between it and that work tree holds a `.git` entry (a file or a directory), as a linked worktree or another repository made inside the work tree, such as `clm land`'s kept tree under `.clm/`, does. When the environment sets neither variable, when that `rev-parse` fails (a bare repository names no work tree), and for a `root` inside the work tree, the command SHALL run with the environment as the process has it. A program other than `git` SHALL run with the environment as the process has it.

#### Scenario: a plain copy beside a clone the environment names
- GIVEN a work tree `w` with a commit holding `a`, whose git directory `g` is elsewhere (`git init --separate-git-dir g w`, its `.git` file removed), a plain directory `c` outside `w` holding only `b`, and a process whose environment sets `GIT_DIR=g` and `GIT_WORK_TREE=w`
- WHEN that process runs `git(c, &["ls-files"])`, `tracked_files(c)` and `git(w, &["ls-files"])`
- THEN the first two fail as in a directory that is no repository, their errors holding `not a git repository`, and the third returns `a`

#### Scenario: a temporary worktree beside a clone the environment names
- GIVEN the same `w`, `g` and environment, with a second commit on `w`'s branch, and a linked worktree `t` outside `w` checked out detached at the first commit
- WHEN that process runs `git(t, &["checkout", "--quiet", "--detach", <second commit>])` and then `git(t, &["rev-parse", "HEAD"])`
- THEN the second returns the second commit's ID, `w`'s `HEAD` still names its branch, and `git status --porcelain` run in `w` with those variables prints nothing

#### Scenario: a worktree nested in the clone the environment names
- GIVEN the same `w`, `g`, environment and two commits, and a linked worktree `w/.clm/n`, ignored by `w`, checked out detached at the first commit
- WHEN that process runs `git(w/.clm/n, &["checkout", "--quiet", "--detach", <second commit>])` and then `git(w/.clm/n, &["rev-parse", "--show-toplevel", "HEAD"])`
- THEN the second returns `w/.clm/n` and the second commit's ID, `w`'s `HEAD` still names its branch, and `git status --porcelain` run in `w` with those variables prints nothing

#### Scenario: the caller's variables kept outside
- GIVEN the same environment and linked worktree `t`, and a path `idx` with no file
- WHEN that process runs `run(t, "git", &["read-tree", "HEAD"], env GIT_INDEX_FILE=idx)`
- THEN `idx` exists afterwards, and neither `t`'s index nor `g`'s changed

### Requirement: fetch-retries
`clm-git` SHALL offer `fetch_retrying(root, args)`, which runs `git` with `args`, a fetch naming `REMOTE`, in `root`, and when it fails runs it again, at most twice more, waiting 1 second before the first of these and 2 seconds before the second. It SHALL return success at the first run that succeeds. When the last run fails, it SHALL run `git ls-remote --quiet <REMOTE> refs/heads/main` in `root` and return a `FetchError`: `Offline`, whose display is `origin can't be reached`, when that fails too, and otherwise `Git` holding the last run's error, whose display is that error's, so a fetch that lost a race with another process of the same clone over a remote-tracking ref (git's `cannot lock ref`) goes on, and a caller says it is offline only when `origin` can't be reached.

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

#### Serves
- capability/work-concurrently

### Requirement: fetch-tries-again
`fetch(root)` SHALL run `git fetch origin` in `root` and, when it fails, run it again at most twice more, with the waits `fetch_retrying` makes (`fetch-retries`). It SHALL return success at the first run that succeeds and, when the last run fails, that run's error, running no `git ls-remote`, so a caller that fetches through it goes on after a fetch that lost a race with another process of the same clone over a remote-tracking ref.

#### Scenario: fetch uses it
- GIVEN a repository whose remote `origin` has a branch `b` the clone hasn't fetched
- WHEN `fetch(root)` runs
- THEN `refs/remotes/origin/b` exists afterwards

#### Scenario: a plain fetch that loses a ref race
- GIVEN a clone whose `origin` is a bare repository with a branch `b` the clone hasn't fetched, and an executable `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/main'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `fetch(root)` runs
- THEN it returns success after at least 1 second, `refs/remotes/origin/b` exists, and the fake ran `fetch` twice

#### Scenario: a plain fetch that keeps failing
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `fetch(root)` runs
- THEN it returns an error whose display holds `cannot lock ref`, after at least 3 seconds, and the fake ran `fetch` three times and `ls-remote` never

#### Serves
- capability/work-concurrently

### Requirement: worktrees-tries-again
`worktrees(root)` SHALL run `git worktree list --porcelain -z` in `root` and, when it fails, run it again at most twice more, waiting 100 milliseconds before the first of these and 500 milliseconds before the second. It SHALL return the list read from the first run that succeeds and, when the last run fails, that run's error, so a caller that lists worktrees while another process of the same clone is adding or removing one, whose entry under the common git directory is then half written (git's `failed to read .git/worktrees/<name>/commondir`), goes on.

#### Scenario: a listing that meets a worktree being added
- GIVEN a repository with a linked worktree on branch `impl/a`, and an executable `git` first on `PATH` that, for the first `worktree list`, prints `fatal: failed to read .git/worktrees/half/commondir: Success` on stderr and exits 128, and runs the real `git` for every other call
- WHEN `worktrees(root)` runs
- THEN it returns both worktrees, the linked one on branch `impl/a`, after at least 100 milliseconds, and the fake ran `worktree list` twice

#### Scenario: a listing that keeps failing
- GIVEN the same, with a `git` that fails every `worktree list` that way
- WHEN `worktrees(root)` runs
- THEN it returns an error whose display holds `commondir`, after at least 600 milliseconds, and the fake ran `worktree list` three times

#### Serves
- capability/work-concurrently

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
