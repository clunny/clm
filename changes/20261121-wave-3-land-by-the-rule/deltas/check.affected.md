---
component: check.affected
base: { contract: "6ca3385d79fd2101" }
---

## MODIFIED Contract
`clm affected [--root <dir>] [[--base <ref>] [--readers | --why <path>] | --overlap <a> <b>]` prints what a change can break, read from the spec graph (`graph-test-selection`): the components it affects and the files whose tests a check should run, or that every test must run. It knows no test runner; the project's check command turns the files into its runner's arguments (`ci`), and `clm land` hands it the output (`land`).

- The *changed files* are the paths that differ between the merge base of `<ref>` (default `origin/main`) and `HEAD`, and the working tree, with the untracked files git doesn't ignore, leaving out every doc-only path (`ledger/doc-only`). A deleted file is a changed file, and so is a changed submodule or nested git repository. Every path it reads or prints, in each mode, is under the CLM root and named from it, as component `paths` are, though the CLM root be a subdirectory of the git repository (`clm-root-paths`).
- When a changed file matches a glob of `config`'s `[affected] all`, or no component's `paths` hold it (`check.coverage/owner`), the output is one line, `all <file>`, naming the first such file in path order.
- Otherwise the *affected* components are the owners of the changed files and every component that depends on one of them, directly or indirectly (`depends_on`). The output is a *component line* for each affected component, `node <id>`, the word `component-words` keeps for these lines, which `ci`'s check script and `clm land` read, then `file <path>` for each selected file, each group sorted, and nothing when no file changed.
- The *selected* files are the changed files that are still a file in the working tree, a symbolic link to one included, whatever their content; the files `check`'s coverage scan reads (tracked, or untracked and not ignored; UTF-8; Markdown left out) that hold a `req:` marker whose requirement ID names an affected component (the part before `/`, whether or not the requirement still exists), or that hold no `req:` marker and whose owner is affected; the *readers*; and, whenever any component is affected, every file the scan lists (tracked, or untracked and not ignored; Markdown left out) that an exemption of its owner names for `undeclared-imports`, whether or not it is UTF-8.
- A *reader* is a file the coverage scan reads that holds a `clm:reads <glob>` marker (`doc-readers-marked`) whose glob matches a path that differs as the changed files do, doc-only paths included: a test that reads the repository's own files, such as its specs or `ARCHITECTURE.md`, names them so, since nothing else connects the test to them. `--readers` prints the readers' `file` lines alone, with no component or `all` line, loading neither the graph nor the settings, which `clm land` runs on a tree whose code was already checked.
- `--why <path>` prints why the change selects `<path>`, as the chain from the file to a changed file, one link per line (`why-chain`). `clm land` runs it to explain a step's cost (`land/check-time-report`).
- `--overlap <a> <b>` prints what the changes of two commits can both break, each against their merge base: the call computes each side from its commit alone, its paths, graph, markers, files and `[affected] all` globs, and reads neither the checkout's graph nor its `clm.toml`, the clone's `clm.local.toml` being read as any loading reads it (`overlap-selection`), which `clm land` runs when `main` moved under a checked branch (`land/overlap-recheck`).

It exits 0 with its output, and 0 with the usage on standard output for `--help` or `-h`; 1 with the reason when two of `--readers`, `--why` and `--overlap`, or `--overlap` and `--base`, are given together, when a ref or revision doesn't resolve to a commit, when the two sides have no merge base, or when the graph or the settings can't be loaded (with `--overlap`, either commit's graph or `clm.toml`, or another layer that loading reads, such as the clone's `clm.local.toml`); 2 with the usage error on standard error for arguments it can't parse; and 2 when git fails otherwise. A reason of its own starts `clm affected: `, and nothing is on standard output when it exits non-zero.

`clm-affected` offers `clm affected` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the CLM root, the environment as a lookup, what loads the `[affected]` settings and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes. Its argument definitions are `Args`, which the root's parser reads. As library calls it offers too `affected(root, base, all)`, which gives the selection as a `Selection`, everything or the affected components and the selected files, which `verify` reads; `readers(differing, reads)`, over paths that differ, each a `Changed`, which the root's test of `clm/doc-readers-marked` reads; and `reads_in(text)`, the globs of the `clm:reads` markers in a file's text, which `verify` reads to name the files a requirement's tests read (`verify/mutation-pass`). `clm land` runs `clm affected` as a process, its `--base`, `--readers`, `--why` and `--overlap` forms, and calls none of the library's selections.

