---
component: gate
base: { contract: "a17e55b995eef1fb" }
---

## MODIFIED Contract

`clm gate --base <ref> [--approved]` decides whether `HEAD` may merge into `<ref>`. Its first line is the branch's kind, `spec review` for a spec branch that needs a human's review, `spec` for one that doesn't, `code` or `mixed`, then any violations in `check`'s format, and it exits 0 to allow or 1 to block. `--approved` says the ledger holds an approval of this diff that the repository's policy accepts; `clm land` computes it from the fold (`land-locally`), and the gate itself reads no ledger and no host.

`clm fingerprint`, the fingerprint an approval is pinned to, is the child `gate.fingerprint`'s (crate `clm-fingerprint`). `clm-gate` re-exports its `Entry`, `parse_ls_tree`, `fingerprint_of`, `adds_change`, `added_changes` and `fingerprint`, so `land`, `ledger`, `web` and `crates/clm` call them as `clm_gate::...` (`fingerprint-reexported`).

`clm-gate` also offers `settings_unchanged(before, after)`: given `clm.toml`'s bytes on each side of a diff (`Option<&[u8]>`, `None` where the file doesn't exist), whether its settings are the same once the upgrade notes `upgraded` and `declined_migrations`, comments and formatting are set aside, false when either side isn't UTF-8 TOML. `land` reads it to count a land whose `clm.toml` changed only so as checked (`land/settings-unchanged-is-checked`).

`clm-gate` also offers `foreign_files(repo, base, head, component)`: the code files `head` changes against `base` that aren't `component`'s build's to change, each with its owner, none when the root at the merge base sets no `wiring` (`own-files`). The gate itself reads no branch name, so `clm gate` doesn't report them: `clm land`, which knows the branch, applies the rule (`land/build-holds-own-files`), as it applies `files-outside-spec-paths` to a spec branch.

`clm-gate` offers `clm gate` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-gate` SHALL offer `clm gate` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `gate`'s requirements SHALL call `clm-gate` itself, this call where the requirement states what a command prints or how it exits, and SHALL sit in `gate`'s own files: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `gate`'s own files
- WHEN it calls `clm-gate`'s call with the arguments of `clm gate` and two buffers
- THEN the buffers hold what `clm gate` prints to standard output and to standard error there, and the call returns the status `clm gate` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm gate` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on
