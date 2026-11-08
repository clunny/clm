---
component: check.affected
base: { contract: "0ca7aed8e414d875" }
---

## MODIFIED Contract
`clm affected [--base <ref>] [--root <dir>] [--readers | --why <path>]` prints what a change can break, read from the spec graph (`graph-test-selection`): the components it affects and the files whose tests a check should run, or that every test must run. It knows no test runner; the project's check command turns the files into its runner's arguments (`ci`), and `clm land` hands it the output (`land`).

- The *changed files* are the paths that differ between the merge base of `<ref>` (default `origin/main`) and `HEAD`, and the working tree, with the untracked files git doesn't ignore, leaving out every path `ledger`'s `doc_only` names. A deleted file is a changed file.
- When a changed file matches a glob of `config`'s `[affected] all`, or no component's `paths` hold it (`check`'s `owner` gives none), the output is one line, `all <file>`, naming the first such file in path order.
- Otherwise the *affected* components are the owners of the changed files, and every component that depends on one of them, directly or indirectly (`depends_on`). The output is a *component line* for each affected component, the word `component-words` keeps for these lines followed by a space and the component's ID, which `ci`'s check script and `clm land` read, then `file <path>` for each selected file, each group sorted, and nothing when no file changed.
- The *selected* files are the changed files that still exist, whatever their content, the files `check`'s coverage scan reads (tracked, or untracked and not ignored; UTF-8; Markdown left out) that hold a `req:` marker whose requirement ID names an affected component (the part before `/`, whether or not the requirement still exists), or that hold no `req:` marker and whose `owner` is affected, and the *readers*.
- A *reader* is a file the coverage scan reads that holds a `clm:reads <glob>` marker (`doc-readers-marked`) whose glob matches a path that differs as the changed files do, doc-only paths included: a test that reads the repository's own files, such as its specs or `ARCHITECTURE.md`, names them so, since `code_key` leaves the doc-only ones out and nothing else connects the test to them. `--readers` prints the readers' `file` lines alone, with no component or `all` line, which `clm land` runs on a tree whose code was already checked.
- `--why <path>` prints why the change selects `<path>`, as the chain from the file to a changed file, one link per line (`why-chain`), which the library offers as `why(root, base, path)`. `clm land` runs it to explain a step's cost (`land/check-time-report`).

It exits 0 with its output, 1 with the reason when `--readers` and `--why` are both given, when `<ref>` doesn't resolve to a commit, when it has no merge base with `HEAD`, or when the graph or the settings can't be loaded, and 2 when git fails otherwise.

## ADDED Requirements
### Requirement: why-chain
`clm-affected`'s `why(root, base, path)`, and `clm affected --why <path> [--base <ref>]`, SHALL give the chain by which the change selects `<path>`, one link per line, from `<path>` to a changed file, taking the first reason that holds in this order: `<path> (changed)` alone when `<path>` is a changed file that still exists; `<path> reads <changed path>` when it is a reader, naming the first such path in path order; `<path> cites <component>` when its `req:` marker names an affected component, the first by ID; `<path> owned by <component>` otherwise. After a `cites` or `owned by` line come `<x> depends on <y>` for each edge of the shortest `depends_on` path from that component to a component that owns a changed file, choosing among equally short paths the first in ID order, and then `<y> owns <changed path> (changed)`, naming that component's first changed path in path order. It SHALL print `<path> not selected` for a path the selection doesn't hold, a deleted changed file among them, and `all: <file>` when the selection is `all <file>`, exiting 0 in each case and as the selection does otherwise. Its lines SHALL say component, never node (`component-words`).

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
