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
The one place CLM shells out to `git` and to GitHub's CLI, `gh` (`git-plumbing-layer`, `reuse-first`). Crate `clm-git` offers `git(root, args)` and `gh(root, args)`, which run the command in `root` and return its trimmed stdout, or an error carrying the command, its exit status and its stderr; `fetch(root)` (`git fetch origin`); `is_ancestor(root, a, b)`; `current_branch(root)` (`None` on a detached `HEAD`); `worktrees(root)` (each worktree's path, branch and whether it is locked, from `git worktree list --porcelain`); `extract_tree(root, rev)`, which checks a revision's tree out into a temporary directory with a temporary index, leaving the working tree alone; `extract_paths(root, rev, paths)`, which does the same for only the files under `paths` (each a file or directory, relative to the top of the tree), so a caller that needs `specs/` doesn't write the whole repository. Both write each file as `git checkout` of `rev` would write it into this repository's working tree: line endings, `ident` and filter drivers (smudge) are applied from `rev`'s attributes and the repository's configuration, and `export-ignore` and `export-subst` are ignored, so hashes read from an extracted tree equal those read from a checkout of the same revision (`extract-as-checkout`). `tracked_files(root)` and `tracked_matching(root, globs)` give the files the index holds under `root`, all of them or those a component's `paths` globs match, each once (`tracked-files`). `run(root, program, args, Options { stdin, env })` is the raw form the others are built on: `program` is whatever program the caller names (`git`, `gh` or any other), `stdin` optional bytes to feed it, `env` variables to set (`GIT_INDEX_FILE`, `GIT_AUTHOR_DATE`, a `GH_TOKEN`), and it returns stdout as bytes untrimmed, so callers that pipe `git check-attr --stdin -z`, run with a temporary index or run a host CLI without a wrapper of its own have one place to go. `run_within(root, program, args, options, limit)` is `run` with a time limit: a command still running when `limit` runs out is killed and the call returns a timed-out error naming the command and the limit (`bounded-runs`), so a caller that must answer in time, such as the release guard, needs no thread of its own and leaves no process behind. Every program is resolved on `PATH`, so a test puts a fake `gh`, `az` or `git` first on `PATH` and the layer runs it. Every crate that lists `clm-git` as a dependency runs no `git` or `gh` itself; a test in this crate fails when one does. A crate comes under the scan by adding the dependency and gaining its own `git-through-layer` requirement. `tree_blobs(root, commit, keep)` reads a commit's files from the local objects in memory, writing nothing, with one `git ls-tree` and one `git cat-file --batch`, and `common_dir(root)` gives the clone's common git directory as an absolute path, shared by every worktree (`tree-blobs`, `common-dir`). `remove_worktree(root, path)` removes a linked worktree whole, ignored files included, also when a build left a directory in it its owner can't write, the one removal `clm run --finish`, `clm wrap-up` and `clm runner` share (`remove-worktree`); whether the worktree is clean enough to remove is the caller's question. `git_isolated(dir, args)` is `git` run as though the environment named no repository: every variable `git rev-parse --local-env-vars` prints (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` and the rest) is removed from the command's environment, so a command that makes or works in a repository other than the caller's clone, such as `git init --bare`, finds it from `dir` alone and doesn't follow, or get refused by, the variables that name the clone (`git-isolated`). Every other git command the crate runs, through `git`, `run`, `run_within` or any function built on them, runs as `git_isolated` does when its `root` lies outside the work tree the calling process's environment names, so a plain copy CLM writes elsewhere is no repository to git and a temporary worktree is its own checkout, whatever `GIT_DIR` and `GIT_WORK_TREE` name; inside that work tree the environment is honoured as before (`git-follows-its-clone`).

It also owns the plumbing that the crates which shell out would otherwise each copy (`shared-plumbing-in-git`): `REMOTE`, the name of the remote every command fetches from and pushes to (`"origin"`, the one `fetch` fetches); `now()`, the current time in whole seconds since the Unix epoch; and `Failure`, the error of a command that either refused (`Refused(why)`, exit 1) or failed in git (`Git(message)`, exit 2). `Failure` converts from this crate's `Error` with `?` (as `Git`), displays its message alone, and gives its exit code with `exit_code()`. A crate whose errors are exactly these two, with no conversions from other crates' errors, uses `Failure` as its `Error`, so `crates/clm` can map them to an exit code in one place; a crate that needs a third kind, or `?` from another crate's error type, keeps its own type.

## Design
Thin wrappers over `std::process::Command`, with no git library. One `Error` type: `Failed { command, status, stderr }`, `Spawn { command, source }`, `TimedOut { command, limit }` and the variants below (`Unreadable`, `StillOnDisk`), printed as `<command>: <stderr or status>`, or `<command>: timed out after <limit>` with the limit in `Duration`'s debug form (`10s`, `250ms`). Callers map it into their own errors, `Failure` or a type of their own; none matches every variant by name, so a new variant breaks none of them. `StillOnDisk { path }`, displayed `<path>: still on disk`, is `remove_worktree`'s when git succeeded but the directory remains. The crate has no dependency on the rest of the workspace, so every component can depend on it, and it stays out of the wasm builds (`web.worker` never shells out), which is why a crate that builds for wasm (`orchestrator`) uses `REMOTE` and `now()` only behind the feature that brings in `clm-git`.

`REMOTE` is a `pub const REMOTE: &str`, and `fetch` passes it rather than spelling the name, so the name is written once in the workspace's code. `pub fn now() -> u64` reads `SystemTime::now()` and returns 0 when the clock is before 1970; callers that need finer time, such as `orchestrator`'s claim IDs, or a formatted date, such as `ledger`'s records, keep their own. `Failure` is `#[derive(Debug)] pub enum Failure { Refused(String), Git(String) }`, with `Display` printing the string either variant holds, `From<Error>` giving `Git(e.to_string())`, and `exit_code(&self) -> u8` giving 1 for `Refused` and 2 for `Git`; it implements `std::error::Error`.

`run` and `run_within` share one body that takes an optional limit; `run` passes none and waits. With a limit, stdout and stderr are each drained on a detached thread (not a scoped one) and stdin written on a third, from an owned copy of the bytes since the thread may outlive the call, and the caller polls `try_wait` every few milliseconds until the command exits or the limit passes. On exit it joins the readers and returns as `run` does. When the limit passes it calls `kill` and `wait` on the child, so it is reaped, and returns `TimedOut` without joining the readers: a process the command started, such as a credential helper, may still hold the pipes open, and the readers end when it closes them. Only the child is killed, not a process group, since `git` and `gh` are run directly and the portable `std` API kills one process. A zero limit starts nothing and returns `TimedOut`. A limit is a `Duration`, not a deadline, so callers sharing a budget pass what remains of it.

`run_within` is a function beside `run` rather than a `timeout` field of `Options`, since `Options` is built by struct literal in other crates (`bounded-runs`).

The scan test lists the workspace's crates from `Cargo.toml`, keeps those whose own `Cargo.toml` lists `clm-git`, reads each one's `src/` tree, and fails naming the file and line of any `Command::new("git")` or `Command::new("gh")`, so a wrapper that comes back is caught where it appears, not by review.

`extract_tree` and `extract_paths` both read the whole of `rev` into a temporary index (`git read-tree <rev>` with `GIT_INDEX_FILE` set), so checkout reads `rev`'s `.gitattributes` from that index even when the paths asked for don't include them, then check entries out with `checkout-index` and the temporary directory as the work tree: `extract_tree` every entry (`--all`), `extract_paths` only the entries under `paths`, listed from the temporary index (`ls-files -z --full-name` with literal, top-relative pathspecs) and fed to `checkout-index --stdin -z`. Not `git checkout <rev> -- <paths>`: it runs the repository's `post-checkout` hook, and `checkout-index` runs none. `extract_paths` checks each path's form first, and errors naming a path the listing matched no entry for, since `ls-files` matching nothing is not a git error.

**Tracked files.** `tracked_files(root: &Path) -> Result<Vec<PathBuf>, Error>` and `tracked_matching(root: &Path, globs: &[String]) -> Result<Vec<PathBuf>, Error>`, `globs` being a component's `paths` as the model holds them. `tracked_files(root)` runs `git ls-files -z --cached --deduplicate` in `root`; `tracked_matching(root, globs)` runs the same with `--` and one pathspec `:(glob)<glob>` per glob, so a component's `paths` (`**`, `*`, `?` as in its frontmatter) match by git's own glob rules, and returns nothing without running git when `globs` is empty: a component with no `paths` owns no file, not every file. Both split the output on NUL and return each name as a `PathBuf` relative to `root`, in git's order (sorted). `--deduplicate` lists a file the index holds at several merge stages, during a conflict, once. On Unix the path is built from git's bytes with `OsStr::from_bytes`, so a name that isn't UTF-8 comes back exactly and a caller can hand it back to git (`check-attr --stdin -z`); elsewhere it is converted lossily, which CLM accepts since it runs on Linux and macOS. `--deduplicate` needs git 2.31 or later. An error is `run`'s, naming the whole `ls-files` command; callers pass it on as they map any other `Error`. `init`'s `clm survey` and `check`'s `clm size` both list a component's files through these, so they agree on which files a component owns (`tracked-files-in-git-layer`). `check`'s coverage scan, which also lists untracked files, and `sync`'s queries of single paths and merge stages are other questions and keep their own `run` calls.

**Reading a commit** (`tree-blobs`, `common-dir`). Used by `view.repo`'s `--at` reader (`view-at-rev`). `tree_blobs(root: &Path, commit: &str, keep: &dyn Fn(&str) -> bool) -> Result<Vec<(String, Vec<u8>)>, Error>` runs `git ls-tree -r -z --full-tree <commit>` and then, with the IDs of the blobs whose path `keep` accepts on stdin, one `git cat-file --batch`, both with `GIT_NO_LAZY_FETCH=1` so a partial clone never fetches a missing object, and both through `run`. It returns each kept blob's path, relative to the top of the repository whatever directory `root` is, and its bytes as stored (no `core.autocrlf`, `ident` or filter driver is applied: unlike `extract_paths`, it reads objects, not a checkout), in the listing's order. It skips gitlinks (submodules), symbolic links (mode `120000`) and paths that aren't UTF-8 before asking `keep`. A batch answer that isn't `<oid> blob <size>` followed by that many bytes, such as `<oid> missing`, returns `Error::Unreadable { command, why }`, a variant displayed `<command>: <why>` with `why` naming the path and the answer; no caller matches every variant by name. `common_dir(root: &Path) -> Result<PathBuf, Error>` runs `git rev-parse --path-format=absolute --git-common-dir`: the main checkout's `.git` from any worktree of the clone.

**Removing a worktree** (`remove-worktree`). A build can leave an ignored directory without its owner's write permission, for instance by unpacking an archive; `git status` still reads the worktree as clean, but `git worktree remove` unregisters the worktree and then can't delete what that directory holds. `remove_worktree(root: &Path, path: &Path) -> Result<(), Error>` runs `git worktree unlock <path>` first, ignoring its failure, since Claude Code locks the worktrees it makes and unlocking one that isn't locked fails; then gives the owner read, write and execute permission on `path` and every directory under it that lacks them, setting a directory's permission before it reads its entries so one its owner can't list is still walked,, walking with `std::fs::symlink_metadata` and never following or changing a symbolic link, so nothing outside `path` changes; then runs `git worktree remove <path>` through `git`, without `--force`, so a worktree git refuses (one holding an initialised submodule, or a file that isn't ignored) is refused; and returns `StillOnDisk` when git succeeded but `path` still exists. The walk is best effort: what it can't change is left for git to report. It is moved here from `clm-run`, where `clm run --finish` had it, so `clm wrap-up` and `clm runner`, which depend on `clm-git` and not on `clm-run`, remove a worktree the same way (`reuse-first`). It is file-system work beside the git call, and lives in this crate because a worktree is git's: the three callers need nothing else from one another. Each caller prints its own `left` line from the error, since the words are the command's, not the layer's.

