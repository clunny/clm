---
component: check.coverage
base: { contract: "45da0db9f35a639b" }
---

## MODIFIED Contract
How far the code has caught up with the spec, for `check` and for every component that reads it (`check-coverage-split`). A component whose code calls this library names `check.coverage` in its `depends_on` and imports `clm-coverage`, so its build is given this Contract. `clm-check` re-exports none of it. The states it computes, *current*, *behind*, *unbuilt* and *obsolete*, and a change's being *complete*, are defined in `check`'s Contract, which every build of this component reads among its ancestors.

Crate `clm-coverage`. Its library:
- `Marker { id, version, path, line }`: a marker at a file's line (1-based), `version` none for a bare marker; it displays as `<file>:<line>`. `State`, one of `Current`, `Behind`, `Unbuilt` and `Obsolete`, and `STATE_NAMES`, each `State` with the word the reports print for it (`state-names`).
- `tests_in(path, text)`: the tests and named groups of tests in one file, each a `TestSpan { name, group, first, last }`, and `markers_outside_tests(path, text)`: the `req:` markers on none of those lines (`test-spans`). `marker_spans(text)`: every `req:` marker in a text, inside a test or not, each a `MarkerSpan { id, version, range }` with its byte range (`marker-spans`). `holds_tests(path)`: whether a path's extension is one `tests_in` reads, so a file for which it is false holds no test whatever its text (`test-holding-paths`), which `land` reads to skip the files that can hold none.
- `markers_in(path, text)`: the `req:` markers in one file that count, those inside a test (`markers-in-tests`), none for a file whose extension is `md`; `scan(root)`, with the default feature `checkout`: the markers of every file a checkout lists, or of the files under `root` where `git ls-files` fails (`markers-from-files`), as `Scan { req, outside, remove_by }`: `req`, the `req:` markers that count (`markers-in-tests`); `outside`, the `req:` markers on no line of a test or group (`test-spans`); and `remove_by`, the `clm:remove-by` markers, each `id` a change's. `read_files(root)`, with the same feature: the files `scan` reads, each as its path relative to `root` and its text, sorted by path (`markers-from-files`), in the form `obsolete_tests`, `deleted_tests` and `uncited_tests` take.
- `report(graph, markers)`: each requirement's `Coverage { id, state, version, markers, proof }`, human-layer entries included, every architecture entry, and a capability holding `#### Untested`, carrying its `Proof` (`Enforced`, `Pending`, `Untested` or `Unenforced`) in place of markers; each displays as its `clm coverage` line (`coverage-report`).
- `pending(graph, markers)`: each component's pending requirements, each a `Pending { id, node, version, state, cited, markers, priority, blocked_by, waits_on }`, `node` the component its line is pending on, displaying as its `clm pending` line (`pending-report`), and `waits(lines, line)`: the lines of that list one of its lines waits on (`wait-lines`). A reader of `waits_on` takes the component of a wait from `waits`, never from the ID's own first part: an obsolete requirement is pending on the owners of the files that cite it, and its line on its own component waits on its own ID.
- `change_progress(graph, markers)`: each change's `ChangeProgress { change, current, total, obsolete, complete }` (`change-progress`).
- `status_report(graph, markers)`: `clm status`'s `components` and `changes` sections as `StatusReport { nodes, changes }`, each row a `Row` of cells, and `complete_changes(graph, markers)`: the IDs of the changes that are complete, both built on `change_progress` (`status-report`). `moved_to(graph, id)`: the requirements the moves take a removed requirement or capability to (`move-targets`).
- `removed(graph)`: each requirement ID that a change outside the archive lists under `REMOVED Requirements` and no component holds, and each `capability/<slug>` that a change outside the archive lists under `removes_capabilities` and `CAPABILITIES.md` doesn't hold (`removed-capabilities`), with those changes' IDs sorted; what makes a marker *obsolete*, and what `check`'s `dangling-markers` and `removed-requirement-names` exempt or report.
- `matches(glob, path)`: whether a `paths` or `tests` glob matches a repository-relative path, as `owner`, `pending` and every other reader of those keys match.
- `uncounted(graph, outside)`, each an `Uncounted { id, markers }`, `obsolete_tests(graph, files)`, each an `ObsoleteTest { test, line, ids, every }`, and `deleted_tests(base_files, head_files)`, each a `DeletedTest { test, line, ids }`: the markers outside tests by what they cite, the tests a change's removals leave with nothing to cite, and the citing tests one tree holds that another doesn't (`test-lists`); `clm coverage` prints them through `clm-check` (`check/coverage-lists`). `bare_name(name)`: a test's name with its markers left out, as `deleted_tests` compares names (`test-lists`).
- `uncited_tests(files)`, each an `UncitedTest { test, line }`: the tests that hold no marker, for `clm health` (`check.health/rule-signals`). `deleted_tests` counts a test as gone when the files hold fewer marker-holding tests of its name, so a test moved to another file is not, nor is one renamed in its own file that still cites every requirement it cited; `clm land` holds a code branch to it (`land/deleted-tests-approved`).
- `owner(graph, path)`: the component that owns a repository-relative path (`owner`).

The crate builds for `wasm32-unknown-unknown` with its default features disabled (`coverage-builds-for-wasm`).

## ADDED Requirements

### Requirement: test-holding-paths
`clm-coverage` SHALL offer `holds_tests(path)`, true exactly when the path's extension, compared as written, is one `test-spans` finds tests by (`rs`; `ts`, `tsx`, `mts`, `cts`, `js`, `jsx`, `mjs` and `cjs`; and `py`), so that `tests_in` and `markers_outside_tests` return nothing, whatever the text, for every path for which it is false. It SHALL run no process and SHALL build without the `checkout` feature.

#### Scenario: which paths can hold a test
- GIVEN the paths `a.rs`, `a.test.ts`, `x.mjs`, `e.spec.tsx`, `test_a.py`, `a_test.go`, `README.md`, `Cargo.lock`, `rs` and `lib.RS`
- WHEN `holds_tests` runs on each
- THEN it is true for the first five and false for the rest

#### Serves
- capability/land-safely
