---
component: check
base: { design: "76005e08ebb21713" }
---

## MODIFIED Design
`run` loads the graph (`model`), validates pending changes (`apply`), then runs the coverage, marker and text rules on the markers `check.coverage`'s `scan` finds in the files `git ls-files --cached --others --exclude-standard` lists. A marker without a version cites no version: it counts for `dangling-markers`, and the requirement it names is `behind` while it stays, so a bare marker never makes a requirement current, alone or beside a versioned one. The same scan finds removal markers: text matching the regex `clm:remove-by (\d{8}-[a-z0-9-]+)`. Text that doesn't match, such as `clm:remove-by <change-id>` in help output, isn't a marker. To mention a marker in code without claiming coverage or a removal, such as a test fixture, build the string so the literal never appears whole (e.g. `concat!("req", ":a/x")`); a fixture that holds a skipped test is built the same way.

A removal marker's change is classified from the graph (`model`), in this order: archived (it is under `changes/archive/`, whatever its status), missing (no change with that ID, including a directory without a parseable `change.md`), `done`, `abandoned`, or live (`proposed`, `approved`, `applied`, `implementing`). Violations use the marker's `<file>:<line>` as their path, as `dangling-markers` does, with messages `marker names <id>, which is archived`, `… which doesn't exist`, `… which is done` or `… which is abandoned`.

**Re-exports** (`check-coverage-split`, `nested-splits`). `check` keeps the rules `clm check` runs, the detectors and `clm size`; `check.coverage` keeps what says how far the code has caught up with the spec, which the queue, the views, `clm rebase` and the Worker read. `clm-check` depends on `clm-coverage` and re-exports its public items (`Coverage`, `Marker`, `State`, `markers_in`, `report`, `Pending`, `pending`, `Row`, `StatusReport`, `status_report`, `complete_changes`, `ChangeProgress`, `change_progress`, `removed`, `matches` (also as `path_matches`), `owner`, and with the `checkout` feature `Scan` and `scan`), and its `checkout` feature turns on `clm-coverage`'s. `pending_in`, `status`, `coverage` and `backlog`, the checkout wrappers the `clm` commands call, live in `clm-check`; `pending_in`'s unknown-node violation carries the rule `check.coverage/pending-report`, the requirement it enforces. `run` matches `paths` with `clm-coverage`'s `matches`, as `pending` does. The `dangling-markers` rule (`violations` in `coverage.rs`) lives in `clm-check` and takes the removed IDs from `clm-coverage`'s `removed`, as `removed-requirement-names` does. Tests that call `clm_check::` live in `crates/clm-check/tests`, citing the child's requirements where they prove them, since `clm-coverage` can't depend on `clm-check` (`undeclared-imports`, and a cycle); the child's own tests call `clm-coverage` directly.

**Shared paths** (`paths-disjoint`). `run` lists the files `git ls-files --cached` lists, Markdown included, itself through `git` (outside a git work tree, none); `run_listed` reads the listing its caller passes instead (`run-listed`). A file belongs to one component, the one whose builds own it and whose size it counts toward, so `nested-splits` keeps `paths` disjoint, parents and children included. A component's `tests` carve its test files out of whatever `paths` match them (`components-own-their-tests`): the file is the `tests` component's, so a suite's directory can belong to a harness component while each feature owns its own tests in it. `paths-disjoint`, `unowned-files` and `undeclared-imports` all read a file's component as its owner, the one place that rule is written. **Exemptions** (`temporary-exemptions`): `undeclared-imports`, `tests-cite-their-owner` and `skipped-tests` drop a violation in a file whose owner's `exemptions` name that rule for it, and nothing else; whether an exemption is still needed is `check.health`'s report, never a violation, so the build that fixes the code still lands.

**Undeclared imports** (`undeclared-imports`, `undeclared-imports-check`, `tree-sitter-analysis`). The import edges come from `init`'s `import_edges(root)`, the same tree-sitter extraction `clm survey` uses: Rust workspace path dependencies and `mod`/`use crate::` paths, TypeScript relative imports and workspace packages, and Python imports that resolve to files in the repository. Tree-sitter is C, which doesn't build for `wasm32-unknown-unknown`, so `run` never reports this rule; the `clm check` command (crate `clm`) does, after `run` and before the detectors, from the graph it loaded, and `--no-detectors` doesn't skip it, since it needs no tool. A file's component is its owner (`paths-disjoint`), test files and production files alike judged against its `depends_on`. Only a direct `depends_on` counts, and a parent or child gets no exemption, since `depends_on` is separate from ownership (`nested-splits`). `import_edges` returns a `Result`: outside a git work tree it returns no edges, as `scan` finds no markers there, so a fixture that isn't a repository passes; when `git` fails inside one, the command reports `<root>: check/undeclared-imports: <error>` and goes on. `clm rebase` calls `run`, so its `rebase ok` doesn't cover this rule; `clm land` runs `clm check` and does.

