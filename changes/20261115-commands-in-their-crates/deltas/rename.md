---
component: rename
base: { contract: "aa1482ba04180437" }
---

## MODIFIED Contract

`clm rename <change-id>` carries out the renames a live, `proposed` change records in its `change.md`'s `renames` (`model/rename-records`): it generates the change's deltas, applies them, rewrites what cites the renamed IDs and terms, test markers included, rebases the other live changes' deltas whose bases the rename moves (`other-deltas`), archives the change as `done` and deletes the items its `resolves` names, so the branch lands with nothing left pending but the requirements whose code still names an ID the rename rewrote in their text (`marker-rewrite`). It writes the working tree and commits nothing. It prints one `renamed <old> -> <new>` line per rename, then one `rebased <element> of <component> in <delta path>` line per element of another change's delta it rebased, then one `still names <old> <file>:<line>` line per mention it leaves for the author to judge, and exits 0; it exits 1 on a refusal, writing nothing.

The crate `clm-rename` exposes `rename(root, change_id)`, the command on a checkout, and `replays(repo, base, head)`, whether `head`'s tree is exactly what `rename` produces from `base`'s with the change `head` adds (`replay`). `gate` calls `replays` to classify a branch, and `sync` calls both to regenerate a rename branch instead of rebasing it (`regenerate-dont-merge`).

`clm-rename` offers `clm rename` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-rename` SHALL offer `clm rename` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `rename`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `rename`'s own files and call `clm-rename` itself, this call where the requirement states what a command prints or how it exits: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `rename`'s own files
- WHEN it calls `clm-rename`'s call with the arguments of `clm rename` and two buffers
- THEN the buffers hold what `clm rename` prints to standard output and to standard error there, and the call returns the status `clm rename` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm rename` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
