---
component: land.approval
base: { contract: "29f5e0c79bd9c68a" }
---

## MODIFIED Contract

Whether a branch's diff is approved, and whether a reviewer's request holds it, for `clm land` (`land`) and `clm approve`, and the signing of approvals and lands (`signed-ledger-records`). An approval names the branch's *fingerprint*, `clm fingerprint --base origin/main` at its head, and counts only while the branch still has it (`approve-pins-the-review`). Who may approve is `config`'s `approval`, read from the base the land lands on (`approval-policy`, `land-policy-from-base`). A reviewer's `changes_requested` record for the current fingerprint, made in the web app (`web-review-feedback-as-ledger-records`), holds the branch until its fingerprint changes or the requester approves it. In a repository whose `main` holds `docs/allowed_signers`, approvals and lands are signed with the person's SSH key, and an approval counts only when its signature proves its writer. Crate `clm-approval` offers `clm-land`: a branch's head, fingerprint and change IDs, computed one way for `clm land` and `clm approve`; `approve`; the policy at a base; whether a fingerprint's approvals count, and why each doesn't; the refusal a request for changes makes; and whether a base is signed, the key check and signing a record. `clm-land` re-exports `approve`.

`clm-approval` offers `clm approve` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-approval` SHALL offer `clm approve` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `land.approval`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `land.approval`'s own files and call `clm-approval` itself, this call where the requirement states what a command prints or how it exits: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `land.approval`'s own files
- WHEN it calls `clm-approval`'s call with the arguments of `clm approve` and two buffers
- THEN the buffers hold what `clm approve` prints to standard output and to standard error there, and the call returns the status `clm approve` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm approve` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
