---
component: check.affected
base: { contract: "6a1279b4e9577590" }
---

## MODIFIED Contract

`clm affected [--base <ref>] [--root <dir>] [--readers | --why <path>]` prints what a change can break, read from the spec graph (`graph-test-selection`): the components it affects and the files whose tests a check should run, or that every test must run. It knows no test runner; the project's check command turns the files into its runner's arguments (`ci`), and `clm land` hands it the output (`land`).

- The *changed files* are the paths that differ between the merge base of `<ref>` (default `origin/main`) and `HEAD`, and the working tree, with the untracked files git doesn't ignore, leaving out every path `ledger`'s `doc_only` names. A deleted file is a changed file.
- When a changed file matches a glob of `config`'s `[affected] all`, or no component's `paths` hold it (`check`'s `owner` gives none), the output is one line, `all <file>`, naming the first such file in path order.
- Otherwise the *affected* components are the owners of the changed files, every component that depends on one of them, directly or indirectly (`depends_on`). The output is a *component line* for each affected component, the word `component-words` keeps for these lines followed by a space and the component's ID, which `ci`'s check script and `clm land` read, then `file <path>` for each selected file, each group sorted, and nothing when no file changed.
- The *selected* files are the changed files that still exist, whatever their content, the files `check`'s coverage scan reads (tracked, or untracked and not ignored; UTF-8; Markdown left out) that hold a `req:` marker whose requirement ID names an affected component (the part before `/`, whether or not the requirement still exists), or that hold no `req:` marker and whose `owner` is affected, and the *readers*.
- A *reader* is a file the coverage scan reads that holds a `clm:reads <glob>` marker (`doc-readers-marked`) whose glob matches a path that differs as the changed files do, doc-only paths included: a test that reads the repository's own files, such as its specs or `ARCHITECTURE.md`, names them so, since `code_key` leaves the doc-only ones out and nothing else connects the test to them. `--readers` prints the readers' `file` lines alone, with no component or `all` line, which `clm land` runs on a tree whose code was already checked.
- `--why <path>` prints why the change selects `<path>`, as the chain from the file to a changed file, one link per line (`why-chain`), which the library offers as `why(root, base, path)`. `clm land` runs it to explain a step's cost (`land/check-time-report`).

It exits 0 with its output, 1 with the reason when `--readers` and `--why` are both given, when `<ref>` doesn't resolve to a commit, when it has no merge base with `HEAD`, or when the graph or the settings can't be loaded, and 2 when git fails otherwise.

`clm-affected` offers `clm affected` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-affected` SHALL offer `clm affected` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `check.affected`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `check.affected`'s own files and call `clm-affected` itself, this call where the requirement states what a command prints or how it exits: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `check.affected`'s own files
- WHEN it calls `clm-affected`'s call with the arguments of `clm affected` and two buffers
- THEN the buffers hold what `clm affected` prints to standard output and to standard error there, and the call returns the status `clm affected` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm affected` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