**Imports before apply** (`undeclared-imports-before-apply`). A change's deltas can rewrite a component's `depends_on` and `paths`, so `undeclared_imports` also judges the same edges against the graph each `proposed` or `approved` change would leave, which `apply`'s `resulting(graph, change)` gives (`apply/resulting-graph-call`), as `removed-names-before-apply` does for names. A pair both graphs leave undeclared is reported once, by the current graph's line. Each change is judged alone against the current graph, so two pending changes that drop the same dependency each report it, naming themselves. A change whose deltas don't apply has no resulting graph: `stale-deltas` reports it. The edges are the code's as it is, read once. So a draft whose delta drops a `depends_on` entry its component's code uses meets the report in its own `clm check`, before `clm apply`, not first at `clm land`. A pair is its importing file, imported file and the IDs of `a` and `b`. A pair a change makes newly judged, by adding component `a` or `b` or by changing `paths` so a file falls in one, is reported the same way. The pending changes are those `is_pending` gives, as for `run`'s scan. The lines come after the current graph's, change by change in ID order, each in the order of the edges. The judging of resulting graphs sits behind `clm-check`'s `checkout` feature, which brings in `clm-apply` for `run`'s own before-apply scan: `undeclared_imports` is public and ungated, and without the feature it judges the current graph alone. `clm rebase` and `clm gate` don't run the rule: once the change is applied the current graph's line reports it, and `clm land` runs `clm check`.

**Skipped tests** (`skipped-tests`, `skipped-tests-in-cited-files`). A marker counts once the suite passes, so a test that never runs would prove a requirement it doesn't check. CLM reads no test runner's output, so rather than judge each marker's test, `run` reports a skipped test anywhere in a file that holds at least one `req:` marker; a skipped test belongs in a file that cites nothing. A file is read by its extension. For TypeScript and JavaScript the skip patterns are `(it|test|describe)\.(skip|skipIf|todo)\b` and `\bx(it|test|describe)\(`, not preceded by a letter, digit, `_` or `.`. Markdown is never read, as for markers.

**Removed requirement names** (`removed-requirement-names`). A requirement ID is *removed* when `check.coverage`'s `removed` gives it; a change not yet applied removes nothing from `specs/` (its removals are judged on the graph it would leave, below), and a requirement added back isn't removed. `run` reads each component's `SPEC.md` below its frontmatter, skipping fenced code blocks. The path is `specs/<component path>/SPEC.md:<line>`. The rule reports only in component specs: it reads `ARCHITECTURE.md` and `CAPABILITIES.md` for their entries' slugs alone (the model loads them), and never reports in them, `CLAUDE.md`, decisions or changes, whose text names decisions and entries rather than component requirements.

**Before apply** (`removed-names-before-apply`). For each `proposed` or `approved` change, the changes `run` validates, `run` also takes the graph the change would leave from `apply`'s `resulting(graph, change)` (`apply/resulting-graph-call`) and runs the same scan on it. The removed IDs are computed from that change's deltas and the two graphs, not from `removed`, and each is attributed to that change alone: when two pending changes remove the same ID, each one's scan reports its spans, naming itself. Only IDs listed under `## REMOVED Requirements` count, as for `removed`, so a component the change removes whole adds none of its requirements. `validate` builds the same graph again; the cost is one in-memory parse of the components the change writes, so the two aren't shared. So a draft sees a name its removal leaves stale before `clm apply`, and a name the change's own deltas rewrite is never reported. The path is the component's `SPEC.md` path with the line in the text the change leaves, which is the file's own line when the change doesn't write the component. An ID the current graph doesn't hold is the applied scan's, so no span is reported by both. A change whose deltas don't apply has no resulting graph (`resulting` gives none): `stale-deltas` reports it.

**Detectors** are separate from `run`. `detectors(root, list)` takes a plain list of `Detector { name, command, hint, timeout }` and spawns each, so `clm-check` doesn't depend on `clm-config`. The spawning code sits behind the default feature `detectors`, so the crate builds for wasm with default features disabled. The `clm check` command (crate `clm`) loads the settings (`config`) and passes their `detectors` in, each with its `timeout`. It calls `detectors` after `run`, whether or not `run` found violations. If the settings can't be loaded, it reports `<file or variable>: check/detectors: <error>` as a violation, skips the detectors and reports `run`'s violations. `clm rebase` and the `web` Worker call `run`, and `clm wrap-up` calls `run_listed` on the tree it extracts; neither has a project toolchain, so they never run detectors.