## MODIFIED Requirements

### Requirement: why-chain <!-- base: 6518d01fd30944d5 -->
`clm affected --why <path> [--base <ref>]` SHALL print the chain by which the change selects `<path>`, one link per line, from `<path>` to a changed file, taking the first reason that holds in this order: `<path> (changed)` alone when `<path>` is a changed file that still exists; `<path> reads <changed path>` when it is a reader, naming the first such path in path order; `<path> cites <component>` when its `req:` marker names an affected component, the first by ID; `<path> owned by <component>` when it is a file the coverage scan reads, holds no `req:` marker and its owner is affected; and `<path> exempt from undeclared-imports` otherwise, when it is selected only as a file an exemption names for that rule (`affected-files`), with no line after it. After a `cites` or `owned by` line come the edges of the shortest path from that component to a component that owns a changed file, choosing among equally short paths the first in ID order: the path follows `depends_on` edges, each printed `<x> depends on <y>`, and then `<y> owns <changed path> (changed)`, naming that component's first changed path in path order. It SHALL print `<path> not selected` for a path the selection doesn't hold, a deleted changed file among them, and `all: <file>` when the selection is `all <file>`, exiting 0 in each case and as the selection does otherwise. Its lines SHALL say component, never node (`component-words`).

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

### Requirement: overlap-selection <!-- base: 8fd7fe1a3f40560a -->
`clm affected --overlap <a> <b>` SHALL print what both changes can break. With `<m>` the merge base of the commits `<a>` and `<b>` name, each *side* is the selection, readers included, of the paths that differ between `<m>` and that side's commit, computed as `affected-files`, `affected-all`, `affected-components` and `affected-readers` compute it but from the two commits alone, with no working tree and no untracked file: each side's changed paths are those that differ between `<m>` and its commit, a rename counting as its old path and its new one, and its graph, its markers (`req:` and `clm:reads`), the files the coverage scan would read and its `[affected] all` globs are read from that side's commit, not from any checkout, a symbolic link of that commit counting as a file when it leads to one as a working tree's link would, links to links and absolute targets followed, except that one whose relative target leads out of the repository's top is no file. A side's globs SHALL be those loading gives with the `clm.toml` at the CLM root's place in that side's commit as the repository's layer, or with none when the commit holds no such file (`config/committed-settings`), the clone's `clm.local.toml` read as any loading reads it. Its output SHALL be:
- when both sides are `all`, the first side's `all` line;
- when one side is `all`, the other side's output unchanged;
- otherwise the component lines of the components both sides affect, then the `file` lines of the files both sides select, each group sorted as `affected-components` sorts it, and nothing at all when no file is in both.

`--overlap` with `--base`, `--readers` or `--why` SHALL exit 1 naming both flags. A `<a>` or `<b>` that doesn't resolve to a commit SHALL exit 1 with `clm affected: unknown revision <rev>`, two commits with no merge base SHALL exit 1 with `clm affected: no merge base between <a> and <b>`, and any other git failure SHALL exit 2, as `affected-base` says for `--base`. A side whose commit holds a graph that can't be loaded SHALL exit 1 with the error, as `affected-base` says for `--base`, and one whose commit holds a `clm.toml` that can't be loaded SHALL exit 1 with `clm affected: <rev>:<path>: ` and the reason loading gives, `<rev>` the side's revision as given and `<path>` the file's path from the top of the git repository; a failure of another layer loading reads, such as the clone's `clm.local.toml`, SHALL exit 1 with `clm affected: ` and the error loading gives. The call SHALL compute the overlap from the two commits alone, reading neither the checkout's graph nor its `clm.toml`. When more than one of these refusals applies, the first in this order SHALL be written: revision `<a>`, revision `<b>`, the merge base, side `<a>`'s settings, side `<a>`'s graph, side `<b>`'s settings, side `<b>`'s graph, a side's settings failing for its commit's `clm.toml` or for another layer loading reads with it.

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
