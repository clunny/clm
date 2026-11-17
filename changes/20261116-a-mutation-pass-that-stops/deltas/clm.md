---
component: clm
---

## MODIFIED Requirements

### Requirement: verify-command <!-- base: af752fb68753fa62 -->
`clm` SHALL dispatch `clm verify [--due [--event work-done] [--dry-run] | --commit <sha> | --import | --bisect <sha>]` to `clm-verify`, for the CLM repository found from the current directory (`root-from-cwd`), and `clm --help` SHALL list it. It SHALL pass `clm-verify`'s call `/proc/meminfo` as the memory file, a minute of 60 seconds, a grace of 10 seconds and a stop request it sets when `clm verify` gets `SIGINT` or `SIGTERM` while the mutation pass runs (`verify/mutation-pass`), a further such signal during the pass changing nothing; once the call returns after such a signal, `clm verify` SHALL exit with 128 plus the signal's number. A signal at any other time ends it as it would any process.

#### Scenario: dispatch
- GIVEN a repository `clm init` has just set up
- WHEN `clm verify --due` runs in a subdirectory of it
- THEN it prints `verify off: no [verify] in origin/main:clm.toml` and exits 0

#### Scenario: stopped by hand
- GIVEN a repository whose `[verify]` sets `mutate` to a command that writes one requirement's verdict and a file, and then sleeps, with two requirements due
- WHEN `clm verify --due` gets `SIGTERM` once that file appears
- THEN the command no longer runs, the ledger's new `mutated` record holds one requirement's verdict and the other under `left`, stderr holds `mutation: stopped`, stdout holds `verify passed`, and it exits 143

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
