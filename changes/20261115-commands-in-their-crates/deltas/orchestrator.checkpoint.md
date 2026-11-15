---
component: orchestrator.checkpoint
base: { contract: "9ccc9c6786c494b7" }
---

## MODIFIED Contract

`clm wrap-up` removes landed work, reports any work that isn't safe on `origin`, names the changes that are complete but not yet archived, and says whether the session can end (`checkpoint-before-done`). It exits 0 with `wrap-up ok` or 1 with `wrap-up incomplete`. It reads git, the ledger and the clone's build places only, so it works on every host and offline. It deletes the places that don't count (`build-places`), the backstop for builds and drafts nobody finished.

`clm-checkpoint` offers `clm wrap-up` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-checkpoint` SHALL offer `clm wrap-up` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `orchestrator.checkpoint`'s requirements SHALL call `clm-checkpoint` itself, this call where the requirement states what a command prints or how it exits, and SHALL sit in `orchestrator.checkpoint`'s own files: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `orchestrator.checkpoint`'s own files
- WHEN it calls `clm-checkpoint`'s call with the arguments of `clm wrap-up` and two buffers
- THEN the buffers hold what `clm wrap-up` prints to standard output and to standard error there, and the call returns the status `clm wrap-up` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm wrap-up` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on
