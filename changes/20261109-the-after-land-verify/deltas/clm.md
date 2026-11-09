---
component: clm
---

## MODIFIED Requirements

### Requirement: verify-command <!-- base: f591ec161ef02b26 -->
`clm` SHALL dispatch `clm verify [--due [--event work-done] [--dry-run] | --commit <sha> | --import | --bisect <sha>]` to `clm-verify`, for the CLM repository found from the current directory (`root-from-cwd`), and `clm --help` SHALL list it.

#### Scenario: dispatch
- GIVEN a repository `clm init` has just set up
- WHEN `clm verify --due` runs in a subdirectory of it
- THEN it prints `verify off: no [verify] in <base>:clm.toml`, `<base>` being the base `verify/settings-from-main` reads, and exits 0

#### Needs
- verify/verify-due
- verify/verify-run
- verify/verify-import
- verify/verify-commit
- verify/verify-bisect
- verify/settings-from-main

#### Serves
- capability/slow-tests-after-land
