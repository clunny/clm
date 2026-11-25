---
component: land
---

## MODIFIED Requirements

### Requirement: deleted-tests-approved <!-- base: 00650f33ee12321a -->
`clm land <branch>`, whatever the branch's name, SHALL exit 1 when `check.coverage`'s `deleted_tests`, over the texts at the merge base and at the head of the files of the two trees under the CLM root that can hold a test (`check.coverage/test-holding-paths`), named from it, returns a test no approved list names, printing for each `<file>: land/deleted-tests-approved: test <name>, which cited <id>[, <id>...], is gone and no approved list names it. Keep the test under its name, citing any requirement, even one it didn't cite; or rename it in its file to a name no test carried at the merge base, citing every requirement it cited, and delete no uncited test of that file on the same branch; or leave the requirement unbuilt and add a question item`; the files that can hold no test hold none on either side, so this is the verdict `clm coverage --deleted-tests` gives for the branch. Re-citing a test to another requirement under its name loses no test, since `check.coverage/test-lists` counts a file's tests of a name whatever requirements they cite, so it needs no list line; only a test renamed in its file must cite every requirement it cited, and it counts as renamed only while its file gains a new test for each uncited test the file loses, since `check.coverage/test-lists` lets a gone uncited test take one of its file's new tests first, so the advice asks a branch that renames a citing test to delete no uncited test of that file beside it. A branch whose diff `rename/replay` accepts from the merge base, asked as the gate asks it of a head that adds exactly one change whose `change.md` sets `renames`, SHALL NOT be judged, whichever paths its rewritten markers sit under, since those markers are what the replay vouches for; every other branch SHALL be, one the gate reads as `spec` for another reason, such as a `clm.toml` change `gate/approval-setting` counts as a spec change, included. An *approved list* is a `changes/<id>/deleted-tests.txt` or `changes/archive/<id>/deleted-tests.txt` in the fetched base's tree, and it names a test by a list line (`gate/cuts`, read with its `list_lines`) whose file is the gone test's and whose name agrees with it as `bare_name` gives them. It SHALL do so before it takes the land's lock, merges or runs any check, and it SHALL push and append nothing. When asking `rename/replay` fails, as when git fails, it SHALL exit 2 with its error, before the lock. A head with no merge base with the base, whose land its merge refuses anyway, and a head whose diff touches no file under the CLM root that can hold a test, SHALL NOT be refused by this rule; a file whose text isn't UTF-8 SHALL be read as holding no test, as `clm coverage --deleted-tests` reads it. `clm land --check-only` SHALL judge `HEAD` against the local `origin/main` as a land judges its head, reading no branch name, print the same lines and exit 1 before it runs the check command; with no local `origin/main` it SHALL judge nothing.

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

#### Scenario: a re-cited test
- GIVEN the first scenario's `main`, and a branch `impl/a` whose `tests/t.rs` keeps `one` as it is and `two` under its name, its marker citing `b/y` in place of `a/x`
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

#### Scenario: a rename beside a deleted uncited test
- GIVEN the first scenario's `main`, its `tests/t.rs` also holding a test `helper` that cites nothing, and a branch `tidy` that renames `two` to `second`, keeping its marker, and deletes `helper`
- WHEN `clm land tidy` runs
- THEN it exits 1 naming `two` under `land/deleted-tests-approved`, its advice asking that no uncited test of that file be deleted on the same branch

#### Scenario: a new name another file carries
- GIVEN the first scenario's `main`, also holding `tests/u.rs` with a test `kept` citing `a/x`, and a branch `tidy` that renames `two` to `kept` in `tests/t.rs`, keeping its marker, and leaves `tests/u.rs` as it is
- WHEN `clm land tidy` runs
- THEN it exits 1 naming `two` under `land/deleted-tests-approved`, as `clm coverage --deleted-tests` lists it

#### Scenario: a spec branch
- GIVEN a branch `spec/r` holding only the committed result of `clm rename r`, which rewrote the marker in the title of a test in `a.test.ts`
- WHEN `clm land spec/r` runs
- THEN this rule refuses nothing

#### Scenario: the pre-check
- GIVEN the first scenario's branch checked out
- WHEN `clm land --check-only` runs
- THEN it prints the same line, exits 1, and the check command never ran

#### Scenario: a spec branch that deletes a test
- GIVEN `main` holding `docs/t.test.ts` with tests `one` and `two`, each citing `a/x` in its title and `clm.toml` setting `approval = "other"`; a branch `spec/s` whose `docs/t.test.ts` holds only `one`; and a branch `t` that does the same and also sets `approval = "any"`
- WHEN `clm land spec/s` runs, and `clm land t`, and `clm land --check-only` in a checkout of `spec/s`
- THEN each exits 1 naming `docs/t.test.ts` and `two` under `land/deleted-tests-approved`, the last before its check command runs

#### Scenario: what it doesn't read
- GIVEN in turn: a branch changing only `README.md` and a binary `assets/logo.png`; and `main` holding a test file that isn't UTF-8, holding a test citing `a/x`, which a branch `tidy` deletes
- WHEN `clm land` runs for each
- THEN this rule refuses neither

#### Serves
- architecture/removal-needs-a-yes
