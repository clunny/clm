---
component: apply
base: { contract: "d2c57a3cb78ef29c" }
---

## MODIFIED Contract

- `validate(graph, change)` returns every reason the change's deltas and assets cannot be applied to the current spec graph, as violations, including every rule across components that the graph the change leaves would break, so a change that applies leaves a graph that loads.
- `resulting(graph, change)` returns the spec graph `validate` checks the change against, the graph the change would leave, or none when a delta of the change can't be applied, for `check`'s rules that judge a pending change's text (`resulting-graph-call`).
- `stale_bases(component, delta)` returns each Contract, Design or requirement `delta` MODIFIES or REMOVES whose recorded base `validate` judges stale against `component` (`stale-base`), with that base, for `sync`'s stale-delta report and merges (`stale-elements`).
- `clm apply <change-id>` applies a change's deltas to `specs/` and copies or removes its assets, deletes the `question` items its `resolves` names (`answered-questions`) and the item its `from` names (`promoted-item`), all-or-nothing, and marks the change `applied`. It exits 1 and writes nothing on any violation of the change, or of the loaded graph that the change doesn't fix (`loaded-graph`).
- `clm hashes <node-id>` prints one `<element> <hash>` line per element of the component: `frontmatter`, `contract`, `design`, then `requirement:<slug>` for each requirement, then `asset:<path>` for each asset. `clm hashes architecture` and `clm hashes capability` print one line per entry of that human layer, in file order, `requirement:<slug> <hash>` or `capability:<slug> <hash>`, the word its heading uses (`model/human-layers`). When the repository has the capability layer's component (`model/reserved-ids`), `clm hashes capability` prints that component's element lines first, then the entries' lines: the component holds no requirement, so no line name is printed twice. It isn't called `hash`, which Claude Code's worktree isolation refuses (`worktree-safe-commands`).

`clm-apply` offers each of `clm apply`, `clm archive` and `clm hashes` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-apply` SHALL offer each of `clm apply`, `clm archive` and `clm hashes` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. It SHALL print a violation as the line `clm-model` gives for it (`model/violation-line`), since this crate does not depend on `check`. Every test of `apply`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `apply`'s own files and call `clm-apply` itself, this call where the requirement states what a command prints or how it exits: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `apply`'s own files
- WHEN it calls `clm-apply`'s call with the arguments of `clm apply` and two buffers
- THEN the buffers hold what `clm apply` prints to standard output and to standard error there, and the call returns the status `clm apply` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm apply` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

#### Needs
- model/violation-line