## Requirements

### Requirement: one-git-layer
No crate whose `Cargo.toml` lists `clm-git` SHALL run `git` or `gh` as a process. A test in `clm-git` SHALL read the `src/` tree of every such crate and fail naming the file and line of any `Command::new("git")` or `Command::new("gh")` it finds, and SHALL NOT scan a crate that doesn't list `clm-git`.

#### Scenario: a wrapper comes back
- GIVEN a crate under `crates/` that lists `clm-git` and whose source holds `Command::new("git")`
- WHEN the workspace's tests run
- THEN the scan test fails naming that file and line

#### Scenario: a crate that hasn't migrated
- GIVEN a crate under `crates/` that doesn't list `clm-git` and whose source holds `Command::new("git")`
- WHEN the scan test runs
- THEN it passes, and its output names the crate as not yet covered

#### Scenario: clean dependants
- GIVEN every crate listing `clm-git` free of such calls
- WHEN the scan test runs
- THEN it passes

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
`git(root, args)` and `gh(root, args)` SHALL run the command in `root` as resolved on `PATH`, return its stdout with trailing whitespace trimmed on exit 0, and otherwise return an error whose display names the command and holds its stderr, or its exit status when stderr is empty.

#### Scenario: success
- GIVEN a git repository at `root`
- WHEN `git(root, &["rev-parse", "--abbrev-ref", "HEAD"])` runs on branch `main`
- THEN it returns `main`