Each detector runs without a shell, from the repository root, one after another in `clm.toml`'s order. Its stdout and stderr are piped and copied, line by line as they arrive, to `clm check`'s stderr, one thread per stream, so stdout keeps one line per violation and a detector that hangs has already shown what it printed (`detector-timeouts`). On Unix the detector starts in a process group of its own; when its `timeout` passes before it exits, the whole group is killed (`SIGKILL`, through `libc::killpg` or an equivalent call, not a spawned `kill`), so a `sh -c` wrapper's children stop with it. Elsewhere the process alone is killed, and a grandchild holding a pipe can keep the copying threads, and `clm check`, waiting past the timeout; CLM is released for no such platform (`no-windows-support`). `clm check` then waits for the copying threads and goes on to the next detector. A detector with no exit code whose timeout didn't pass is reported as `<name> failed (killed by a signal)`.

**Size** (`node-size`, `oversized-is-a-state`, `tests-count-where-cited`, `inline-tests-count-where-cited`). A token is estimated as four bytes, rounded up, with no tokenizer, so the count is deterministic and cheap. A component's *spec tokens* count its `SPEC.md` (`specs/SPEC.md` for the root); its assets don't count, since a context bundle lists them rather than including them. Its *code tokens* count what a build of the component reads: among the files `clm-git`'s `tracked_files` lists, each once even mid-merge, every part of a file whose `req:` markers cite one of its requirements, wherever it sits, and every part with no markers among `clm-git`'s `tracked_matching` of its `paths`, the same list `clm survey --components` reads, so the two agree on which files a component owns (`tracked-files-in-git-layer`). `size` hands those paths back to `git check-attr --stdin -z` as their bytes (`OsStrExt::as_bytes` on Unix). A file is one part, except a Rust file (`.rs`) holding a line whose text, trimmed, is exactly `#[cfg(test)]`: from the first such line to the end of the file is its *test part*, and what comes before it is its *source part*, since Rust keeps unit tests in a module at the end of the file they test. A test file, or a test part, counts toward each component it cites and not toward the component whose `paths` merely hold it, so `crates/clm/tests/run.rs` is `orchestrator.run`'s cost, not the root's, while the source part of `crates/clm-orchestrator/src/queue.rs` stays its owner's whatever its inline tests cite. Ignored files such as `node_modules/` and `target/` never count. Two kinds of tracked file are left out, being nothing a session reads: a binary file, one with a NUL byte in its first 8000 bytes as git decides, and a file whose `linguist-generated` or `linguist-vendored` attribute is set (`git check-attr`; `set` or `true`, not `false` or unset), the `.gitattributes` convention GitHub uses for lockfiles, built bundles and vendored code (`reuse-first`). The bytes of a count's parts are summed and rounded up once. Each count's budget is the component's `size_budget` for it (`model/size-budget`), else the default the caller passes in. A component is *oversized* when a count exceeds its budget; a `size_budget` that covers the count is the exception a human chose in a spec PR, and one the component has outgrown isn't. Nothing fails for it: `size` reports the state and `view` and `check.health` show it; nothing files an item for it (`oversized-shown-not-filed`). `size(root, graph, markers, defaults)` reads the working tree and sits behind the default feature `detectors`, so the crate builds for wasm. The `clm size` command (crate `clm`) loads the settings (`config`) and passes `component_size` in, as it does the detectors for `clm check`.

The counting itself is `size_of(graph, files, markers, defaults)` (`size-of-listing`), built without features so the web app's `replica`, which has a Trees API listing but no working tree, counts as `clm size` does (`replica/snapshot-size`). `files` maps each tracked file that may count to its length in bytes, with binary and linguist-excluded files already left out; each component's `SPEC.md` length is read from it, and one missing from it counts 0. `size` lists and probes the working tree as above, puts each component's `SPEC.md` length in from disk (tracked or not), and calls `size_of`, so the two can't drift. `size_of` matches `paths` with `glob::matches`, as `pending` does. The Worker can't run `git check-attr`, so `linguist_excluded(attributes, path)` (`linguist-attributes`) decides the linguist exclusion from the texts of the tree's `.gitattributes` files, each with its directory: patterns as gitattributes reads them (relative to their file's directory, a pattern without `/` matching at any depth below it, `**`, a trailing `/` matching no file), the last matching line deciding, deeper files over shallower ones, `-attr` and `!attr` unsetting, `attr=false` not setting, and `[attr]<name>` macro lines in the root `.gitattributes` expanded where a later line names the macro (git's built-in `binary` macro sets no linguist attribute, so it changes nothing here). It reads no `.git/info/attributes` or `core.attributesFile`, which a hosted listing doesn't have; `size` keeps asking `git check-attr`, which also honours those, so a clone's local attribute files can make `clm size` differ from the Worker, and only for them. An existing crate that matches gitattributes without git, such as `gix-attributes`, is preferred to writing the matcher (`reuse-first`) if it builds for wasm and keeps the Worker inside its size and CPU limits (`web-on-free-plan`); otherwise the matcher is written here.

