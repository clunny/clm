---
component: check
base: { contract: "18d17e57e70517d9" }
---

## MODIFIED Contract

`clm check` runs every deterministic rule and exits non-zero on any violation, printing one actionable line per violation. Rules include schema, graph integrity, `paths` that two components share (`paths-disjoint`), imports between components that `depends_on` doesn't declare (`undeclared-imports`), code files in no component's `paths` (`unowned-files`), requirement test coverage, skipped tests in files that cite requirements (`skipped-tests`), names of requirements a live change removed (`removed-requirement-names`), references to component assets, stale deltas, and `clm:remove-by` markers whose change isn't live (`expand-contract`). When either human layer holds an entry, every rule must serve one (`check/untraced`, `traced-rules`). Path ownership (`unowned-files`), in a repository whose root sets `outside` (even to `[]`), means every tracked code file is in some component's `paths` or matched by the root's `outside`, the code not yet adopted, and every `outside` glob matches code no component owns (`incremental-adoption`). A root without the key isn't checked for it, so a repository opts in by setting it; `clm init` always does. The `clm check` command also runs the dead-code and unused-dependency detectors the repository's `clm.toml` names (`no-dead-code`, `dead-code-detectors`), each for at most its `timeout` (`detector-timeouts`), unless it is given `--no-detectors`. A caller that knows no code changed, such as `clm land` on code already checked, passes it to skip them. A caller that checks a commit's files extracted outside a git work tree, such as `clm wrap-up`, passes the commit's file listing to `run_listed`, so the rules that read the tracked files see what `clm check` sees in a checkout of that commit (`run-listed`).

A test cites the requirement it proves as `req:<id>@<version>`, where the version is the first 8 hex digits of the requirement's content hash (`model/content-hash`, `versioned-markers`). The marker says which text the test was written for. A marker counts where a test can run it: in a Rust, TypeScript, JavaScript or Python file, inside a test or a named group of tests or in the comment directly above one, and anywhere in a file of another language (`check.coverage/markers-in-tests`). A requirement is *current* when markers that count cite it, every one cites its current version and no `bug` item names it under `unproven` at that version (`mutation-after-land`); *behind* when a marker cites it at another version or without one, even beside one that cites the current version (`stale-markers-keep-behind`); *unbuilt* when no marker that counts cites it, or such an item names it; and *obsolete* when a marker cites a requirement that a change outside the archive removed (`removed-requirements-build-out`), or a capability that such a change lists under `removes_capabilities` and `CAPABILITIES.md` no longer holds (`removed-capabilities-build-out`). The last three are *pending*: the component's code hasn't caught up with its spec. A capability is cited like a rule. An architecture entry is never cited: it is current when every rule its `#### Enforced by` names is current, and an entry or capability holding `#### Untested` is never pending, so neither is in the queue (`human-layer-proof`, `check.coverage/coverage-report`). `clm pending [<component>]` lists them (`check.coverage/pending-report`), and the queue (`orchestrator`) is built on it. A change is *complete* as `check.coverage/status-report` defines it.

The markers and the states are computed by the child component `check.coverage` (crate `clm-coverage`), and `clm-check` re-exports its library, so a component that depends on `check` keeps calling it through `clm-check` (`check-coverage-split`): `markers_in(path, text)` returns the markers `clm coverage` would find in one file (none for Markdown), with their versions; `report(graph, markers)` turns the markers of every file into the coverage `clm coverage` prints; `pending(graph, markers)` derives the pending list from them; `change_progress(graph, markers)` gives each live change's progress, the counts `clm status` prints for it (`check.coverage/change-progress`); `status_report(graph, markers)` and `complete_changes`, built on it, give `clm status`'s sections and the complete changes; and `scan(root)` reads a checkout's markers. The crate builds for `wasm32-unknown-unknown` with its default features disabled (`check-builds-for-wasm`). Which component a file belongs to is available as library calls too, for `check.affected`: `owner`, `path_matches` and `scan` (`path-ownership`).

`clm size [<component>]` reports each component's size in tokens against its budget, marking the components over either (`node-size`, `node-size-budgets`). It is advisory: it exits 0 whatever the sizes (1 only for an unknown component or settings that can't be loaded), and no rule of `clm check` reads them.

## MODIFIED Requirements

