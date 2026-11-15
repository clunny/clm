---
component: check.unreferenced
base: { contract: "bd1851d2df8f34ce" }
---

## MODIFIED Contract

`clm unreferenced [--root <dir>] [--allow <entry>]... [--entry <attribute>]...` lists the public items of a workspace's libraries that nothing outside their own library uses, one line per item, and exits 1 when it lists any, so a project can name it as one of its dead-code detectors in `clm.toml` (`dead-code-detectors`, `public-item-detector`): `command = ["clm", "unreferenced"]`, with an `--allow` pair per accepted exception and an `--entry` pair per Rust attribute that marks an item called from outside Rust beyond those the detector knows (`unreferenced-entry-attributes`). A library is a Rust package with `src/lib.rs`, whose public items are its `pub` items, or a TypeScript or Python package, whose public items are the names its entry modules export (`unreferenced-package-api`). rustc's `dead_code` lint, `knip` and `vulture` each assume or see one package; in an application workspace a public item rarely has users outside it, and this detector covers that gap. A type that a used public item names in its signature counts as used (`unreferenced-signature-carriers`). It exits 0 when it lists nothing, including in a repository with no library, and 2 when it can't read the repository. The library call `scan(root, allow, entry)` returns the findings it prints.

`clm-unreferenced` offers `clm unreferenced` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-unreferenced` SHALL offer `clm unreferenced` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `check.unreferenced`'s requirements SHALL call `clm-unreferenced` itself, this call where the requirement states what a command prints or how it exits, and SHALL sit in `check.unreferenced`'s own files: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `check.unreferenced`'s own files
- WHEN it calls `clm-unreferenced`'s call with the arguments of `clm unreferenced` and two buffers
- THEN the buffers hold what `clm unreferenced` prints to standard output and to standard error there, and the call returns the status `clm unreferenced` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm unreferenced` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on
