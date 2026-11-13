---
component: check.affected
base: { contract: "55525e9e68b8957c", design: "a02df69aae3cdcec" }
---

## MODIFIED Contract
`clm affected [--base <ref>] [--root <dir>] [--readers | --why <path>]` prints what a change can break, read from the spec graph (`graph-test-selection`): the components it affects and the files whose tests a check should run, or that every test must run. It knows no test runner; the project's check command turns the files into its runner's arguments (`ci`), and `clm land` hands it the output (`land`).

- The *changed files* are the paths that differ between the merge base of `<ref>` (default `origin/main`) and `HEAD`, and the working tree, with the untracked files git doesn't ignore, leaving out every path `ledger`'s `doc_only` names. A deleted file is a changed file.
- When a changed file matches a glob of `config`'s `[affected] all`, or no component's `paths` hold it (`check`'s `owner` gives none), the output is one line, `all <file>`, naming the first such file in path order.
- Otherwise the *affected* components are the owners of the changed files, every component that depends on one of them, directly or indirectly (`depends_on`), and every component whose `test_depends_on` names one of those, since its tests import it. The output is a *component line* for each affected component, the word `component-words` keeps for these lines followed by a space and the component's ID, which `ci`'s check script and `clm land` read, then `file <path>` for each selected file, each group sorted, and nothing when no file changed.
- The *selected* files are the changed files that still exist, whatever their content, the files `check`'s coverage scan reads (tracked, or untracked and not ignored; UTF-8; Markdown left out) that hold a `req:` marker whose requirement ID names an affected component (the part before `/`, whether or not the requirement still exists), or that hold no `req:` marker and whose `owner` is affected, and the *readers*.
- A *reader* is a file the coverage scan reads that holds a `clm:reads <glob>` marker (`doc-readers-marked`) whose glob matches a path that differs as the changed files do, doc-only paths included: a test that reads the repository's own files, such as its specs or `ARCHITECTURE.md`, names them so, since `code_key` leaves the doc-only ones out and nothing else connects the test to them. `--readers` prints the readers' `file` lines alone, with no component or `all` line, which `clm land` runs on a tree whose code was already checked.
- `--why <path>` prints why the change selects `<path>`, as the chain from the file to a changed file, one link per line (`why-chain`), which the library offers as `why(root, base, path)`. `clm land` runs it to explain a step's cost (`land/check-time-report`).

It exits 0 with its output, 1 with the reason when `--readers` and `--why` are both given, when `<ref>` doesn't resolve to a commit, when it has no merge base with `HEAD`, or when the graph or the settings can't be loaded, and 2 when git fails otherwise.

## MODIFIED Design
Crate `clm-affected`. `select(graph, changed, markers, all) -> Selection` is pure: it takes the graph (`model`), the changed paths with whether each still exists, the markers of every scanned file (`check`'s `scan`, each with its path) and the `[affected] all` globs, and returns `All(path)` or `Selection::Nodes`, holding the affected components' IDs and the selected files. Owners and globs are matched with `check`'s `owner` and `path_matches`, so a file belongs to the same component as in `clm pending`; paths are disjoint across components, and `owner`'s deepest-first rule settles any overlap. A `clm:remove-by` marker isn't a `req:` marker: a file holding only those belongs to its owner, and neither is a `clm:reads` marker. Dependants are found by walking `depends_on` backwards from the changed components until no component is added; then each component whose `test_depends_on` names one of those is added, and the walk doesn't go on from it, since only its tests, not its code, use the changed component. Doc-only paths come from `ledger`'s `doc_only`, the one list `code_key` uses, so a change whose code key is unchanged selects only readers.

**Readers.** `readers(differing, reads) -> Vec<path>` is pure too: it takes every differing path, doc-only ones included, with whether it still exists, and each scanned file's `clm:reads` globs, and returns the files, sorted, one of whose globs matches a differing path by `check`'s `path_matches`, so a glob reads as a component's `paths` do. A marker is the text `clm:reads ` and the glob after it, up to the next whitespace; a file may hold several, and a marker with no glob after it is ignored. A reader is selected whether or not its owner is affected, and a deleted reader isn't printed. The markers come from the files the coverage scan reads, so a Markdown file is never a reader. Every file of this component, source and doc comments as well as test fixtures, builds or writes the marker's text so the literal and a space never appear whole (`concat!("clm", ":reads specs/**")`), as fixtures do `req:` markers, or the file would be a reader of whatever follows; so does any other file that only mentions the marker. Without `--readers` the readers join the other `file` lines, sorted with them; with it, `affected` prints them alone and doesn't read `[affected] all` or the owners at all, since the question is only which tests read what changed.

`affected(root, base, all)` gathers the inputs through `git` (`git-plumbing-layer`): `git rev-parse --verify <base>^{commit}` (failing: exit 1, `unknown base <ref>`), `git merge-base <base> HEAD` (exit 1, `no merge base between <ref> and HEAD`, when it prints nothing with status 1), then `git diff --name-only --no-renames -z <merge base>` (the working tree against the merge base, so a rename counts as its old and new path) and `git ls-files --others --exclude-standard -z`. Any other git failure is exit 2. A changed path is matched as git lists it, relative to the repository root. A deleted file still selects through its path, its owner being affected, but has no `file` line; a deleted file no component holds selects everything.

`crates/clm` dispatches `clm affected`, with `--root` as every command has it, loading `config`'s `affected` and passing its globs in, as it passes `detectors` to `clm check`, so this crate doesn't depend on `clm-config`; settings that can't be loaded exit 1 naming the file and key, as for `clm size`. This component's build adds the dispatch (the root's Design), and its scenarios run the command, so they wait on `check`'s, `ledger`'s and `config`'s requirements below (`#### Needs`). Fixture graphs in the tests give their root component no `depends_on`, so the root is affected only when a scenario says so.

## MODIFIED Requirements

### Requirement: why-chain <!-- base: 7e236f82983f2cc6 -->
`clm-affected`'s `why(root, base, path)`, and `clm affected --why <path> [--base <ref>]`, SHALL give the chain by which the change selects `<path>`, one link per line, from `<path>` to a changed file, taking the first reason that holds in this order: `<path> (changed)` alone when `<path>` is a changed file that still exists; `<path> reads <changed path>` when it is a reader, naming the first such path in path order; `<path> cites <component>` when its `req:` marker names an affected component, the first by ID; `<path> owned by <component>` otherwise. After a `cites` or `owned by` line come the edges of the shortest path from that component to a component that owns a changed file, choosing among equally short paths the first in ID order: the path follows `depends_on` edges, each printed `<x> depends on <y>`, and only as its first edge, when no `depends_on` path reaches such a component, a `test_depends_on` edge, printed `<x> tests against <y>`, and then `<y> owns <changed path> (changed)`, naming that component's first changed path in path order. It SHALL print `<path> not selected` for a path the selection doesn't hold, a deleted changed file among them, and `all: <file>` when the selection is `all <file>`, exiting 0 in each case and as the selection does otherwise. Its lines SHALL say component, never node (`component-words`).

#### Scenario: through a dependency
- GIVEN components `a`, owning `a/src/lib.rs`, and `b`, owning `b/src/lib.rs`, with `a` depending on `b`, and a branch that changes only `b/src/lib.rs`
- WHEN `clm affected --why a/src/lib.rs` runs
- THEN it prints `a/src/lib.rs owned by a`, `a depends on b` and `b owns b/src/lib.rs (changed)`, and exits 0

#### Scenario: through a test dependency
- GIVEN components `app`, owning `app/src/lib.rs`, depending on `feat`, and `feat`, owning `feat/tests/t.rs`, with `test_depends_on: [app]`, and a branch that changes only `app/src/lib.rs`
- WHEN `clm affected --why feat/tests/t.rs` runs
- THEN it prints `feat/tests/t.rs owned by feat`, `feat tests against app` and `app owns app/src/lib.rs (changed)`, and exits 0

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
