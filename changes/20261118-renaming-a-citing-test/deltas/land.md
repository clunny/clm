---
component: land
---

## MODIFIED Requirements

### Requirement: deleted-tests-approved <!-- base: f6971347d8b31843 -->
`clm land <branch>`, for a branch not named `spec/*`, SHALL exit 1 when `check.coverage`'s `deleted_tests`, over the texts at the merge base and at the head of the files the branch adds, changes or deletes, returns a test no approved list names, printing for each `<file>: land/deleted-tests-approved: test <name>, which cited <id>[, <id>...], is gone and no approved list names it. Keep the test under its name, or under a new name in its file citing every requirement it cited, or leave the requirement unbuilt and add a question item`. An *approved list* is a `changes/<id>/deleted-tests.txt` or `changes/archive/<id>/deleted-tests.txt` in the fetched base's tree, and it names a test by a list line (`gate/cuts`, read with its `list_lines`) whose file is the gone test's and whose name agrees with it as `bare_name` gives them. It SHALL do so before it takes the land's lock, merges or runs any check, and it SHALL push and append nothing. `clm land --check-only` SHALL print the same lines for `HEAD` against the local `origin/main`, on a branch not named `spec/*` and on a detached `HEAD`, and exit 1 before it runs the check command; with no local `origin/main` it SHALL judge nothing. A branch named `spec/*` SHALL NOT be judged by this rule.

#### Scenario: a deleted test
- GIVEN `main` holding `tests/t.rs` with tests `one` and `two`, each citing `a/x`, a branch `impl/a` whose `tests/t.rs` holds only `one`, and a check command that records that it ran
- WHEN `clm land impl/a` runs
- THEN it exits 1 naming `tests/t.rs`, `two` and `a/x` under `land/deleted-tests-approved`, the check command never ran, and `main` on `origin` and the ledger are unchanged

#### Scenario: a test an approved list names
- GIVEN the same, `main` also holding `changes/c/deleted-tests.txt` with the line `DELETE tests/t.rs::two`
- WHEN `clm land impl/a` runs
- THEN it lands

#### Scenario: a list in the archive
- GIVEN the same, the list at `changes/archive/c/deleted-tests.txt` instead
- WHEN `clm land impl/a` runs
- THEN it lands

#### Scenario: a moved test
- GIVEN the first scenario's `main`, and a branch `impl/a` that moves `two`, under its name and with its marker, into a new file `tests/u.rs`
- WHEN `clm land impl/a` runs
- THEN it lands

#### Scenario: a renamed test
- GIVEN the first scenario's `main`, and a branch `tidy` that renames `two` to `second`, keeping its marker
- WHEN `clm land tidy` runs
- THEN it lands

#### Scenario: a renamed test that drops its citation
- GIVEN the first scenario's `main`, and a branch `tidy` that renames `two` to `second` and takes its marker off
- WHEN `clm land tidy` runs
- THEN it exits 1 naming `two` under `land/deleted-tests-approved`

#### Scenario: a spec branch
- GIVEN a branch `spec/r` holding only the committed result of `clm rename r`, which rewrote the marker in the title of a test in `a.test.ts`
- WHEN `clm land spec/r` runs
- THEN this rule refuses nothing

#### Scenario: the pre-check
- GIVEN the first scenario's branch checked out
- WHEN `clm land --check-only` runs
- THEN it prints the same line, exits 1, and the check command never ran

#### Serves
- architecture/removal-needs-a-yes
- capability/land-safely

#### Needs
- check.coverage/test-lists
- gate/cuts
