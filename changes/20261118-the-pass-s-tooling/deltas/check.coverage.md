---
component: check.coverage
base: { contract: "428109b95f0ef4cb" }
---

## MODIFIED Contract
How far the code has caught up with the spec, for `check` and for every component that reads it (`check-coverage-split`). A component whose code calls this library names `check.coverage` in its `depends_on` and imports `clm-coverage`, so its build is given this Contract. `clm-check` re-exports none of it. The states it computes, *current*, *behind*, *unbuilt* and *obsolete*, and a change's being *complete*, are defined in `check`'s Contract, which every build of this component reads among its ancestors.

Crate `clm-coverage`. Its library:
- `tests_in(path, text)`: the tests and named groups of tests in one file, each with its name and its lines, and `markers_outside_tests(path, text)`: the `req:` markers on none of those lines (`test-spans`).
- `markers_in(path, text)`: the `req:` markers in one file that count, those inside a test (`markers-in-tests`), none for Markdown; `scan(root)`, with the default feature `checkout`: the `req:` and `clm:remove-by` markers of every file a checkout lists, as `Scan { req, remove_by }`.
- `report(graph, markers)`: each requirement's `Coverage`, human-layer entries included (`coverage-report`).
- `pending(graph, markers)`: each component's pending requirements (`pending-report`).
- `change_progress(graph, markers)`: each change's `ChangeProgress { change, current, total, obsolete, complete }` (`change-progress`).
- `status_report(graph, markers)` and `complete_changes`, built on `change_progress` (`status-report`).
- `removed(graph)`: each requirement ID that a change outside the archive lists under `REMOVED Requirements` and no component holds, and each `capability/<slug>` that a change outside the archive lists under `removes_capabilities` and `CAPABILITIES.md` doesn't hold (`removed-capabilities`), with those changes' IDs sorted; what makes a marker *obsolete*, and what `check`'s `dangling-markers` and `removed-requirement-names` exempt or report.
- `matches(glob, path)`: whether a `paths` glob matches a repository-relative path, as `pending` and `check`'s `paths-disjoint` read `paths`.
- `uncounted(graph, outside)`, `obsolete_tests(graph, files)` and `deleted_tests(base_files, head_files)`: the markers outside tests by what they cite, the tests a change's removals leave with nothing to cite, and the citing tests one tree holds that another doesn't (`test-lists`); `clm coverage` prints them through `clm-check` (`check/coverage-lists`).
- `uncited_tests(files)`: the tests that hold no marker, for `clm health` (`check.health/rule-signals`). `deleted_tests` counts a test as gone when the files hold fewer marker-holding tests of its name, so a test moved to another file is not; `clm land` holds a code branch to it (`land/deleted-tests-approved`).
- `owner(graph, path)`: the component that owns a repository-relative path (`owner`).

The crate builds for `wasm32-unknown-unknown` with its default features disabled (`coverage-builds-for-wasm`).

## MODIFIED Requirements

### Requirement: test-lists <!-- base: 23fd9f71da938437 -->
`clm-coverage` SHALL offer, building without the `checkout` feature and taking files as paths and texts: `uncounted(graph, outside)`, returning, sorted by ID, each requirement or capability that exists, or that a change outside the archive removed (`removed`), and that a marker outside a test cites, with those markers' locations; `obsolete_tests(graph, files)`, returning each test (`test-spans`, groups left out) that holds a marker citing a requirement or capability a change outside the archive removed, sorted by file and then by line, with the removed IDs it cites, sorted, and whether every marker on the test's lines cites one; `deleted_tests(base_files, head_files)`, returning the tests of `base_files` that are *gone*, sorted the same way, with the IDs each cited; and `uncited_tests(files)`, returning each test (groups left out) with no marker on its lines, sorted the same way, with its first line. A test is named `<file>::<name>`. For `deleted_tests` a test's name is taken with the version of every marker in it left out (`@` and the eight hex digits), so re-citing a test whose title carries its marker doesn't rename it. Where `base_files` hold `n` tests of one name that each hold a marker and `head_files` hold `m` such tests, `m` less than `n`, the gone tests are the first `n - m`, in sorted order, of those `n` whose `<file>::<name>` no marker-holding test of `head_files` has; where `m` is `n` or more, none is gone. So a test moved to another file under its name is not gone, and a renamed test, a deleted one, and one that loses its last marker each are.

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
- GIVEN the same, with `two` renamed `second` at the head
- WHEN it runs
- THEN it returns `tests/t.rs::two`, since a program can't tell a rename from a removal

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
- GIVEN base files whose `a.test.ts` holds a test titled `keeps the list (req:a/x@11111111)`, and head files in which the title ends `(req:a/x@22222222)`
- WHEN `deleted_tests` runs
- THEN it returns nothing

#### Scenario: uncited tests
- GIVEN files whose `tests/t.rs` holds a test `one` citing `a/x` and, at line 9, a test `helper_rounds_up` with no marker, and whose `a.test.ts` holds a `describe` group with no marker around a test that cites `a/y`
- WHEN `uncited_tests` runs
- THEN it returns `tests/t.rs::helper_rounds_up` with line 9, and nothing else
