---
component: view.repo
base: { contract: "763859fe2e67edcf" }
---

## MODIFIED Contract

Crate `clm-view-repo`: the `clm status`, `clm view` and `clm pending` commands' reading of the repository, for `view`'s models and renderers (`views-rendered-by-clm`). `status(root, budgets)` and `view(root, name, format, width, source)` load the graph and markers from the working tree (`Source::Tree`) or, for `clm view --at <rev>`, from that commit's tree (`Source::At`, `view-at-rev`), read the checkout, the claim and `propose/*` refs and the time, and return what `view` renders, or the spec graph's violations, or why `<rev>` couldn't be read. `pending(root, component, at)` gives `clm pending [<component>] [--at <rev>]` its commit line and its pending lines (`pending-names-its-commit`). It never fetches and writes no file.

`clm-view-repo` offers each of `clm status`, `clm view` and `clm pending` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-view-repo` SHALL offer each of `clm status`, `clm view` and `clm pending` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `view.repo`'s requirements SHALL call `clm-view-repo` itself, this call where the requirement states what a command prints or how it exits, and SHALL sit in `view.repo`'s own files: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `view.repo`'s own files
- WHEN it calls `clm-view-repo`'s call with the arguments of `clm status` and two buffers
- THEN the buffers hold what `clm status` prints to standard output and to standard error there, and the call returns the status `clm status` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm status` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on
