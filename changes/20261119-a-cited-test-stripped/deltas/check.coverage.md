---
component: check.coverage
---

## MODIFIED Requirements

### Requirement: test-lists <!-- base: b02100ac306bdbf7 -->
`clm-coverage` SHALL offer, building without the `checkout` feature and taking files as paths and texts: `uncounted(graph, outside)`, returning, sorted by ID, each requirement or capability that exists, or that a change outside the archive removed (`removed`), and that a marker outside a test cites, with those markers' locations; `obsolete_tests(graph, files)`, returning each test (`test-spans`, groups left out) that holds a marker citing a requirement or capability a change outside the archive removed, sorted by file and then by line, with the removed IDs it cites, sorted, and whether every marker on the test's lines cites one; `deleted_tests(base_files, head_files)`, returning the tests of `base_files` that are *gone*, sorted the same way, with the IDs each cited; and `uncited_tests(files)`, returning each test (groups left out) with no marker on its lines, sorted the same way, with its first line. A test is named `<file>::<name>`. For `deleted_tests` a test's name is compared with every `req:` marker in it left out, ID and version, and the space before it, so re-citing a test whose title carries its marker doesn't rename it; `clm-coverage` SHALL offer that as `bare_name(name)`. A file's *gained* tests of a bare name number how many fewer of its tests of that name holding no marker the head's text of the file holds than its text in `base_files` does, but no more than the tests of that name holding a marker the head's text holds: that many of its citing tests at the head may be unmarked tests that gained a marker, and stand for no test the base held with one. Where `base_files` hold `n` tests of one bare name that each hold a marker and `head_files` hold `m` such tests less every file's gained tests of that name, `m` less than `n`, `n - m` tests are gone: taking the files in sorted order, each file gives as many of its tests of that name holding a marker, last first, as its tests of that name holding a marker at the head, less its gained tests of that name, fall short of those at the base, until `n - m` are taken; where `m` is `n` or more, none is gone. A test so taken *fits* a test of the head's text of its file that holds a marker, whose bare name no test of `base_files` (groups left out) carries, with a marker or without, and whose markers cite every ID the taken test's markers cite, versions left out. A test so taken SHALL instead count as *renamed*, and not be returned, as follows: taking the tests so taken in the order `deleted_tests` returns them, each counts as renamed when it and every earlier one counted renamed can each be paired with a different test it fits. So as many are renamed as can be, each head test standing for one at most. Here groups are left out on both sides. A test of the head's text of a file is *new* when no test of `base_files` carries its bare name, with a marker or without. A file's *stripped* tests of a bare name number how many fewer of its tests of that name holding a marker the head's text of the file holds than its text in `base_files` does, but no more than the tests of that name holding no marker the head's text holds: that many of its unmarked tests at the head may be cited tests that lost their marker, and stand for no test the base held without one. A file's *gone unmarked* tests number, for each bare name, how many more of its tests of `base_files` holding no marker carry that name than the head's text of the same file accounts for, which is its tests of that name there holding no marker less its stripped tests of that name, added to its gained tests of that name; none where it is no more. A gone unmarked test could have become any new test of its file, so it takes one before a taken test does: a taken test SHALL count as renamed only when, with it counted, the tests counted renamed in its file would number no more than the file's new tests less its gone unmarked tests, and the tests of its bare name counted renamed in its file would number no more than how many fewer tests of that name, with a marker or without, the head's text of the file holds than its text in `base_files` does: a test whose name its file still carries as often as before was not renamed. A test moved to another file under its name is not gone, unless that file's gained tests of the name take its place, nor is one renamed in its own file that cites at least what it cited; a test renamed and moved to another file at once, one renamed that cites less, one whose place a test the base held with no marker takes by gaining one, under its name or a new one, a deleted one, and one that loses its last marker each are. Of two taken tests that a head test both fits, one deleted and one renamed, the trees can't tell which is which, and the earlier counts as renamed. Where a file's text in `base_files` holds a test of a name with a marker and one of that name with none, and its head's text holds only one test of that name, with a marker, the trees can't tell the unmarked one deleted from the cited one deleted as the other gains a marker, and the cited one counts as gone. Where it holds a test of a name with a marker and one of that name with none, and its head's text holds only one test of that name, with no marker, beside a new test that holds a marker, the trees can't tell the cited one renamed beside the unmarked one kept from the cited one losing its last marker as the unmarked one is deleted, and the cited one counts as gone.

#### Scenario: uncounted
- GIVEN `a/x` existing, `a/gone` removed by an `applied` change, and markers outside a test citing `a/x` at `src/lib.rs:3` and `a/gone` at `src/lib.rs:9`
- WHEN `uncounted` runs
- THEN it returns `a/gone` with `src/lib.rs:9`, then `a/x` with `src/lib.rs:3`

#### Scenario: a removed requirement's tests
- GIVEN an `applied` change that removes `a/x`, a test `only_x` citing `a/x` alone and a test `both` citing `a/x` and `a/y`, both in `tests/t.rs`, `both` first
- WHEN `obsolete_tests` runs
- THEN it returns `tests/t.rs::both` with `a/x`, not every marker, then `tests/t.rs::only_x` with `a/x`, every marker