### Requirement: dangling-markers <!-- base: 2e31c5b8395a0bb1 -->
`clm check` SHALL report every `req:<id>` marker in a scanned file whose `<id>` names no existing requirement or human-layer entry (`model/human-layers`), giving the file and line; and every marker that cites an architecture entry, or an entry holding `#### Untested`, with the message `architecture entries are proven by the rules their #### Enforced by names, not by tests` or `<id> is untested`, since a test never proves either (`human-layer-proof`); unless a change outside the archive lists `<id>` under `REMOVED Requirements` in one of its deltas, or `<id>` is a capability that a change outside the archive lists under `removes_capabilities` and `CAPABILITIES.md` doesn't hold (`check.coverage/removed-capabilities`): that code is obsolete and is removed when the component is next built (`removed-requirements-build-out`, `removed-capabilities-build-out`). The message for a marker whose `<id>` is `capability/<slug>` and names no entry, with no such change, SHALL end `; a change that removes the capability lists it under removes_capabilities`, so the author of a branch that removes a cited capability learns how to land it. Once the change is archived, the marker SHALL be reported as before. An `abandoned` change is archived (`archive-done`), so it grants no exemption; a requirement another change adds back under the same ID is a new requirement once that change applies, and the marker then cites it as usual. It SHALL judge the markers outside a test, `scan`'s `outside` (`check.coverage/test-spans`), as it judges `scan`'s `req`, each marker once by its file, line and ID, so a marker that proves nothing still can't name what doesn't exist.

#### Scenario: marker on an entry
- GIVEN a test citing `capability/approve`, an entry of `CAPABILITIES.md`, and another citing `capability/nope`, which isn't one
- WHEN `clm check` runs
- THEN it reports the second marker's file and line, and not the first

#### Scenario: marker on an architecture or untested entry
- GIVEN a test citing `architecture/git-is-truth`, an entry of `ARCHITECTURE.md`, and another citing `capability/watch`, a capability holding `#### Untested`
- WHEN `clm check` runs
- THEN it reports both markers' files and lines, the first saying architecture entries are proven by their `#### Enforced by` rules and the second that `capability/watch` is untested

#### Scenario: test cites a removed requirement
- GIVEN `tests/t.rs` line 3 contains the marker for `a/gone`, and `a` has no requirement `gone`
- WHEN `clm check` runs
- THEN it exits 1 reporting `tests/t.rs:3` and `a/gone`

#### Scenario: removed by a live change
- GIVEN `tests/t.rs` line 3 contains the marker for `a/gone`, `a` has no requirement `gone`, and an `applied` change `x` whose delta for `a` lists `gone` under `REMOVED Requirements`
- WHEN `clm check` runs
- THEN it reports no `check/dangling-markers` violation

#### Scenario: capability removed by a live change
- GIVEN `tests/t.rs` line 3 contains the marker for `capability/gone`, `CAPABILITIES.md` has no entry `gone`, and a `proposed` change `x` with `removes_capabilities: [gone]`
- WHEN `clm check` runs
- THEN it reports no `check/dangling-markers` violation; with `x` under `changes/archive/`, or with no `removes_capabilities`, it exits 1 reporting `tests/t.rs:3` and `capability/gone`, the line ending `; a change that removes the capability lists it under removes_capabilities`

#### Scenario: removing change archived
- GIVEN the same files, with change `x` under `changes/archive/`
- WHEN `clm check` runs
- THEN it exits 1 reporting `tests/t.rs:3` and `a/gone`

#### Scenario: a dangling marker outside a test
- GIVEN `src/lib.rs` holding `req:a/nope` in a function with no test attribute, and no requirement `a/nope`
- WHEN `clm check` runs
- THEN it reports the marker once as `check/dangling-markers`, naming the file and line

#### Needs
- check.coverage/test-spans
- model/human-layers
- check.coverage/removed-capabilities

## ADDED Requirements

### Requirement: coverage-lists
`clm-check` SHALL re-export `clm-coverage`'s `tests_in`, `markers_outside_tests`, `uncounted`, `obsolete_tests` and `deleted_tests` (`check-coverage-split`). After the obsolete lines `clm coverage` SHALL print one line `<id> uncounted <file>:<line>[ <file>:<line>...]` for each entry `uncounted` returns for the checkout, so a build finds the markers to move. `clm coverage --obsolete-tests` SHALL print, in place of the coverage, the line `# tests to delete: <n>; tests that only lose a marker: <m>` and then one line per test `obsolete_tests` returns for the checkout, in its order: `DELETE <file>::<name>`, a tab and `cites only removed: <id>[, <id>...]` when every marker on the test's lines cites a removed requirement, and otherwise `UNCITE <file>::<name>`, a tab and `loses: <id>[, <id>...]`. `clm coverage --deleted-tests <base>` SHALL print `# tests deleted since <base>: <n>` and then one line `DELETED <file>::<name>`, a tab and the IDs it cited, per test `deleted_tests` returns for the files of the revision `<base>` against the working tree's. Each SHALL exit 0; `--deleted-tests` SHALL exit 1 naming the revision when `<base>` resolves to no commit; the two flags together SHALL exit 2 as a usage error; and a graph that doesn't load SHALL fail as `clm coverage` fails.

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
