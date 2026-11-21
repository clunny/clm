---
title: Affected
status: active
owners: ["@clunny"]
depends_on: [model, check, check.coverage, ledger, vcs, config]
paths: ["crates/clm-affected/**"]
successor: ""
serves: [capability/land-safely]
---

## Contract
`clm affected [--root <dir>] [[--base <ref>] [--readers | --why <path>] | --overlap <a> <b>]` prints what a change can break, read from the spec graph (`graph-test-selection`): the components it affects and the files whose tests a check should run, or that every test must run. It knows no test runner; the project's check command turns the files into its runner's arguments (`ci`), and `clm land` hands it the output (`land`).

- The *changed files* are the paths that differ between the merge base of `<ref>` (default `origin/main`) and `HEAD`, and the working tree, with the untracked files git doesn't ignore, leaving out every doc-only path (`ledger/doc-only`). A deleted file is a changed file, and so is a changed submodule or nested git repository. Every path it reads or prints, in each mode, is under the CLM root and named from it, as component `paths` are, though the CLM root be a subdirectory of the git repository (`clm-root-paths`).
- When a changed file matches a glob of `config`'s `[affected] all`, or no component's `paths` hold it (`check.coverage/owner`), the output is one line, `all <file>`, naming the first such file in path order.
- Otherwise the *affected* components are the owners of the changed files and every component that depends on one of them, directly or indirectly (`depends_on`). The output is a *component line* for each affected component, `node <id>`, the word `component-words` keeps for these lines, which `ci`'s check script and `clm land` read, then `file <path>` for each selected file, each group sorted, and nothing when no file changed.
- The *selected* files are the changed files that are still a file in the working tree, a symbolic link to one included, whatever their content; the files `check`'s coverage scan reads (tracked, or untracked and not ignored; UTF-8; Markdown left out) that hold a `req:` marker whose requirement ID names an affected component (the part before `/`, whether or not the requirement still exists), or that hold no `req:` marker and whose owner is affected; the *readers*; and, whenever any component is affected, every file the scan lists (tracked, or untracked and not ignored; Markdown left out) that an exemption of its owner names for `undeclared-imports`, whether or not it is UTF-8.
- A *reader* is a file the coverage scan reads that holds a `clm:reads <glob>` marker (`doc-readers-marked`) whose glob matches a path that differs as the changed files do, doc-only paths included: a test that reads the repository's own files, such as its specs or `ARCHITECTURE.md`, names them so, since nothing else connects the test to them. `--readers` prints the readers' `file` lines alone, with no component or `all` line, loading neither the graph nor the settings, which `clm land` runs on a tree whose code was already checked.
- `--why <path>` prints why the change selects `<path>`, as the chain from the file to a changed file, one link per line (`why-chain`), which the library offers as `why(root, base, path, all)`. `clm land` runs it to explain a step's cost (`land/check-time-report`).
- `--overlap <a> <b>` prints what the changes of two commits can both break, each against their merge base: the call computes each side from its commit alone, its paths, graph, markers, files and `[affected] all` globs, and reads neither the checkout's graph nor its settings (`overlap-selection`), which `clm land` runs when `main` moved under a checked branch (`land/overlap-recheck`).

It exits 0 with its output, and 0 with the usage on standard output for `--help` or `-h`; 1 with the reason when two of `--readers`, `--why` and `--overlap`, or `--overlap` and `--base`, are given together, when a ref or revision doesn't resolve to a commit, when the two sides have no merge base, or when the graph or the settings can't be loaded (with `--overlap`, either commit's graph or `clm.toml`, or another layer that loading reads, such as the clone's `clm.local.toml`); 2 with the usage error on standard error for arguments it can't parse; and 2 when git fails otherwise. A reason of its own starts `clm affected: `, and nothing is on standard output when it exits non-zero.