#### Scenario: failure carries stderr
- GIVEN a directory that isn't a git repository
- WHEN `git(root, &["status"])` runs
- THEN it returns an error whose display holds `git status` and `not a git repository`

#### Scenario: fake gh on PATH
- GIVEN a directory first on `PATH` holding an executable `gh` that prints `{"state":"MERGED"}`
- WHEN `gh(root, &["pr", "view", "1"])` runs
- THEN it returns `{"state":"MERGED"}`

### Requirement: repo-queries
`fetch(root)` SHALL run `git fetch origin`. `is_ancestor(root, a, b)` SHALL be true exactly when `git merge-base --is-ancestor a b` exits 0. `current_branch(root)` SHALL return the checked-out branch's name, or `None` on a detached `HEAD`. `worktrees(root)` SHALL list every worktree of the repository with its path, its branch (`None` when detached) and whether it is locked. `extract_tree(root, rev)` SHALL return a temporary directory holding `rev`'s tree, leaving the working tree and the index unchanged.

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
`run_within(root, program, args, options, limit)` SHALL behave as `run(root, program, args, options)` when the command exits within `limit`. When it is still running once `limit` has passed, it SHALL kill the command's process, wait for it to exit, and return `Error::TimedOut { command, limit }`, whose display is `<command>: timed out after <limit>`, with the limit in `Duration`'s debug form. It SHALL return within one second after `limit` passes even when a process the command started keeps the command's stdout or stderr open. A zero `limit` SHALL return the timed-out error without starting the command, so a caller whose budget is spent passes what remains and starts nothing. `run` SHALL keep waiting with no limit.

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
`clm-git` SHALL export `REMOTE`, a `&str` equal to `"origin"`, and `fetch(root)` SHALL fetch the remote `REMOTE` names.

