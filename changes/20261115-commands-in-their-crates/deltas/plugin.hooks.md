---
component: plugin.hooks
base: { contract: "d93f586c8039927a" }
---

## MODIFIED Contract

The rules behind `clm hook session-start`, `clm hook autosync` and `clm defect` (`hook-logic-in-clm`): what the session-start hook prints, when autosync runs, and how a defect report is filed. `plugin`'s `hooks.json` wires Claude Code's events to `clm hook <name>`; this component and its child `plugin.hooks.guard` decide what they do. `clm hook guard`, which asks the human to confirm `clm approve`, `clm capabilities decide` and a `clm defect` whose report leaves the project, and blocks pushes to `main` and release runs, is `plugin.hooks.guard`'s. Crate `clm-hooks`: `session_start(event)`, `autosync(event)` and `defect(...)`, with `settings(cwd)` and `clm_root(cwd)`, which find the CLM repository for a directory and load its settings the one way the hooks, the guard and `clm defect` share.

`clm-hooks` offers each of `clm defect`, `clm hook session-start` and `clm hook autosync` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the directory it runs in and the text on standard input, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-hooks` SHALL offer each of `clm defect`, `clm hook session-start` and `clm hook autosync` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the directory it runs in and the text on standard input, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `plugin.hooks`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `plugin.hooks`'s own files and call `clm-hooks` itself, this call where the requirement states what a command prints or how it exits: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `plugin.hooks`'s own files
- WHEN it calls `clm-hooks`'s call with the arguments of `clm defect` and two buffers
- THEN the buffers hold what `clm defect` prints to standard output and to standard error there, and the call returns the status `clm defect` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm defect` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