`clm-affected` offers `clm affected` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the CLM root, the environment as a lookup, what loads the `[affected]` settings and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes. Its argument definitions are `Args`, which the root's parser reads. As library calls it offers too `affected(root, base, all)`, which gives the selection as a `Selection`, everything or the affected components and the selected files, which `verify` reads; `readers(differing, reads)`, over paths that differ, each a `Changed`, which the root's test of `clm/doc-readers-marked` reads; `reads_in(text)`, the globs of the `clm:reads` markers in a file's text, which `verify` reads to name the files a requirement's tests read (`verify/mutation-pass`); and `overlap(a, b)`, the rule of `overlap-selection` for two selections, with `affected_between(root, from, to, all)`, the selection between two commits.

## Design
Crate `clm-affected`. `select(graph, changed, markers, all) -> Selection` is pure: it takes the graph (`model`), the changed paths with whether each still exists, the markers of every scanned file (`check.coverage`'s `scan`, each with its path) and the `[affected] all` globs, and returns `All(path)` or `Selection::Nodes`, holding the affected components' IDs and the selected files. Owners and globs are matched with `check.coverage`'s `owner` and `matches`, so a file belongs to the same component as in `clm pending`; paths are disjoint across components, and `owner`'s deepest-first rule settles any overlap. A `clm:remove-by` marker isn't a `req:` marker: a file holding only those belongs to its owner, and neither is a `clm:reads` marker. Dependants are found by walking `depends_on` backwards from the changed components until no component is added. A file an exemption names for `undeclared-imports` is selected whenever any component is affected, since what it imports is outside the graph's view (`temporary-exemptions`); it is looked for among every file the scan lists, Markdown left out, a file that isn't UTF-8 included: the scan keeps the paths of the listed files it can't read as UTF-8 apart from the ones it reads, and `select` takes them for this rule alone, so such a file is never selected by its owner or a citation, which need its text. A Markdown file is left out because it imports nothing, so an exemption excuses nothing in it (`check/undeclared-imports` reads code). Doc-only paths come from `ledger`'s `doc_only`, the one list `code_key` uses.

**Readers.** `readers(differing, reads) -> Vec<path>` is pure too: it takes every differing path, doc-only ones included, with whether it still exists, and each scanned file's `clm:reads` globs, and returns the files, sorted, one of whose globs matches a differing path by `check.coverage`'s `matches`, so a glob reads as a component's `paths` do. A marker is the text `clm:reads ` and the glob after it, up to the next whitespace; a file may hold several, and a marker with no glob after it is ignored. A reader is selected whether or not its owner is affected, and a deleted reader isn't printed. The markers come from the files the coverage scan reads, so a Markdown file is never a reader. Every file of this component, source and doc comments as well as test fixtures, builds or writes the marker's text so the literal and a space never appear whole (`concat!("clm", ":reads specs/**")`), as fixtures do `req:` markers, or the file would be a reader of whatever follows; so does any other file that only mentions the marker. `--readers` skips the graph, the owners and `[affected] all`, since its question is only which tests read what changed.

`affected(root, base, all)` gathers the inputs through `git` (`git-plumbing-layer`): `git rev-parse --verify --quiet <base>^{commit}`, whose status 1 is a ref that names no commit, `git merge-base <base> HEAD`, which has none when it prints nothing with status 1, then `git diff --name-only --no-renames -z <merge base>` (the working tree against the merge base, so a rename counts as its old and new path) and `git ls-files --others --exclude-standard -z`. The diff runs with `--relative` from the CLM root, so a changed path is named from the CLM root, as `ls-files` names the untracked and scanned ones, and a path outside it is left out (`clm-root-paths`). A changed path still exists when it is a file in the working tree with links followed, so a submodule, a nested repository, which `ls-files` names with a trailing `/`, and a link to a directory are changed paths that never get a `file` line.

`affected_between(root, from, to, all)` diffs the two commits with `--relative` too, keeps of `clm_git::tree_blobs`' paths, which start at the top of the repository, only those under `git rev-parse --show-prefix` (its trailing newline trimmed) and strips that prefix from them, in the `keep` filter as well as in what it returns, so a changed Markdown file a reader reads is matched by its name from the CLM root, as `clm land`'s deleted-tests check reads a revision, and loads the graph from the prefix's directory in the tree `extract_tree` gives. `tree_blobs` leaves out symbolic links, so `affected_between` lists `to`'s links under the prefix through `clm-git` too (mode `120000`) and judges each in the tree `extract_tree` writes for `to`, which it extracts before the scan: a link that `is_file` follows there to a file counts as a file that exists and is scanned with the text read through it, as `--base` judges and reads a link in a checkout. So a link to a link to a file, and a link whose absolute target is a file on the machine, count as that file in both modes. A relative target that leads out of the repository's top differs: `--base` follows it out of the checkout, while in the extracted tree it leads out of a temporary directory and names nothing, so under `--overlap` such a link is no file.

`crates/clm` routes `clm affected`, loading `config`'s `affected` and passing the loader in, as it passes `detectors` to `clm check`; the call runs that loader on the CLM root for `--base` and `--why`, and never for `--readers` or `--overlap`. For `--overlap` each side's globs come from its own commit: the call reads the text of `clm.toml` at the CLM root's place in that side's tree through `clm-git` (none when the tree holds no such file) and loads it with `config`'s `load_committed(root, text, source, env)` (`config/committed-settings`), `source` being `<rev>:<path>`, the side's revision as given and the file's path from the top of the git repository, and `env` the call's lookup, so the clone's `clm.local.toml` still applies as it does to any loading. That call is why this crate depends on `clm-config`; the root's loader stays the one for the checkout, since it words a failure as the root's settings line. Under `--overlap` each side's `clm.toml` is loaded before `affected_between` loads its graph, side `a` first (`overlap-selection` gives the order of refusals). The stand-in loader in this crate's tests words its line as the root does. Fixture graphs in the tests give their root component no `depends_on`, so the root is affected only when a scenario says so.

## Requirements

### Requirement: affected-files
`clm affected` SHALL print, after the components, `file <path>` for each changed file that is still a file in the working tree, a symbolic link to a file included, and for each file `check`'s coverage scan reads that holds a `req:` marker, inside a test or outside one, whose requirement ID names an affected component, or that holds no `req:` marker at all and whose owner is affected, and for each reader (`affected-readers`), and, whenever any component is affected, for each file that git tracks, or that is untracked and not ignored, and that is a file in the working tree, other than a Markdown file, which imports nothing, that an exemption of its owner names for `undeclared-imports` (`model/exemptions`), whether or not it can be read as UTF-8 text, since it imports code the graph doesn't connect it to and so can break from a change anywhere (`land-checks-stay-fast`, `temporary-exemptions`), sorted by path, and for no other file. A changed submodule, nested git repository or link to a directory SHALL get no `file` line and SHALL count as a changed file all the same, affecting its owner (`affected-components`), or printing `all <path>` with the path as git names it, a nested repository's ending in `/`, when no component's `paths` hold it (`affected-all`).

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

#### Scenario: a changed path that is no file
- GIVEN component `a` owning `crates/a/**`, depended on by no component, the root component's `paths` holding `.gitmodules`, and in turn: (1) a branch that adds only a submodule at `crates/a/sub`, with the `.gitmodules` entry for it; (2) a branch that adds only a symbolic link `crates/a/dir` to `src`; (3) a branch that adds only a symbolic link `crates/a/src/link.rs` to `lib.rs`; (4) a branch equal to `main` with an untracked nested git repository at `crates/a/nested/`; (5) the same with an untracked nested git repository at `tools/nested/` too, which no component's `paths` hold
- WHEN `clm affected --base main` runs in each
- THEN (1), (2) and (4) each print the component line of `a` and no line naming `crates/a/sub`, `crates/a/dir` or `crates/a/nested`; (3) prints the component line of `a` and `file crates/a/src/link.rs`; and (5) prints only `all tools/nested/`

#### Scenario: an exempt file whatever its bytes
- GIVEN component `d`, on which nothing depends, owning `crates/d/**` and exempting `crates/d/tests/t.rs` and `crates/d/tests/t.py` from `undeclared-imports` until `backlog/x`, both committed, `crates/d/tests/t.py` holding bytes that aren't UTF-8, and a branch that changes only `crates/a/src/lib.rs`
- WHEN `clm affected --base main` runs
- THEN it prints `file crates/d/tests/t.py` and `file crates/d/tests/t.rs`, and no component line of `d`

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
`clm affected` SHALL compare the working tree, untracked files included, with the merge base of `--base <ref>` and `HEAD`, defaulting `<ref>` to `origin/main`. It SHALL exit 1 writing `clm affected: unknown base <ref>` to standard error when `<ref>` doesn't resolve to a commit, and `clm affected: no merge base between <ref> and HEAD` when it has no merge base with `HEAD`; 1 with the error when the graph can't be loaded; and 1 when its caller can't load the `[affected]` settings (`config/affected-setting`), writing the line the caller gives for the failure to standard error, which names the file and the key and is the root's to print (`clm/routes-only`). With `--readers` it SHALL load neither the graph nor the settings, so neither failure stops it. When git fails in any other way it SHALL exit 2, writing to standard error the git command that failed and its error. Whenever it exits non-zero it SHALL write nothing to standard output, and every reason it words itself, for any of its flags, SHALL start `clm affected: `.

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
- GIVEN a directory that is no git repository, whose `specs/` load as a graph
- WHEN `clm affected --base main` runs there
- THEN it exits 2, standard error starts `clm affected: ` and names the git command that failed, and standard output is empty

#### Scenario: readers with a graph and settings that can't be loaded
- GIVEN reader files, a dependency cycle in `specs/`, a `clm.toml` whose `[affected] all` is no list, and a branch that changes a path the readers read
- WHEN `clm affected --base main --readers` runs
- THEN it prints the readers' `file` lines and exits 0

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
`clm affected --why <path> [--base <ref>]` SHALL print, and the Contract's `why` SHALL give as its lines, the chain by which the change selects `<path>`, one link per line, from `<path>` to a changed file, taking the first reason that holds in this order: `<path> (changed)` alone when `<path>` is a changed file that still exists; `<path> reads <changed path>` when it is a reader, naming the first such path in path order; `<path> cites <component>` when its `req:` marker names an affected component, the first by ID; `<path> owned by <component>` when it is a file the coverage scan reads, holds no `req:` marker and its owner is affected; and `<path> exempt from undeclared-imports` otherwise, when it is selected only as a file an exemption names for that rule (`affected-files`), with no line after it. After a `cites` or `owned by` line come the edges of the shortest path from that component to a component that owns a changed file, choosing among equally short paths the first in ID order: the path follows `depends_on` edges, each printed `<x> depends on <y>`, and then `<y> owns <changed path> (changed)`, naming that component's first changed path in path order. It SHALL print `<path> not selected` for a path the selection doesn't hold, a deleted changed file among them, and `all: <file>` when the selection is `all <file>`, exiting 0 in each case and as the selection does otherwise. Its lines SHALL say component, never node (`component-words`).

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

#### Scenario: an exempt test whose owner is affected
- GIVEN components `lib`, owning `lib/src/lib.rs`, `other`, owning `other/src/lib.rs`, both depending on nothing, and `app`, owning `app/**` and depending on `lib`, which exempts `app/tests/t.rs` from `undeclared-imports` until `backlog/x`; `app/tests/t.rs` citing only a requirement of `other`; and a branch that changes only `lib/src/lib.rs`
- WHEN `clm affected --why app/tests/t.rs` runs
- THEN it prints `app/tests/t.rs exempt from undeclared-imports` alone and exits 0

#### Needs
- model/exemptions

### Requirement: overlap-selection
`clm affected --overlap <a> <b>` SHALL print what both changes can break. With `<m>` the merge base of the commits `<a>` and `<b>` name, each *side* is the selection, readers included, of the paths that differ between `<m>` and that side's commit, computed as `affected-files`, `affected-all`, `affected-components` and `affected-readers` compute it but from the two commits alone, with no working tree and no untracked file: each side's changed paths are those that differ between `<m>` and its commit, a rename counting as its old path and its new one, and its graph, its markers (`req:` and `clm:reads`), the files the coverage scan would read and its `[affected] all` globs are read from that side's commit, not from any checkout, a symbolic link of that commit counting as a file when it leads to one as a working tree's link would, links to links and absolute targets followed, except that one whose relative target leads out of the repository's top is no file. A side's globs SHALL be those loading gives with the `clm.toml` at the CLM root's place in that side's commit as the repository's layer, or with none when the commit holds no such file (`config/committed-settings`), the clone's `clm.local.toml` read as any loading reads it. Its output SHALL be:
- when both sides are `all`, the first side's `all` line;
- when one side is `all`, the other side's output unchanged;
- otherwise the component lines of the components both sides affect, then the `file` lines of the files both sides select, each group sorted as `affected-components` sorts it, and nothing at all when no file is in both.

The Contract's `overlap(a, b)` SHALL apply these rules to two selections, and its `affected_between(root, from, to, all)` SHALL give the selection between two commits. `--overlap` with `--base`, `--readers` or `--why` SHALL exit 1 naming both flags. A `<a>` or `<b>` that doesn't resolve to a commit SHALL exit 1 with `clm affected: unknown revision <rev>`, two commits with no merge base SHALL exit 1 with `clm affected: no merge base between <a> and <b>`, and any other git failure SHALL exit 2, as `affected-base` says for `--base`. A side whose commit holds a graph that can't be loaded SHALL exit 1 with the error, as `affected-base` says for `--base`, and one whose commit holds a `clm.toml` that can't be loaded SHALL exit 1 with `clm affected: <rev>:<path>: ` and the reason loading gives, `<rev>` the side's revision as given and `<path>` the file's path from the top of the git repository; a failure of another layer loading reads, such as the clone's `clm.local.toml`, SHALL exit 1 with `clm affected: ` and the error loading gives. The call SHALL compute the overlap from the two commits alone, reading neither the checkout's graph nor its settings. When more than one of these refusals applies, the first in this order SHALL be written: revision `<a>`, revision `<b>`, the merge base, side `<a>`'s `clm.toml`, side `<a>`'s graph, side `<b>`'s `clm.toml`, side `<b>`'s graph.

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

#### Scenario: a side whose graph can't be loaded
- GIVEN the components of `independent changes`, a commit `m`, and commits `x` and `y` on top of `m`, `x` changing `crates/a/src/lib.rs` and `y` adding components `c` and `d` that depend on each other
- WHEN `clm affected --overlap x y` runs with `x` checked out, and again `clm affected --overlap x x` with `y` checked out
- THEN the first exits 1, its standard error starts `clm affected: ` and holds `dependency cycle`, and its standard output is empty; and the second exits 0

#### Scenario: a link on both sides
- GIVEN the components of `independent changes`, a commit `m` holding `crates/b/src/link.rs`, a symbolic link to `lib.rs`, and commits `x` and `y` on top of `m`, `x` changing `crates/b/src/lib.rs` and `y` adding `crates/b/src/new.rs`
- WHEN `clm affected --overlap x y` runs
- THEN it prints the component line of `b`, `file crates/b/src/lib.rs` and `file crates/b/src/link.rs`, the lines `clm affected --base m` prints with `x` checked out, and nothing else

#### Scenario: the settings for both sides
- GIVEN the commits of `one side selects everything`, the root component's `paths` holding `Cargo.lock` and `clm.toml` and no component depending on the root, and in turn: (1) no commit holding a `clm.toml`, and the working tree holding an untracked `clm.toml` with `[affected]` and `all = ["Cargo.lock"]`; (2) `m` committing that `clm.toml`, and the working tree's `clm.toml` holding no `[affected]`; (3) `m` committing that `clm.toml`, `y` also committing a `clm.toml` that isn't TOML, and the working tree's `clm.toml` holding `[affected]` with `all = 3`; (4) as (3), and `x` also committing a `clm.toml` that isn't TOML, while `y` also adds two components that depend on each other
- WHEN `clm-affected`'s call for `clm affected --overlap x y` runs in each
- THEN (1) prints nothing and returns 0; (2) prints exactly what `clm affected --base m` prints in a clean checkout of `y`; (3) returns 1, its error stream starts `clm affected: y:clm.toml: `, and its output stream is empty; and (4) returns 1 and its error stream starts `clm affected: x:clm.toml: `

### Requirement: commands-in-crate
`clm-affected` SHALL offer `clm affected` as a call (the Contract's) that takes the command's arguments as the command line gives them, the repository root its caller found, which the command's `--root` overrides, the environment as a lookup, what loads the `[affected]` settings, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. Given `--help` or `-h` it SHALL write the command's help to the output stream, write nothing to the error stream and return 0: the help's first line SHALL be the command's description, exactly ``Print what a change can break: `node <id>` and `file <path>` lines, or `all <path>` when every test must run``, with no full stop after it, then an empty line, then a line starting `Usage: clm affected `, then the options, each of `--base`, `--root`, `--readers`, `--overlap` and `--why` among them.

#### Scenario: a command called in a test
- GIVEN a fixture repository
- WHEN the call is given the arguments of `clm affected` and two buffers
- THEN the buffers hold what `clm affected` prints to standard output and to standard error there, and the call returns the status `clm affected` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN it is given an option `clm affected` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the caller's process goes on

#### Scenario: help
- GIVEN the same call
- WHEN it is given `--help`, and again `-h`
- THEN each time it returns 0, the output buffer's first line is the command's description with no full stop after it, its second is empty, its third starts `Usage: clm affected `, it names `--base`, `--root`, `--readers`, `--overlap` and `--why`, and the error buffer is empty

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: clm-root-paths
`clm affected` SHALL read a change from the CLM root it runs in, which may be a subdirectory of its git repository. With `--base`, `--readers`, `--why` and `--overlap` alike, the paths that differ, the files the scan reads and every path it prints SHALL be those under the CLM root, named from it as component `paths` and `clm:reads` globs are, so a path outside the CLM root is no changed file, selects nothing and differs for no reader; and with `--overlap` each commit's graph, markers and files SHALL be read from that commit's tree at the CLM root's place in it.

#### Scenario: a branch in a subdirectory root
- GIVEN a git repository whose CLM root is `sub/`, a component `a` owning `crates/a/**`, `sub/crates/a/src/util.rs` with no marker, and a branch that changes `sub/crates/a/src/lib.rs` and `other/build.sh`, outside `sub/`
- WHEN `clm affected --base main` runs in `sub/`
- THEN it prints the component line of `a`, `file crates/a/src/lib.rs` and `file crates/a/src/util.rs`, and no `all` line

#### Scenario: two commits in a subdirectory root
- GIVEN the same repository, a commit `m`, and commits `x` and `y` on top of `m`, `x` changing `sub/crates/a/src/lib.rs` and `other/build.sh`, and `y` changing `sub/crates/a/src/util.rs`
- WHEN `clm affected --overlap x y` runs in `sub/`
- THEN it prints the component line of `a`, `file crates/a/src/lib.rs` and `file crates/a/src/util.rs`, and exits 0

#### Scenario: a reader in a subdirectory root
- GIVEN the same repository, `sub/crates/a/tests/r.rs` holding `clm:reads ARCHITECTURE.md`, and a branch that changes only `ARCHITECTURE.md` at the top of the git repository, outside `sub/`
- WHEN `clm affected --base main --readers` runs in `sub/`
- THEN it prints nothing and exits 0
