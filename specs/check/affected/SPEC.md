---
title: Affected
status: active
owners: ["@clunny"]
depends_on: [model, check, check.coverage, ledger, vcs]
paths: ["crates/clm-affected/**"]
successor: ""
serves: [capability/land-safely]
---

## Contract
`clm affected [--base <ref>] [--root <dir>] [--readers | --why <path> | --overlap <a> <b>]` prints what a change can break, read from the spec graph (`graph-test-selection`): the components it affects and the files whose tests a check should run, or that every test must run. It knows no test runner; the project's check command turns the files into its runner's arguments (`ci`), and `clm land` hands it the output (`land`).

- The *changed files* are the paths that differ between the merge base of `<ref>` (default `origin/main`) and `HEAD`, and the working tree, with the untracked files git doesn't ignore, leaving out every doc-only path (`ledger/doc-only`). A deleted file is a changed file.
- When a changed file matches a glob of `config`'s `[affected] all`, or no component's `paths` hold it (`check.coverage/owner`), the output is one line, `all <file>`, naming the first such file in path order.
- Otherwise the *affected* components are the owners of the changed files and every component that depends on one of them, directly or indirectly (`depends_on`). The output is a *component line* for each affected component, `node <id>`, the word `component-words` keeps for these lines, which `ci`'s check script and `clm land` read, then `file <path>` for each selected file, each group sorted, and nothing when no file changed.
- The *selected* files are the changed files that still exist, whatever their content, the files `check`'s coverage scan reads (tracked, or untracked and not ignored; UTF-8; Markdown left out) that hold a `req:` marker whose requirement ID names an affected component (the part before `/`, whether or not the requirement still exists), or that hold no `req:` marker and whose owner is affected, and the *readers*.
- A *reader* is a file the coverage scan reads that holds a `clm:reads <glob>` marker (`doc-readers-marked`) whose glob matches a path that differs as the changed files do, doc-only paths included: a test that reads the repository's own files, such as its specs or `ARCHITECTURE.md`, names them so, since nothing else connects the test to them. `--readers` prints the readers' `file` lines alone, with no component or `all` line, loading neither the graph nor the settings, which `clm land` runs on a tree whose code was already checked.
- `--why <path>` prints why the change selects `<path>`, as the chain from the file to a changed file, one link per line (`why-chain`). `clm land` runs it to explain a step's cost (`land/check-time-report`).
- `--overlap <a> <b>` prints what the changes of two commits can both break, each against their merge base and read from the commits alone (`overlap-selection`), which `clm land` runs when `main` moved under a checked branch (`land/overlap-recheck`).

It exits 0 with its output; 1 with the reason when two of `--readers`, `--why` and `--overlap`, or `--overlap` and `--base`, are given together, when a ref or revision doesn't resolve to a commit, when the two sides have no merge base, or when the graph or the settings can't be loaded; and 2 when git fails otherwise. A reason of its own starts `clm affected: `, and nothing is on standard output when it exits non-zero.

`clm-affected` offers `clm affected` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup, what loads the `[affected]` settings and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes. As library calls it offers too `affected(root, base, all)`, the selection as a value, which `verify` reads; `readers(differing, reads)` and `reads_in(text)`, the globs of the `clm:reads` markers in a file's text, which `verify` reads to name the files a requirement's tests read (`verify/mutation-pass`); and `overlap(a, b)`, the rule of `overlap-selection` for two selections, with `affected_between(root, from, to, all)`, the selection between two commits.

