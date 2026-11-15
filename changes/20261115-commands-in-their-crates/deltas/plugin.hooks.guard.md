---
component: plugin.hooks.guard
base: { contract: "eabc0de726d48fc8" }
---

## MODIFIED Contract

The rules behind `clm hook guard` (`hook-logic-in-clm`): which Bash commands the guard asks the human about or blocks, and why. It asks the human to confirm `clm approve`, `clm capabilities decide` and `clm architecture decide` (`approval-policy`), blocks pushes to `main` from a repository under CLM (`guard-blocks-push-to-main`), release runs (`release-by-dispatch`) and a `clm land` whose output is piped (`guard-refuses-piped-land`), and asks about `clm defect` when the report leaves the project. Crate `clm-guard`: `guard(event)`, which returns a `Decision` (`Allow`, `Ask(reason)` or `Block(reason)`); `guard_within(event, budgets)`, the same judgement with the time `Budgets` its git queries and host lookups may spend, `guard` being `guard_within` with `Budgets::HOOK` (`hook-budgets`); and `ask_json(reason)`, the `ask` permission decision's JSON; `crates/clm`'s `clm hook guard` prints and exits by them. How a command is read before the rules judge it is `plugin.hooks.guard.shell`.

`clm-guard` offers `clm hook guard` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root and the text on standard input, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-guard` SHALL offer `clm hook guard` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides and the text on standard input, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `plugin.hooks.guard`'s requirements SHALL call `clm-guard` itself, this call where the requirement states what a command prints or how it exits, and SHALL sit in `plugin.hooks.guard`'s own files: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `plugin.hooks.guard`'s own files
- WHEN it calls `clm-guard`'s call with the arguments of `clm hook guard` and two buffers
- THEN the buffers hold what `clm hook guard` prints to standard output and to standard error there, and the call returns the status `clm hook guard` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm hook guard` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on