#### Scenario: a deleted test
- GIVEN base files whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and head files whose `tests/t.rs` holds only `one`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x`

#### Scenario: a renamed test
- GIVEN the same, with `two` renamed `second` at the head, still citing `a/x`
- WHEN it runs
- THEN it returns nothing

#### Scenario: a renamed test that cites less
- GIVEN base files whose `tests/t.rs` holds tests `one` citing `a/x` and `two` citing `a/x` and `a/y`, and head files whose `tests/t.rs` holds `one` and `second`, `second` citing `a/x` alone
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x` and `a/y`

#### Scenario: a test renamed into another file
- GIVEN the base files of *a deleted test*, and head files whose `tests/t.rs` holds only `one` and whose `tests/u.rs` holds `second`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two`

#### Scenario: one new name for two gone tests
- GIVEN base files whose `tests/t.rs` holds tests `one`, `two` and `three`, each citing `a/x`, and head files whose `tests/t.rs` holds `one` and `second`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::three` alone

#### Scenario: a test that gains its first marker
- GIVEN base files whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and `quiet`, with no marker, and head files whose `tests/t.rs` holds `one` and `quiet`, `quiet` now citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x`

#### Scenario: an uncited test renamed as it gains its first marker
- GIVEN base files whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and `quiet`, with no marker, and head files whose `tests/t.rs` holds `one` and `quiet_now`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x`

#### Scenario: an uncited test renamed to a name it doesn't cite under
- GIVEN base files whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and `quiet`, with no marker, and head files whose `tests/t.rs` holds `one`, `second`, citing `a/x`, and `quiet_now`, with no marker
- WHEN `deleted_tests` runs
- THEN it returns nothing

#### Scenario: an uncited test deleted beside a rename
- GIVEN base files whose `tests/t.rs` holds tests `one` and `two`, each citing `a/x`, and `quiet`, with no marker, and head files whose `tests/t.rs` holds `one` and `second`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x`

#### Scenario: two renames that each keep their citations
- GIVEN base files whose `tests/t.rs` holds tests `first` citing `a/x` and `second` citing `a/x` and `a/y`, and head files whose `tests/t.rs` holds, in this order, `wide` citing `a/x` and `a/y` and `narrow` citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns nothing

#### Scenario: a moved test
- GIVEN the base files of *a deleted test*, and head files whose `tests/t.rs` holds only `one` and whose `tests/u.rs` holds `two`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns nothing

#### Scenario: a name two files share
- GIVEN base files whose `tests/t.rs` and `tests/u.rs` each hold a test `two` citing `a/x`, and head files in which only `tests/u.rs` holds it
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two`

#### Scenario: a test that loses its marker
- GIVEN the base files of *a deleted test*, and head files whose `tests/t.rs` holds `one` and `two`, `two` with no marker
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two`

#### Scenario: a title that carries its marker
- GIVEN base files whose `a.test.ts` holds a test titled `keeps the list (req:a/x@11111111)`, and head files in which the title ends `(req:a/z@22222222)`
- WHEN `deleted_tests` runs
- THEN it returns nothing

#### Scenario: one of two tests of a name in one file
- GIVEN base files whose `a.test.ts` holds two tests titled `two`, each citing `a/x`, and head files in which it holds one
- WHEN `deleted_tests` runs
- THEN it returns `a.test.ts::two` once

#### Scenario: a cited test deleted beside an uncited test of its name
- GIVEN base files whose `a.test.ts` holds two tests titled `two`, the first citing `a/x` and the second with no marker, and head files in which it holds one test titled `two`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `a.test.ts::two` with `a/x`

#### Scenario: one of two tests of a name renamed
- GIVEN base files whose `a.test.ts` holds two tests titled `two`, each citing `a/x`, and head files in which it holds one test titled `two` and one titled `second`, each citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns nothing

#### Scenario: a test that loses its last marker beside a new citing test
- GIVEN the base files of *a deleted test*, and head files whose `tests/t.rs` holds `one`, citing `a/x`, `two`, with no marker, and `second`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `tests/t.rs::two` with `a/x`

#### Scenario: a cited test stripped beside a deleted uncited test of its name
- GIVEN base files whose `a.test.ts` holds two tests titled `two`, the first citing `a/x` and the second with no marker, and head files in which it holds one test titled `two`, with no marker, and one titled `second`, citing `a/x`
- WHEN `deleted_tests` runs
- THEN it returns `a.test.ts::two` with `a/x`

#### Scenario: uncited tests
- GIVEN files whose `tests/t.rs` holds a test `one` citing `a/x` and, at line 9, a test `helper_rounds_up` with no marker, and whose `a.test.ts` holds a `describe` group with no marker around a test that cites `a/y`
- WHEN `uncited_tests` runs
- THEN it returns `tests/t.rs::helper_rounds_up` with line 9, and nothing else
