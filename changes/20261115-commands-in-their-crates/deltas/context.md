---
component: context
base: { contract: "7f73695daa7f3e73" }
---

## MODIFIED Contract

`clm context <component>` deterministically prints the bundle an agent needs to work on a component: `ARCHITECTURE.md`, CLM's carried constraints in force in a project that isn't CLM's own (`carried-constraints`), the capabilities the component serves, the contract and design of every component among the ancestors from the root down to the component, the component's frontmatter, requirements and list of assets, and the contract of each component it depends on directly, through `depends_on`. `clm context <component> --hash` prints the bundle's content hash, so a caller can tell whether anything an agent working on the component would read has changed (`hash-based-staleness`); `clm rebase` uses it to report a spec that moved under a build (`sync/spec-changed`).

The bundle is also available without a checkout (`bundle_from`), from the graph and the text of `ARCHITECTURE.md`, with the same result: the graph holds `CAPABILITIES.md`'s entries and the carried constraints in force. The crate builds for `wasm32-unknown-unknown`, so the `web` Worker can use it through `orchestrator`.

`clm-context` offers `clm context` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-context` SHALL offer `clm context` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `context`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `context`'s own files and call `clm-context` itself, this call where the requirement states what a command prints or how it exits: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `context`'s own files
- WHEN it calls `clm-context`'s call with the arguments of `clm context` and two buffers
- THEN the buffers hold what `clm context` prints to standard output and to standard error there, and the call returns the status `clm context` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm context` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
