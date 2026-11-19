---
component: check
---

## MODIFIED Requirements

### Requirement: coverage-lists <!-- base: ee87592c885559fe -->
After the obsolete lines `clm coverage` SHALL print one line `<id> uncounted <file>:<line>[ <file>:<line>...]` for each entry `uncounted` returns for the checkout, so a build finds the markers to move. `clm coverage --obsolete-tests` SHALL print, in place of the coverage, the line `# tests to delete: <n>; tests that only lose a marker: <m>` and then one line per test `obsolete_tests` returns for the checkout, in its order: `DELETE <file>::<name>`, a tab and `cites only removed: <id>[, <id>...]` when every marker on the test's lines cites a removed requirement, and otherwise `UNCITE <file>::<name>`, a tab and `loses: <id>[, <id>...]`. `clm coverage --deleted-tests <base>` SHALL print `# tests deleted since <base>: <n>` and then one line `DELETED <file>::<name>`, a tab and the IDs it cited, per test `deleted_tests` returns for the files of the revision `<base>` against the working tree's, the revision's files being those under the CLM root and both sides' named from the CLM root, as the working tree's are and as `clm land` names them (`land/deleted-tests-approved`), so the two give one verdict in a repository whose CLM root is a subdirectory of its git repository too. Each SHALL exit 0; `--deleted-tests` SHALL exit 1 naming the revision when `<base>` resolves to no commit; the two flags together SHALL exit 2 as a usage error; and a graph that doesn't load SHALL fail as `clm coverage` fails.

#### Scenario: an uncounted marker
- GIVEN `a/x`, and `src/lib.rs` holding `req:a/x` at its current version in the body of a function with no test attribute, cited nowhere else
- WHEN `clm coverage` runs
- THEN it prints `a/x unbuilt`, and after the obsolete lines `a/x uncounted src/lib.rs:<line>`

#### Scenario: a removed requirement's tests
- GIVEN an `applied` change that removes `a/x`, a test `both` citing `a/x` and `a/y` and after it a test `only_x` citing `a/x` alone, both in `tests/t.rs`
- WHEN `clm coverage --obsolete-tests` runs
- THEN it prints `# tests to delete: 1; tests that only lose a marker: 1`, then `UNCITE tests/t.rs::both` with `loses: a/x`, then `DELETE tests/t.rs::only_x` with `cites only removed: a/x`

#### Scenario: a branch that deletes a test
- GIVEN a commit `b` whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and a working tree whose `tests/t.rs` holds only `one`
- WHEN `clm coverage --deleted-tests b` runs
- THEN it prints `# tests deleted since b: 1` and `DELETED tests/t.rs::two` with `a/x`

#### Scenario: a CLM root in a subdirectory
- GIVEN a git repository whose CLM root is `sub/`, a commit `b` whose `sub/tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and whose `other/tests/u.rs` holds a test `three` citing `a/x`, and a working tree whose `sub/tests/t.rs` holds `one` and in place of `two` a test `second` citing `a/x`
- WHEN `clm coverage --deleted-tests b` runs in `sub/`
- THEN it prints `# tests deleted since b: 0` and no `DELETED` line

#### Scenario: an unknown base
- GIVEN no commit `nope`
- WHEN `clm coverage --deleted-tests nope` runs
- THEN it exits 1 naming `nope`

#### Serves
- capability/prove-by-tests
- architecture/proven-by-tests

#### Needs
- check.coverage/test-lists
- check.coverage/markers-in-tests
- check.coverage/owner
