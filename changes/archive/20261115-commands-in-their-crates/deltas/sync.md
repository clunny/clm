---
component: sync
base: { contract: "7e8d80dfa638bbcd" }
---

## MODIFIED Contract

`clm rebase` brings the current branch up to date with `origin/main`: it fetches, rebases, regenerates derived files and runs `clm check`. It exits 0 when the branch is up to date and passes, 1 when something is left for the caller to resolve (with what and where), and 2 when git fails. Running it again after resolving carries on. It never pushes. Its last line of stdout always states the outcome: `rebase ok`, `rebase incomplete` or `rebase failed`, for exit codes 0, 1 and 2, so a caller that can't read the exit code (an unattended session, `builds-in-subagents`) still knows it.

On a branch that adds an applied change, or a change it archived itself after applying it (`archive-in-spec-pr`), it rebases a stale delta itself where it can (`stale-delta-merge`): each section of the delta whose edits don't overlap what moved the section since the delta's base, `main`'s edits or an earlier change's on the same branch, is merged three-way, written into the delta with its new base, and committed as `Merge <movers> into <change>`, naming what moved it, printing `merged <element> of <component> into <delta path>`. A section whose edits overlap, one the delta removes or that is absent from the tree, and one whose base text git doesn't have are left for the caller, with what moved in each and who moved it (`stale-delta-diff`). The delta of another live `proposed` or `approved` change that the branch's apply made stale, on `main` or on the branch, is merged the same way against the branch's applied text and committed on its own (`stale-delta-merge`). A `git` command that fails on a transient `index.lock` is run again before the run fails (`retry-index-lock`).

On a branch that adds a change recording `renames` (`rename`), it regenerates the branch instead of rebasing it (`regenerate-renames`): the branch is one commit on `origin/main` holding what `clm rename` produces there, so a moved `main` never leaves a rename to merge by hand (`regenerate-dont-merge`).

On `impl/*` branches it also reports a changed component spec (`spec-changed`), archives complete changes (`archive-complete`, `archive-on-sync`), and files reconsider and proposal items (`file-reconsider-item`, `health-advises`, `file-proposal-items`, `web-proposals-as-ledger-records`). It files no item for a component over its size budget: the component shows as oversized in `clm status`, `clm size` and `clm health` (`oversized-shown-not-filed`).

`clm autosync` pushes a snapshot of the spec work on the current `spec/*` branch to `origin` as `autosync/<branch>` (`autosync`).

`clm-sync` offers each of `clm rebase` and `clm autosync` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-sync` SHALL offer each of `clm rebase` and `clm autosync` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `sync`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `sync`'s own files and call `clm-sync` itself, this call where the requirement states what a command prints or how it exits: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `sync`'s own files
- WHEN it calls `clm-sync`'s call with the arguments of `clm rebase` and two buffers
- THEN the buffers hold what `clm rebase` prints to standard output and to standard error there, and the call returns the status `clm rebase` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm rebase` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