#### Scenario: the remote's name
- GIVEN the crate
- WHEN a test reads `clm_git::REMOTE`
- THEN it is `origin`

#### Scenario: fetch uses it
- GIVEN a repository whose remote `origin` has a branch `b` the clone hasn't fetched
- WHEN `fetch(root)` runs
- THEN `refs/remotes/origin/b` exists afterwards

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
`tracked_files(root)` SHALL return every file the index holds under `root`, as `git ls-files --cached` run in `root` names it, each once even when the index holds it at several merge stages. `tracked_matching(root, globs)` SHALL return those of them that any glob matches as the pathspec `:(glob)<glob>`, and SHALL return nothing, running no git, when `globs` is empty. On Unix each returned path's bytes SHALL be exactly the name git lists, whether or not it is UTF-8.

#### Scenario: globs match by git's rules
- GIVEN a repository tracking `src/a/x.rs`, `src/a/b/y.rs` and `src/b/z.rs`
- WHEN `tracked_matching(root, ["src/a/**"])` runs
- THEN it returns `src/a/b/y.rs` and `src/a/x.rs`, and not `src/b/z.rs`

#### Scenario: no globs runs no git
- GIVEN a directory that isn't a git repository
- WHEN `tracked_matching(root, [])` runs
- THEN it returns an empty list, not an error

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
`tree_blobs(root, commit, keep)` SHALL return, in the order `git ls-tree -r` lists them, the path from the top of the repository and the bytes as stored of every blob of `commit`'s tree whose path `keep` accepts, skipping gitlinks, symbolic links and paths that aren't UTF-8, reading the objects with one `git ls-tree` and one `git cat-file --batch` without fetching and without writing any file, the working tree and the index unchanged, whatever directory of the repository `root` is. A kept blob missing from the local objects SHALL make it return an error whose display names the blob's path. A `commit` that names no commit SHALL make it return an error.

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
`files_to_commit(root: &Path) -> Result<Vec<PathBuf>, Error>` SHALL return every file the index holds under `root` and every untracked file under it that git's standard excludes don't ignore (`.gitignore` files, `.git/info/exclude` and `core.excludesFile`), as `git ls-files -z --cached --others --exclude-standard --deduplicate` run in `root` names them, each once, so a caller sees the files a `git add -A` would commit. A file deleted from the work tree that the index still holds is listed, as `tracked_files` lists it. On Unix each returned path's bytes SHALL be exactly the name git lists, whether or not it is UTF-8. `tracked_files` and `tracked_matching` keep reading the index alone.

