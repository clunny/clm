---
component: clm
---

## MODIFIED Requirements

### Requirement: verify-command <!-- base: 31b600ddb79d3417 -->
`clm` SHALL dispatch `clm verify [--due [--event work-done] [--dry-run] | --commit <sha> | --import | --bisect <sha>]` to `clm-verify`, for the CLM repository found from the current directory (`root-from-cwd`), and `clm --help` SHALL list it. It SHALL call `clm-verify` with its standard call (`verify/mutation-pass`), whose stop request it sets when `clm verify` gets `SIGINT` or `SIGTERM` once it has printed its outcome line, whatever that line says, until `clm-verify`'s call returns, the mutation pass, the clearing of the verify tree and the filing of what the pass found (`verify/files-unproven`) together, a further such signal in that time changing nothing; once the call returns after such a signal, `clm verify` SHALL exit with 128 plus the signal's number. A signal at any other time ends it as it would any process.

#### Scenario: dispatch
- GIVEN a repository `clm init` has just set up
- WHEN `clm verify --due` runs in a subdirectory of it
- THEN it prints `verify off: no [verify] in origin/main:clm.toml` and exits 0

#### Scenario: stopped by hand
- GIVEN a repository whose `[verify]` sets `mutate` to a command that writes one requirement's verdict and a file, and then sleeps, with two requirements due
- WHEN `clm verify --due` gets `SIGTERM` once that file appears
- THEN the command no longer runs, the ledger's new `mutated` record holds one requirement's verdict and the other under `left`, stderr holds `mutation: stopped`, stdout holds `verify passed`, and it exits 143

#### Scenario: stopped while it files after a run that wasn't due
- GIVEN a repository in `local` mode whose `[verify]` sets `mutate`, with no trigger holding, a `mutated` record for `origin/main`'s head holding an `unproven` entry at its requirement's current version, and a root `clm.toml` whose `[land] check` writes a file, then sleeps for two seconds and exits 0
- WHEN `clm verify --due` gets `SIGTERM`, sent to it alone, once that file appears
- THEN stdout holds `verify not-due`, stderr holds `unproven: filed 1 items for 1 requirements`, `origin` holds no branch `unproven/<short sha>`, and it exits 143

#### Needs
- verify/verify-due
- verify/verify-run
- verify/verify-import
- verify/verify-commit
- verify/verify-bisect
- verify/settings-from-main
- verify/mutation-pass

#### Serves
- capability/slow-tests-after-land