## Design
Crate `clm-affected`. `select(graph, changed, markers, all) -> Selection` is pure: it takes the graph (`model`), the changed paths with whether each still exists, the markers of every scanned file (`check.coverage`'s `scan`, each with its path) and the `[affected] all` globs, and returns `All(path)` or `Selection::Nodes`, holding the affected components' IDs and the selected files. Owners and globs are matched with `check.coverage`'s `owner` and `matches`, so a file belongs to the same component as in `clm pending`; paths are disjoint across components, and `owner`'s deepest-first rule settles any overlap. A `clm:remove-by` marker isn't a `req:` marker: a file holding only those belongs to its owner, and neither is a `clm:reads` marker. Dependants are found by walking `depends_on` backwards from the changed components until no component is added. A file an exemption names for `undeclared-imports` is selected whenever any component is affected, since what it imports is outside the graph's view (`temporary-exemptions`). Doc-only paths come from `ledger`'s `doc_only`, the one list `code_key` uses.

**Readers.** `readers(differing, reads) -> Vec<path>` is pure too: it takes every differing path, doc-only ones included, with whether it still exists, and each scanned file's `clm:reads` globs, and returns the files, sorted, one of whose globs matches a differing path by `check.coverage`'s `matches`, so a glob reads as a component's `paths` do. A marker is the text `clm:reads ` and the glob after it, up to the next whitespace; a file may hold several, and a marker with no glob after it is ignored. A reader is selected whether or not its owner is affected, and a deleted reader isn't printed. The markers come from the files the coverage scan reads, so a Markdown file is never a reader. Every file of this component, source and doc comments as well as test fixtures, builds or writes the marker's text so the literal and a space never appear whole (`concat!("clm", ":reads specs/**")`), as fixtures do `req:` markers, or the file would be a reader of whatever follows; so does any other file that only mentions the marker. `--readers` skips the graph, the owners and `[affected] all`, since its question is only which tests read what changed.

`affected(root, base, all)` gathers the inputs through `git` (`git-plumbing-layer`): `git rev-parse --verify --quiet <base>^{commit}`, whose status 1 is a ref that names no commit, `git merge-base <base> HEAD`, which has none when it prints nothing with status 1, then `git diff --name-only --no-renames -z <merge base>` (the working tree against the merge base, so a rename counts as its old and new path) and `git ls-files --others --exclude-standard -z`. A changed path is matched as git lists it, relative to the repository root.

`crates/clm` routes `clm affected`, loading `config`'s `affected` and passing the loader in, as it passes `detectors` to `clm check`, so this crate doesn't depend on `clm-config`. The stand-in loader in this crate's tests words its line as the root does. Fixture graphs in the tests give their root component no `depends_on`, so the root is affected only when a scenario says so.

## Requirements

### Requirement: affected-files
`clm affected` SHALL print, after the components, `file <path>` for each changed file that still exists, and for each file `check`'s coverage scan reads that holds a `req:` marker, inside a test or outside one, whose requirement ID names an affected component, or that holds no `req:` marker at all and whose owner is affected, and for each reader (`affected-readers`), and, whenever any component is affected, for each file an exemption of its owner names for `undeclared-imports` (`model/exemptions`), since it imports code the graph doesn't connect it to and so can break from a change anywhere (`land-checks-stay-fast`, `temporary-exemptions`), sorted by path, and for no other file.

#### Scenario: cited, owned and changed
- GIVEN the components of `affected-components` and a component `e` depending on nothing with `paths: ["crates/e/**"]`; `crates/e/tests/x.rs` citing a requirement of `c` and one of `d`; `crates/e/tests/y.rs` citing only `d`; `crates/a/src/util.rs` with no marker; `crates/d/src/lib.rs` with no marker; and a branch that changes `crates/a/src/lib.rs` and `crates/e/tests/z.rs`, which cites only `d`
- WHEN `clm affected --base main` runs
- THEN the `file` lines are `crates/a/src/lib.rs`, `crates/a/src/util.rs`, `crates/e/tests/x.rs` and `crates/e/tests/z.rs`, and not `crates/e/tests/y.rs` or `crates/d/src/lib.rs`

#### Scenario: marker naming a removed requirement
- GIVEN `crates/e/tests/old.rs` citing `b/gone`, which no longer exists, and a branch that affects `b`
- WHEN `clm affected --base main` runs
- THEN it prints `file crates/e/tests/old.rs`

#### Scenario: uncommitted, untracked and deleted
- GIVEN a clean branch equal to `main`, then an uncommitted edit to `crates/a/src/lib.rs`, a new untracked, unignored file `crates/a/src/new.rs`, and `crates/a/src/old.rs` deleted
- WHEN `clm affected --base main` runs
- THEN it prints the component line of `a`, `file crates/a/src/lib.rs` and `file crates/a/src/new.rs`, and no line for `crates/a/src/old.rs`

#### Scenario: a marker outside a test still selects
- GIVEN `tests/t.ts` owned by `b`, whose only marker is a file-header comment citing `a/x`, and a change to `a`'s code
- WHEN `clm affected` runs
- THEN it prints `file tests/t.ts`

#### Needs
- check.coverage/test-spans
- model/exemptions
- check.coverage/owner

### Requirement: affected-all
When a changed file matches a glob of the `[affected] all` setting, or lies in no component's `paths`, `clm affected` SHALL print exactly one line, `all <path>`, naming the first such file in path order, and exit 0.

#### Scenario: file no node holds
- GIVEN a branch that changes `crates/a/src/lib.rs` and `tools/build.sh`, which no component's `paths` hold
- WHEN `clm affected --base main` runs
- THEN it prints only `all tools/build.sh`

#### Scenario: setting's glob
- GIVEN a repository `clm.toml` with `[affected]` and `all = ["Cargo.lock"]`, the root component's `paths` holding `Cargo.lock`, and a branch that changes only `Cargo.lock`
- WHEN `clm affected --base main` runs
- THEN it prints only `all Cargo.lock`

#### Scenario: deleted file no node holds
- GIVEN a branch that deletes `tools/build.sh`, which no component's `paths` hold
- WHEN `clm affected --base main` runs
- THEN it prints only `all tools/build.sh`

#### Needs
- check.coverage/owner
- config/affected-setting

### Requirement: doc-only-selects-nothing
`clm affected` SHALL leave out of the changed files every doc-only path (`ledger/doc-only`), so a change only to those paths prints only the `file` lines of their readers (`affected-readers`), and nothing when no reader reads them, and a change to those paths and to code prints what the code alone would and those readers.

#### Scenario: docs and backlog only
- GIVEN a branch that changes only `docs/decisions/x.md`, `backlog/y.md` and `specs/a/SPEC.md`, and no file holding a `clm:reads` marker
- WHEN `clm affected --base main` runs
- THEN it prints nothing and exits 0

#### Scenario: docs beside code
- GIVEN a branch that changes `backlog/y.md` and `crates/d/src/lib.rs`, nothing depends on `d`, and no file holding a `clm:reads` marker
- WHEN `clm affected --base main` runs
- THEN it prints the component line of `d` and `d`'s files, and no `all` line

#### Needs
- ledger/doc-only

### Requirement: affected-base
`clm affected` SHALL compare the working tree, untracked files included, with the merge base of `--base <ref>` and `HEAD`, defaulting `<ref>` to `origin/main`. It SHALL exit 1 writing `clm affected: unknown base <ref>` to standard error when `<ref>` doesn't resolve to a commit, and `clm affected: no merge base between <ref> and HEAD` when it has no merge base with `HEAD`; 1 with the error when the graph can't be loaded; and 1 when its caller can't load the `[affected]` settings (`config/affected-setting`), writing the line the caller gives for the failure to standard error, which names the file and the key and is the root's to print (`clm/routes-only`). With `--readers` it SHALL load neither the graph nor the settings, so neither failure stops it. When git fails in any other way it SHALL exit 2 with git's error on standard error. Whenever it exits non-zero it SHALL write nothing to standard output.

#### Scenario: main moved on
- GIVEN a branch that changes `crates/a/src/lib.rs`, and `main` that has since changed `crates/d/src/lib.rs`
- WHEN `clm affected --base main` runs on the branch
- THEN it prints the component line of `a` and none of `d`

#### Scenario: unknown base
- GIVEN a repository with no ref `nope`
- WHEN `clm affected --base nope` runs
- THEN it exits 1 and standard error holds `clm affected: unknown base nope`

#### Scenario: invalid graph
- GIVEN a repository whose `specs/` has a dependency cycle
- WHEN `clm affected --base main` runs
- THEN it exits 1 with the error and prints no component, `file` or `all` line

#### Scenario: settings that can't be loaded
- GIVEN a caller whose loading of the settings fails with a line
- WHEN `clm-affected`'s call for `clm affected --base main` runs
- THEN it returns 1, writes that line to standard error and nothing to standard output

#### Scenario: no merge base
- GIVEN a branch `lonely` that shares no commit with `HEAD`
- WHEN `clm affected --base lonely` runs
- THEN it exits 1 and standard error holds `clm affected: no merge base between lonely and HEAD`

#### Scenario: git fails
- GIVEN a directory holding `specs/SPEC.md` that is no git repository
- WHEN `clm affected --base main` runs there
- THEN it exits 2, with git's error on standard error and nothing on standard output

#### Scenario: readers with a graph and settings that can't be loaded
- GIVEN reader files, a dependency cycle in `specs/`, a `clm.toml` whose `[affected] all` is no list, and a branch that changes a path the readers read
- WHEN `clm affected --base main --readers` runs
- THEN it prints the readers' `file` lines and exits 0

### Requirement: git-through-layer
`clm-affected` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `run` or repo queries (`git-plumbing-layer`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-affected` reads `crates/clm-affected/Cargo.toml` and every file under `crates/clm-affected/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")` or `Command::new("gh")`

#### Serves
- architecture/reuse-first

### Requirement: affected-components
`clm affected` SHALL print a *component line*, `node <id>`, for each component that owns a changed file (`check.coverage/owner`) and for each component that depends on one of those, directly or indirectly, sorted by ID and before any `file` line, and for no other component. The line keeps the word `node` (`component-words`).

#### Scenario: dependants
- GIVEN a root with no dependencies, components `a`, `b` depending on `a`, `c` depending on `b`, and `d` depending on nothing, and a branch that changes only `crates/a/src/lib.rs`, which `a`'s `paths` hold
- WHEN `clm affected --base main` runs on the branch
- THEN it prints `node a`, `node b` and `node c`, and no other component line

#### Scenario: nothing changed
- GIVEN a branch with no commits beyond `main` and a clean working tree
- WHEN `clm affected --base main` runs
- THEN it prints nothing and exits 0

#### Needs
- check.coverage/owner

### Requirement: affected-readers
`clm affected` SHALL treat as a *reader* each file `check`'s coverage scan reads that holds the text `clm:reads ` followed by a glob, up to the next whitespace, matching, as component `paths` match, a path that differs between the merge base and the working tree, doc-only paths and untracked files included. It SHALL print `file <path>` for each reader that still exists among its other `file` lines, sorted with them, whether or not the reader's owner is affected, and given `--readers` SHALL print those lines alone, with no component or `all` line, and nothing when no reader matches.

#### Scenario: reader of a doc-only file
- GIVEN `crates/e/tests/r.rs` holding `clm:reads ARCHITECTURE.md`, `crates/e/tests/s.rs` holding `clm:reads specs/**`, nothing depending on `e`, and a branch that changes only `ARCHITECTURE.md`
- WHEN `clm affected --base main` runs
- THEN it prints only `file crates/e/tests/r.rs`

#### Scenario: readers beside code
- GIVEN the same files and a branch that changes `ARCHITECTURE.md` and `crates/a/src/lib.rs`
- WHEN `clm affected --base main` runs
- THEN it prints the component lines and `file` lines a change to `crates/a/src/lib.rs` alone prints, and `file crates/e/tests/r.rs` among the `file` lines

#### Scenario: readers alone
- GIVEN the same files and branch
- WHEN `clm affected --base main --readers` runs
- THEN it prints only `file crates/e/tests/r.rs`

#### Scenario: no reader matches
- GIVEN the same files and a branch that changes only `backlog/y.md` and `tools/build.sh`, which no component's `paths` hold
- WHEN `clm affected --base main --readers` runs
- THEN it prints nothing and exits 0

#### Scenario: a marker in Markdown
- GIVEN `docs/notes.md` holding `clm:reads ARCHITECTURE.md` and a branch that changes only `ARCHITECTURE.md`
- WHEN `clm affected --base main --readers` runs
- THEN it prints nothing

#### Needs
- check.coverage/owner
- ledger/doc-only

### Requirement: why-chain
`clm affected --why <path> [--base <ref>]` SHALL print the chain by which the change selects `<path>`, one link per line, from `<path>` to a changed file, taking the first reason that holds in this order: `<path> (changed)` alone when `<path>` is a changed file that still exists; `<path> reads <changed path>` when it is a reader, naming the first such path in path order; `<path> cites <component>` when its `req:` marker names an affected component, the first by ID; `<path> owned by <component>` when its owner is affected; and `<path> exempt from undeclared-imports` otherwise, when it is selected only as a file an exemption names for that rule (`affected-files`), with no line after it. After a `cites` or `owned by` line come the edges of the shortest path from that component to a component that owns a changed file, choosing among equally short paths the first in ID order: the path follows `depends_on` edges, each printed `<x> depends on <y>`, and then `<y> owns <changed path> (changed)`, naming that component's first changed path in path order. It SHALL print `<path> not selected` for a path the selection doesn't hold, a deleted changed file among them, and `all: <file>` when the selection is `all <file>`, exiting 0 in each case and as the selection does otherwise. Its lines SHALL say component, never node (`component-words`).

#### Scenario: through a dependency
- GIVEN components `a`, owning `a/src/lib.rs`, and `b`, owning `b/src/lib.rs`, with `a` depending on `b`, and a branch that changes only `b/src/lib.rs`
- WHEN `clm affected --why a/src/lib.rs` runs
- THEN it prints `a/src/lib.rs owned by a`, `a depends on b` and `b owns b/src/lib.rs (changed)`, and exits 0

#### Scenario: a changed file
- GIVEN the same graph and branch
- WHEN `clm affected --why b/src/lib.rs` runs
- THEN it prints `b/src/lib.rs (changed)` alone

#### Scenario: not selected
- GIVEN the same graph, a component `c` owning `c/src/lib.rs` that depends on nothing, and the same branch
- WHEN `clm affected --why c/src/lib.rs` runs
- THEN it prints `c/src/lib.rs not selected` and exits 0

#### Scenario: both flags
- GIVEN the same graph and branch
- WHEN `clm affected --readers --why a/src/lib.rs` runs
- THEN it exits 1 naming both flags

#### Scenario: an exempt test
- GIVEN components `app`, owning `app/src/lib.rs`, depending on `feat`, and `feat`, owning `feat/tests/t.rs`, which it exempts from `undeclared-imports` until `backlog/x`, and a branch that changes only `app/src/lib.rs`
- WHEN `clm affected --why feat/tests/t.rs` runs
- THEN it prints `feat/tests/t.rs exempt from undeclared-imports` and exits 0

#### Needs
- model/exemptions

### Requirement: overlap-selection
`clm affected --overlap <a> <b>` SHALL print what both changes can break. With `<m>` the merge base of the commits `<a>` and `<b>` name, each *side* is the selection, readers included, of the paths that differ between `<m>` and that side's commit, computed as `affected-files`, `affected-all`, `affected-components` and `affected-readers` compute it but from the two commits alone, with no working tree and no untracked file: each side's changed paths are those that differ between `<m>` and its commit, a rename counting as its old path and its new one, and its markers (`req:` and `clm:reads`) and the files the coverage scan would read are read from that side's commit, not from any checkout. Its output SHALL be:
- when both sides are `all`, the first side's `all` line;
- when one side is `all`, the other side's output unchanged;
- otherwise the component lines of the components both sides affect, then the `file` lines of the files both sides select, each group sorted as `affected-components` sorts it, and nothing at all when no file is in both.

`--overlap` with `--base`, `--readers` or `--why` SHALL exit 1 naming both flags. A `<a>` or `<b>` that doesn't resolve to a commit SHALL exit 1 with `clm affected: unknown revision <rev>`, two commits with no merge base SHALL exit 1 with `clm affected: no merge base between <a> and <b>`, and any other git failure SHALL exit 2, as `affected-base` says for `--base`.

#### Scenario: independent changes
- GIVEN components `a` and `b` with no `depends_on` between them, owning `crates/a/**` and `crates/b/**`, a commit `m`, and commits `x` and `y` on top of `m`, `x` changing `crates/a/src/lib.rs` and `y` changing `crates/b/src/lib.rs`
- WHEN `clm affected --overlap x y` runs
- THEN it prints nothing and exits 0

#### Scenario: a dependant of both
- GIVEN the same, and a component `c` owning `crates/c/**` with `depends_on: [a, b]` and a test `crates/c/tests/t.rs`
- WHEN `clm affected --overlap x y` runs
- THEN it prints the component line for `c` and `file crates/c/tests/t.rs`, and no line for `a` or `b`

#### Scenario: one side selects everything
- GIVEN `[affected] all = ["Cargo.lock"]`, `x` changing `Cargo.lock` and `y` changing `crates/b/src/lib.rs`
- WHEN `clm affected --overlap x y` runs
- THEN it prints exactly what `clm affected --base m` prints with `y` checked out

#### Scenario: a reader on one side, its owner on the other
- GIVEN `x` changing `specs/a/SPEC.md` and `y` changing `crates/a/src/lib.rs`, and `crates/a/tests/spec.rs` holding a `clm:reads specs/**` marker
- WHEN `clm affected --overlap x y` runs
- THEN its output holds `file crates/a/tests/spec.rs`

#### Scenario: flags that don't combine
- GIVEN any repository
- WHEN `clm affected --overlap x y --readers` runs
- THEN it exits 1 naming `--overlap` and `--readers`

### Requirement: commands-in-crate
`clm-affected` SHALL offer `clm affected` as a call (the Contract's) that takes the command's arguments as the command line gives them, the repository root its caller found, which the command's `--root` overrides, the environment as a lookup, what loads the `[affected]` settings, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2.

#### Scenario: a command called in a test
- GIVEN a fixture repository
- WHEN the call is given the arguments of `clm affected` and two buffers
- THEN the buffers hold what `clm affected` prints to standard output and to standard error there, and the call returns the status `clm affected` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN it is given an option `clm affected` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the caller's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
