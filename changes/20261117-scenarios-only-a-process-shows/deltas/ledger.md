---
component: ledger
---

## MODIFIED Requirements



### Requirement: verify <!-- base: ff31400196f4b3f8 -->
`clm ledger verify [--since <rev>]` SHALL report, in `check`'s format, `ledger/unrecorded-land` for each commit on `main`'s first-parent chain after `<rev>`, merge or not, that no `landed` record names, except one whose message starts `Release v`, and `ledger/unchecked-land` for each landed commit whose land's code key no `checked` record names, exiting 0 with no output when there are none and 1 otherwise. A land's code key SHALL be its record's `code_key`, or, only when the record has none, its legacy key: the commit's tree hashed as `code_key` does with `docs/spec-format.md` left out as well (`legacy-code-key`). `clm-ledger` SHALL offer the same findings, as violations, for `clm check` to report: those of `main` when the checked-out branch is `main` and the repository has a ledger branch, and none otherwise. That `clm check` reports them is the root's to state and prove, since `crates/clm` hands one crate's findings to the other's call (`clm/check-across-components`).

#### Scenario: clean history
- GIVEN a `main` whose two merge commits each have a `landed` record and whose code keys have `checked` records
- WHEN `clm ledger verify` runs
- THEN it prints nothing and exits 0

#### Scenario: push behind clm's back
- GIVEN a `main` with a merge commit no `landed` record names
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unrecorded-land` naming that commit and exits 1

#### Scenario: direct push
- GIVEN a landed `main` and then a single-parent commit pushed straight onto it, which no `landed` record names
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unrecorded-land` naming that commit and exits 1

#### Scenario: release exempt
- GIVEN a `main` whose only unrecorded commits are a merge commit with the message `Release v0.9.0` and, later, a single-parent commit with the message `Release v0.10.0`
- WHEN `clm ledger verify` runs
- THEN it prints nothing and exits 0

#### Scenario: unchecked land
- GIVEN a `main` whose merge commit has a `landed` record with code key `k` and no `checked` record names `k`
- WHEN `clm ledger verify` runs
- THEN it prints `ledger/unchecked-land` naming that commit and exits 1

#### Scenario: recorded key read
- GIVEN a landed merge commit whose `landed` record's `code_key` is `k`, a `checked` record for `k`, and no `checked` record for the commit's own computed code key
- WHEN `clm ledger verify` runs
- THEN it prints nothing and exits 0

#### Scenario: legacy key for a land recorded without one
- GIVEN a landed merge commit whose tree holds `docs/spec-format.md`, whose `landed` record has no `code_key` field, a `checked` record for the commit's legacy key, and no `checked` record for `code_key(root, commit)`
- WHEN `clm ledger verify` runs
- THEN it prints nothing and exits 0

#### Scenario: offered to the check on main only
- GIVEN a repository on `main` with a ledger branch and the history of *push behind clm's back*
- WHEN a test reads the findings `clm-ledger` offers `clm check`, and again with another branch checked out
- THEN the first hold `ledger/unrecorded-land`, and the second are empty

#### Serves
- capability/land-safely
- architecture/git-is-truth
