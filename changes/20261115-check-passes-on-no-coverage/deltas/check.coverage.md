---
component: check.coverage
base: { contract: "81649e9551adda97" }
---

## MODIFIED Contract

How far the code has caught up with the spec, for `check` and for every component that reads it (`check-coverage-split`). A component reaches it one of two ways. One whose code calls this library names `check.coverage` in its `depends_on` and imports `clm-coverage`, so its build is given this Contract: `check`, `check.affected`, `check.health`, `rename`, `gate` and `verify` do, and so does any component that comes to read coverage. `clm-check` passes none of this library on. The states it computes, *current*, *behind*, *unbuilt* and *obsolete*, and a change's being *complete*, are defined in `check`'s Contract, which every build of this component reads among its ancestors.

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
- `owner(graph, path)`: the component that owns a repository-relative path (`owner`).

The crate builds for `wasm32-unknown-unknown` with its default features disabled (`coverage-builds-for-wasm`).

## MODIFIED Requirements

### Requirement: owner <!-- base: 50c03c134863adbf -->
`owner(graph, path)` SHALL return the component whose `tests` match `path` (`model/test-ownership-keys`, `components-own-their-tests`), or, when none's do, the component whose `paths` hold `path` (the deepest such component, then the first by ID), or none when no component's `tests` or `paths` do, matching each glob with `matches`; when two components' `tests` match, it SHALL return the first by ID, which `check/paths-disjoint` reports. `pending` SHALL decide a file's component with this `owner`, falling back to the root when it returns none. `matches(pattern, path)` SHALL match `path` against one glob as `pending` matches `paths` globs (git's `:(glob)` pathspec semantics).

#### Scenario: owned and unowned
- GIVEN components `a` with `paths: ["crates/a/**"]` and `a.b` with `paths: ["crates/b/**"]`
- WHEN `owner` runs on `crates/a/src/lib.rs`, `crates/b/x.rs` and `tools/x.sh`
- THEN it returns `a`, `a.b` and none

#### Scenario: tests inside another component's paths
- GIVEN component `harness` with `paths: ["tests/**"]` and component `a` with `tests: ["tests/a/**"]`
- WHEN `owner` runs on `tests/a/conftest.py` and `tests/helpers.py`
- THEN it returns `a` and `harness`

#### Scenario: one glob
- GIVEN the pattern `crates/*/tests/**`
- WHEN `matches` runs on `crates/a/tests/t.rs` and `crates/a/src/t.rs`
- THEN it returns true and false

#### Serves
- capability/build-from-spec
- architecture/path-ownership

#### Needs
- model/test-ownership-keys

### Requirement: state-names <!-- base: 63dc5b5dfd9eb7af -->
`check.coverage` SHALL expose, as a public constant, every requirement `State` with the word `clm coverage` and `clm pending` print for it (`current`, `behind`, `unbuilt`, `obsolete`). A test SHALL fail when `State` gains a variant the list leaves out.

#### Scenario: list matches the type
- GIVEN the crate
- WHEN a test reads the list
- THEN every variant of `State` appears once, with the word the reports print

#### Serves
- architecture/plain-terms