## MODIFIED Requirements

### Requirement: undeclared-imports <!-- base: 1e2f49ba5589bb56 -->
`clm check` SHALL report each import edge `init`'s `import_edges` returns from a file of component `a` to a file of component `b`, where `b` is not `a` and `a`'s `depends_on` doesn't list `b`, as `<importing file>: check/undeclared-imports: imports <imported file> of component `b`, and `a` doesn't depend on `b``, where a file's component is its owner (`paths-disjoint`); an edge with an end no component owns SHALL NOT be reported. A file `a`'s `tests` match SHALL be judged against `a`'s `depends_on` as any of its files is. An edge from a file that an exemption of `a` names for `undeclared-imports` (`model/exemptions`) SHALL NOT be reported, judged by the exemptions of the graph being judged, the current one or the one a change would leave. `clm check --no-detectors` SHALL report it as well, and `run` SHALL NOT. It SHALL also judge the same edges against the graph each `proposed` or `approved` change would leave (`apply/resulting-graph-call`), reporting each edge and component pair that graph leaves undeclared and the current graph doesn't as `<importing file>: check/undeclared-imports: imports <imported file> of component `b`, and `a` wouldn't depend on `b` after <change>`; a change whose deltas don't apply SHALL be skipped.

#### Scenario: undeclared path dependency
- GIVEN components `a` (paths `["crates/a/**"]`, `depends_on: []`) and `b` (paths `["crates/b/**"]`), and a Cargo workspace where `crates/a/Cargo.toml` has a path dependency on `b`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``crates/a/Cargo.toml: check/undeclared-imports: imports crates/b/Cargo.toml of component `b`, and `a` doesn't depend on `b` ``

#### Scenario: declared
- GIVEN the same workspace with `a`'s `depends_on: [b]`
- WHEN `clm check` runs
- THEN it reports no `check/undeclared-imports` violation

#### Scenario: unowned file
- GIVEN the same workspace with no component whose `paths` match `crates/b/Cargo.toml`
- WHEN `clm check` runs
- THEN it reports no `check/undeclared-imports` violation

#### Scenario: a test imports the entry point
- GIVEN components `app` (paths `["app/main.py"]`, `depends_on: [feat]`) and `feat` (paths `["app/feat.py"]`, tests `["tests/feat/**"]`), where `tests/feat/test_x.py` imports `app/main.py`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``tests/feat/test_x.py: check/undeclared-imports: imports app/main.py of component `app`, and `feat` doesn't depend on `app` ``

#### Scenario: exempt until refactored
- GIVEN the same components, and `feat` with `exemptions: [{rule: undeclared-imports, files: ["tests/feat/**"], until: backlog/x}]`
- WHEN `clm check` runs
- THEN it reports no `check/undeclared-imports` violation for `tests/feat/test_x.py`

#### Scenario: production code still judged
- GIVEN the same components, where `app/feat.py` also imports `app/main.py`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``app/feat.py: check/undeclared-imports: imports app/main.py of component `app`, and `feat` doesn't depend on `app` ``

#### Scenario: not in run
- GIVEN the undeclared workspace
- WHEN `clm_check::run` is called on it
- THEN it returns no `check/undeclared-imports` violation

#### Scenario: not applied yet
- GIVEN the declared workspace and a `proposed` change `20260101-y` whose delta for `a` sets `depends_on: []` with its base
- WHEN `clm check` runs
- THEN it exits 1 with the line ``crates/a/Cargo.toml: check/undeclared-imports: imports crates/b/Cargo.toml of component `b`, and `a` wouldn't depend on `b` after 20260101-y ``

#### Scenario: not applied yet, already undeclared
- GIVEN the undeclared workspace and a `proposed` change whose delta for `a` keeps `depends_on: []` and modifies its Contract, with their bases
- WHEN `clm check` runs
- THEN it reports the edge once, with the line of `undeclared path dependency`