#### Scenario: tracked and untracked, not ignored
- GIVEN a repository tracking `src/a.rs` and `.gitignore` holding `build/`, with the untracked files `notes.txt` and `build/out.o`
- WHEN `files_to_commit(root)` runs
- THEN it returns `.gitignore`, `notes.txt` and `src/a.rs`, and not `build/out.o`

#### Scenario: a file both staged and changed once
- GIVEN a repository tracking `src/a.rs`, changed in the work tree since it was staged
- WHEN `files_to_commit(root)` runs
- THEN it lists `src/a.rs` exactly once

#### Serves
- capability/adopt-clm

### Requirement: remove-worktree
`remove_worktree(root: &Path, path: &Path) -> Result<(), Error>` SHALL remove the linked worktree at `path` whole, ignored files included: it SHALL unlock it, ignoring the failure for a worktree that isn't locked; give the owner read, write and execute permission on `path` and on every directory under it that lacks them, setting a directory's permission before it reads its entries; and then run `git worktree remove <path>` through `git`, without `--force`. It SHALL never follow or change a symbolic link, so no permission outside `path` changes. It SHALL return `git`'s error when `git worktree remove` fails, and `Error::StillOnDisk`, whose display holds the path and `still on disk`, when it succeeds but `path` still exists. It SHALL NOT judge whether the worktree has uncommitted changes, which is its caller's question.

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

#### Serves
- capability/work-concurrently

### Requirement: git-isolated
`git_isolated(dir, args)` SHALL run `git` with `args` in `dir` and return what `git(dir, args)` returns there, its stdout trimmed or the same error, except that each environment variable `git rev-parse --local-env-vars` prints SHALL be removed from the command's environment, whatever the calling process has set. The variables SHALL be the ones git lists, not a list written in the code.

#### Scenario: a bare repository beside a clone the environment names
- GIVEN a work tree `w` whose git directory `g` is elsewhere (`git init --separate-git-dir g w`), and `git` started with `GIT_DIR=g` and `GIT_WORK_TREE=w` in its environment, as from a process that has them set
- WHEN `git_isolated(w, &["init", "--quiet", "--bare", b])` runs, `b` an absolute path with nothing there
- THEN it succeeds, `b` is a bare repository (`git --git-dir b rev-parse --is-bare-repository` prints `true`), and `g` is unchanged; `git(w, ...)` with the same arguments in that environment fails with git's `GIT_WORK_TREE ... not allowed without specifying GIT_DIR`

#### Scenario: the same as git otherwise
- GIVEN a git repository at `root` on branch `main`, and no git variable set
- WHEN `git_isolated(root, &["rev-parse", "--abbrev-ref", "HEAD"])` and `git_isolated(root, &["no-such-command"])` run
- THEN the first returns `main`, and the second an error whose display holds `git no-such-command`

### Requirement: git-follows-its-clone
Every `git` command `clm-git` runs, through `git`, `run` or `run_within` with the program `git`, or any function built on them, SHALL run as `git_isolated` runs it, with each variable `git rev-parse --local-env-vars` prints removed from its environment, when its `root` lies outside the work tree the calling process's environment names; a variable the caller sets in `Options::env` SHALL be kept, even one git lists. The work tree the environment names SHALL be the directory `git rev-parse --show-toplevel` prints when run, not through this rule, with the process's environment in the process's current directory, found only when the environment sets `GIT_DIR` or `GIT_WORK_TREE` and found again only when either variable's value has changed since. `root` SHALL lie outside it when `root`, with symbolic links resolved, is neither that directory nor under it, or when `root` or a directory between it and that work tree holds a `.git` entry (a file or a directory), as a linked worktree or another repository made inside the work tree, such as `clm land`'s kept tree under `.clm/`, does. When the environment sets neither variable, when that `rev-parse` fails (a bare repository names no work tree), and for a `root` inside the work tree, the command SHALL run with the environment as the process has it. A program other than `git` SHALL run with the environment as the process has it.

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