#### Scenario: not applied yet, stale delta
- GIVEN the change of `not applied yet`, its delta for `a` holding a stale frontmatter base
- WHEN `clm check` runs
- THEN it reports `apply/stale-base` for the delta and no `check/undeclared-imports` violation

#### Serves
- capability/catch-drift
- architecture/ancestor-scoped-context

#### Needs
- init/import-edges
- model/exemptions

### Requirement: tests-cite-their-owner <!-- base: 27661ae3d1f58526 -->
`clm check` SHALL report each `req:` marker, in a file of its scan that a component's `tests` match, that cites a requirement of a component other than that file's owner (`paths-disjoint`), as `<file>:<line>: check/tests-cite-their-owner: cites <requirement> of `b`, but the file is `a`'s test; move the test to `b`'s tests`, and `run` SHALL report it too. A marker citing a capability (`capability/<slug>`) SHALL be read as citing the `capability` component's requirement, which proves it (`human-layer-proof`). A file that only `paths` match SHALL NOT be judged, so a project gives its tests to their components one `tests` glob at a time (`components-own-their-tests`), and a marker that names no existing requirement is left to `dangling-markers`. A marker in a file that an exemption of its owner names for `tests-cite-their-owner` (`model/exemptions`) SHALL NOT be reported.

#### Scenario: another component's requirement
- GIVEN components `a` with tests `["tests/a/**"]` and `b` with requirement `y`, and `tests/a/t.rs` whose line 4 cites `b/y`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``tests/a/t.rs:4: check/tests-cite-their-owner: cites b/y of `b`, but the file is `a`'s test; move the test to `b`'s tests``

#### Scenario: its own requirement
- GIVEN the same components, and `tests/a/t.rs` citing only `a`'s requirements
- WHEN `clm check` runs
- THEN it reports no `check/tests-cite-their-owner` violation

#### Scenario: a file only paths own
- GIVEN component `a` with paths `["tests/**"]` and no `tests`, and `tests/t.rs` citing `b/y`
- WHEN `clm check` runs
- THEN it reports no `check/tests-cite-their-owner` violation

#### Scenario: exempt until split
- GIVEN the file of `another component's requirement`, and `a` with an exemption from `tests-cite-their-owner` for `tests/a/t.rs` until `backlog/x`
- WHEN `clm check` runs
- THEN it reports no `check/tests-cite-their-owner` violation

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

#### Needs
- model/test-ownership-keys
- model/exemptions

### Requirement: skipped-tests <!-- base: f3cf033294c07d0a -->
`clm check` SHALL report, in each scanned file that holds a `req:` marker, each line that skips a test: in a `.rs` file a line starting `#[ignore` after leading whitespace; in a `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` or `.cjs` file a call of `it`, `test` or `describe` with `.skip`, `.skipIf` or `.todo`, or of `xit`, `xtest` or `xdescribe`; and in a `.py` file a line starting `@pytest.mark.skip` or `@unittest.skip` after leading whitespace, or holding `pytest.skip(`; as `<file>:<line>: check/skipped-tests: a skipped test in a file that cites requirements; run it, or move it to a file that cites none`. A file without markers, a Markdown file, and a file that an exemption of its owner names for `skipped-tests` (`model/exemptions`) SHALL NOT be reported.

#### Scenario: ignored Rust test
- GIVEN `tests/t.rs` holding a marker citing `a/x` on line 2 and `#[ignore]` on line 5
- WHEN `clm check` runs
- THEN it exits 1 reporting `tests/t.rs:5` with `check/skipped-tests`

#### Scenario: skipped TypeScript test
- GIVEN `web/t.test.ts` holding a marker citing `a/x` and `it.skip("x", () => {})` on line 7
- WHEN `clm check` runs
- THEN it reports `web/t.test.ts:7` with `check/skipped-tests`

#### Scenario: skipped Python test
- GIVEN `tests/test_a.py` holding a marker citing `a/x` and `@pytest.mark.skipif(True, reason="x")` on line 4
- WHEN `clm check` runs
- THEN it reports `tests/test_a.py:4` with `check/skipped-tests`

#### Scenario: no markers
- GIVEN `tests/slow.rs` holding `#[ignore]` and no marker
- WHEN `clm check` runs
- THEN it reports no `check/skipped-tests` violation

#### Scenario: exempt skip
- GIVEN `tests/t.rs` holding a marker citing `a/x` and `#[ignore]` on line 5, owned by `a`, which has an exemption from `skipped-tests` for `tests/t.rs` until `backlog/x`
- WHEN `clm check` runs
- THEN it reports no `check/skipped-tests` violation

#### Serves
- capability/prove-by-tests
- architecture/proven-by-tests

#### Needs
- model/exemptions
