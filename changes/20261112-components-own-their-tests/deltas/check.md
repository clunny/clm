---
component: check
base: { design: "da0a929aa254fa9c" }
---

## MODIFIED Design
`run` loads the graph (`model`), validates pending changes (`apply`), then runs the coverage, marker and text rules on the markers `check.coverage`'s `scan` finds in the files `git ls-files --cached --others --exclude-standard` lists. A marker without a version cites no version: it counts for `dangling-markers`, and the requirement it names is `behind` while it stays, so a bare marker never makes a requirement current, alone or beside a versioned one. The same scan finds removal markers: text matching the regex `clm:remove-by (\d{8}-[a-z0-9-]+)`. Text that doesn't match, such as `clm:remove-by <change-id>` in help output, isn't a marker. To mention a marker in code without claiming coverage or a removal, such as a test fixture, build the string so the literal never appears whole (e.g. `concat!("req", ":a/x")`); a fixture that holds a skipped test is built the same way.

A removal marker's change is classified from the graph (`model`), in this order: archived (it is under `changes/archive/`, whatever its status), missing (no change with that ID, including a directory without a parseable `change.md`), `done`, `abandoned`, or live (`proposed`, `approved`, `applied`, `implementing`). Violations use the marker's `<file>:<line>` as their path, as `dangling-markers` does, with messages `marker names <id>, which is archived`, `… which doesn't exist`, `… which is done` or `… which is abandoned`.

**Re-exports** (`check-coverage-split`, `nested-splits`). `check` keeps the rules `clm check` runs, the detectors and `clm size`; `check.coverage` keeps what says how far the code has caught up with the spec, which the queue, the views, `clm rebase` and the Worker read. `clm-check` depends on `clm-coverage` and re-exports its public items (`Coverage`, `Marker`, `State`, `markers_in`, `report`, `Pending`, `pending`, `Row`, `StatusReport`, `status_report`, `complete_changes`, `ChangeProgress`, `change_progress`, `removed`, `matches` (also as `path_matches`), `owner`, and with the `checkout` feature `Scan` and `scan`), and its `checkout` feature turns on `clm-coverage`'s. `pending_in`, `status`, `coverage` and `backlog`, the checkout wrappers the `clm` commands call, live in `clm-check`; `pending_in`'s unknown-node violation carries the rule `check.coverage/pending-report`, the requirement it enforces. `run` matches `paths` with `clm-coverage`'s `matches`, as `pending` does. The `dangling-markers` rule (`violations` in `coverage.rs`) lives in `clm-check` and takes the removed IDs from `clm-coverage`'s `removed`, as `removed-requirement-names` does. Tests that call `clm_check::` live in `crates/clm-check/tests`, citing the child's requirements where they prove them, since `clm-coverage` can't depend on `clm-check` (`undeclared-imports`, and a cycle); the child's own tests call `clm-coverage` directly.

**Shared paths** (`paths-disjoint`). `run` lists the files `git ls-files --cached` lists, Markdown included, itself through `git` (outside a git work tree, none); `run_listed` reads the listing its caller passes instead (`run-listed`). A file belongs to one component, the one whose builds own it and whose size it counts toward, so `nested-splits` keeps `paths` disjoint, parents and children included. A component's `tests` carve its test files out of whatever `paths` match them (`components-own-their-tests`): the file is the `tests` component's, so a suite's directory can belong to a harness component while each feature owns its own tests in it. `paths-disjoint`, `unowned-files` and `undeclared-imports` all read a file's component as its owner, the one place that rule is written.

**Undeclared imports** (`undeclared-imports`, `undeclared-imports-check`, `tree-sitter-analysis`). The import edges come from `init`'s `import_edges(root)`, the same tree-sitter extraction `clm survey` uses: Rust workspace path dependencies and `mod`/`use crate::` paths, TypeScript relative imports and workspace packages, and Python imports that resolve to files in the repository. Tree-sitter is C, which doesn't build for `wasm32-unknown-unknown`, so `run` never reports this rule; the `clm check` command (crate `clm`) does, after `run` and before the detectors, from the graph it loaded, and `--no-detectors` doesn't skip it, since it needs no tool. A file's component is its owner (`paths-disjoint`), and a file a component's `tests` match is judged against that component's `depends_on` and `test_depends_on` together, production files against `depends_on` alone. Only a direct `depends_on` counts, and a parent or child gets no exemption, since `depends_on` is separate from ownership (`nested-splits`). `import_edges` returns a `Result`: outside a git work tree it returns no edges, as `scan` finds no markers there, so a fixture that isn't a repository passes; when `git` fails inside one, the command reports `<root>: check/undeclared-imports: <error>` and goes on. `clm rebase` calls `run`, so its `rebase ok` doesn't cover this rule; `clm land` runs `clm check` and does.

**Imports before apply** (`undeclared-imports-before-apply`). A change's deltas can rewrite a component's `depends_on` and `paths`, so `undeclared_imports` also judges the same edges against the graph each `proposed` or `approved` change would leave, which `apply`'s `resulting(graph, change)` gives (`apply/resulting-graph-call`), as `removed-names-before-apply` does for names. A pair both graphs leave undeclared is reported once, by the current graph's line. Each change is judged alone against the current graph, so two pending changes that drop the same dependency each report it, naming themselves. A change whose deltas don't apply has no resulting graph: `stale-deltas` reports it. The edges are the code's as it is, read once. So a draft whose delta drops a `depends_on` entry its component's code uses meets the report in its own `clm check`, before `clm apply`, not first at `clm land`. A pair is its importing file, imported file and the IDs of `a` and `b`. A pair a change makes newly judged, by adding component `a` or `b` or by changing `paths` so a file falls in one, is reported the same way. The pending changes are those `is_pending` gives, as for `run`'s scan. The lines come after the current graph's, change by change in ID order, each in the order of the edges. The judging of resulting graphs sits behind `clm-check`'s `checkout` feature, which brings in `clm-apply` for `run`'s own before-apply scan: `undeclared_imports` is public and ungated, and without the feature it judges the current graph alone. `clm rebase` and `clm gate` don't run the rule: once the change is applied the current graph's line reports it, and `clm land` runs `clm check`.

**Skipped tests** (`skipped-tests`, `skipped-tests-in-cited-files`). A marker counts once the suite passes, so a test that never runs would prove a requirement it doesn't check. CLM reads no test runner's output, so rather than judge each marker's test, `run` reports a skipped test anywhere in a file that holds at least one `req:` marker; a skipped test belongs in a file that cites nothing. A file is read by its extension. For TypeScript and JavaScript the skip patterns are `(it|test|describe)\.(skip|skipIf|todo)\b` and `\bx(it|test|describe)\(`, not preceded by a letter, digit, `_` or `.`. Markdown is never read, as for markers.

**Removed requirement names** (`removed-requirement-names`). A requirement ID is *removed* when `check.coverage`'s `removed` gives it; a change not yet applied removes nothing from `specs/` (its removals are judged on the graph it would leave, below), and a requirement added back isn't removed. `run` reads each component's `SPEC.md` below its frontmatter, skipping fenced code blocks. The path is `specs/<component path>/SPEC.md:<line>`. The rule reports only in component specs: it reads `ARCHITECTURE.md` and `CAPABILITIES.md` for their entries' slugs alone (the model loads them), and never reports in them, `CLAUDE.md`, decisions or changes, whose text names decisions and entries rather than component requirements.

**Before apply** (`removed-names-before-apply`). For each `proposed` or `approved` change, the changes `run` validates, `run` also takes the graph the change would leave from `apply`'s `resulting(graph, change)` (`apply/resulting-graph-call`) and runs the same scan on it. The removed IDs are computed from that change's deltas and the two graphs, not from `removed`, and each is attributed to that change alone: when two pending changes remove the same ID, each one's scan reports its spans, naming itself. Only IDs listed under `## REMOVED Requirements` count, as for `removed`, so a component the change removes whole adds none of its requirements. `validate` builds the same graph again; the cost is one in-memory parse of the components the change writes, so the two aren't shared. So a draft sees a name its removal leaves stale before `clm apply`, and a name the change's own deltas rewrite is never reported. The path is the component's `SPEC.md` path with the line in the text the change leaves, which is the file's own line when the change doesn't write the component. An ID the current graph doesn't hold is the applied scan's, so no span is reported by both. A change whose deltas don't apply has no resulting graph (`resulting` gives none): `stale-deltas` reports it.

**Detectors** are separate from `run`. `detectors(root, list)` takes a plain list of `Detector { name, command, hint, timeout }` and spawns each, so `clm-check` doesn't depend on `clm-config`. The spawning code sits behind the default feature `detectors`, so the crate builds for wasm with default features disabled. The `clm check` command (crate `clm`) loads the settings (`config`) and passes their `detectors` in, each with its `timeout`. It calls `detectors` after `run`, whether or not `run` found violations. If the settings can't be loaded, it reports `<file or variable>: check/detectors: <error>` as a violation, skips the detectors and reports `run`'s violations. `clm rebase` and the `web` Worker call `run`, and `clm wrap-up` calls `run_listed` on the tree it extracts; neither has a project toolchain, so they never run detectors.

Each detector runs without a shell, from the repository root, one after another in `clm.toml`'s order. Its stdout and stderr are piped and copied, line by line as they arrive, to `clm check`'s stderr, one thread per stream, so stdout keeps one line per violation and a detector that hangs has already shown what it printed (`detector-timeouts`). On Unix the detector starts in a process group of its own; when its `timeout` passes before it exits, the whole group is killed (`SIGKILL`, through `libc::killpg` or an equivalent call, not a spawned `kill`), so a `sh -c` wrapper's children stop with it. Elsewhere the process alone is killed, and a grandchild holding a pipe can keep the copying threads, and `clm check`, waiting past the timeout; CLM is released for no such platform (`no-windows-support`). `clm check` then waits for the copying threads and goes on to the next detector. A detector with no exit code whose timeout didn't pass is reported as `<name> failed (killed by a signal)`.

**Size** (`node-size`, `oversized-is-a-state`, `tests-count-where-cited`, `inline-tests-count-where-cited`). A token is estimated as four bytes, rounded up, with no tokenizer, so the count is deterministic and cheap. A component's *spec tokens* count its `SPEC.md` (`specs/SPEC.md` for the root); its assets don't count, since a context bundle lists them rather than including them. Its *code tokens* count what a build of the component reads: among the files `clm-git`'s `tracked_files` lists, each once even mid-merge, every part of a file whose `req:` markers cite one of its requirements, wherever it sits, and every part with no markers among `clm-git`'s `tracked_matching` of its `paths`, the same list `clm survey --components` reads, so the two agree on which files a component owns (`tracked-files-in-git-layer`). `size` hands those paths back to `git check-attr --stdin -z` as their bytes (`OsStrExt::as_bytes` on Unix). A file is one part, except a Rust file (`.rs`) holding a line whose text, trimmed, is exactly `#[cfg(test)]`: from the first such line to the end of the file is its *test part*, and what comes before it is its *source part*, since Rust keeps unit tests in a module at the end of the file they test. A test file, or a test part, counts toward each component it cites and not toward the component whose `paths` merely hold it, so `crates/clm/tests/run.rs` is `orchestrator.run`'s cost, not the root's, while the source part of `crates/clm-orchestrator/src/queue.rs` stays its owner's whatever its inline tests cite. Ignored files such as `node_modules/` and `target/` never count. Two kinds of tracked file are left out, being nothing a session reads: a binary file, one with a NUL byte in its first 8000 bytes as git decides, and a file whose `linguist-generated` or `linguist-vendored` attribute is set (`git check-attr`; `set` or `true`, not `false` or unset), the `.gitattributes` convention GitHub uses for lockfiles, built bundles and vendored code (`reuse-first`). The bytes of a count's parts are summed and rounded up once. Each count's budget is the component's `size_budget` for it (`model/size-budget`), else the default the caller passes in. A component is *oversized* when a count exceeds its budget; a `size_budget` that covers the count is the exception a human chose in a spec PR, and one the component has outgrown isn't. Nothing fails for it: `size` reports the state and `view` and `check.health` show it; nothing files an item for it (`oversized-shown-not-filed`). `size(root, graph, markers, defaults)` reads the working tree and sits behind the default feature `detectors`, so the crate builds for wasm. The `clm size` command (crate `clm`) loads the settings (`config`) and passes `component_size` in, as it does the detectors for `clm check`.

The counting itself is `size_of(graph, files, markers, defaults)` (`size-of-listing`), built without features so the web app's `replica`, which has a Trees API listing but no working tree, counts as `clm size` does (`replica/snapshot-size`). `files` maps each tracked file that may count to its length in bytes, with binary and linguist-excluded files already left out; each component's `SPEC.md` length is read from it, and one missing from it counts 0. `size` lists and probes the working tree as above, puts each component's `SPEC.md` length in from disk (tracked or not), and calls `size_of`, so the two can't drift. `size_of` matches `paths` with `glob::matches`, as `pending` does. The Worker can't run `git check-attr`, so `linguist_excluded(attributes, path)` (`linguist-attributes`) decides the linguist exclusion from the texts of the tree's `.gitattributes` files, each with its directory: patterns as gitattributes reads them (relative to their file's directory, a pattern without `/` matching at any depth below it, `**`, a trailing `/` matching no file), the last matching line deciding, deeper files over shallower ones, `-attr` and `!attr` unsetting, `attr=false` not setting, and `[attr]<name>` macro lines in the root `.gitattributes` expanded where a later line names the macro (git's built-in `binary` macro sets no linguist attribute, so it changes nothing here). It reads no `.git/info/attributes` or `core.attributesFile`, which a hosted listing doesn't have; `size` keeps asking `git check-attr`, which also honours those, so a clone's local attribute files can make `clm size` differ from the Worker, and only for them. An existing crate that matches gitattributes without git, such as `gix-attributes`, is preferred to writing the matcher (`reuse-first`) if it builds for wasm and keeps the Worker inside its size and CPU limits (`web-on-free-plan`); otherwise the matcher is written here.

## MODIFIED Requirements

### Requirement: paths-disjoint <!-- base: fad0d1e8c3adc494 -->
A file's *owner* SHALL be the component whose `tests` match it or, when no component's `tests` do, the component whose `paths` match it (`components-own-their-tests`). `clm check` SHALL report each file of its scan that the `tests` of two or more components match, or that no component's `tests` match and the `paths` of two or more components match, as `<file>: check/paths-disjoint: owned by <components>`, the components in backticks, sorted by ID and comma-separated, once per file, and `run` SHALL report it too. A file that one component's `tests` match and another's `paths` match is the first component's, and SHALL NOT be reported.

#### Scenario: two owners
- GIVEN component `a` with paths `["src/**"]`, component `b` with paths `["src/b/**"]`, and a tracked `src/b/x.rs`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``src/b/x.rs: check/paths-disjoint: owned by `a`, `b` ``

#### Scenario: parent and child
- GIVEN component `a` with paths `["src/**"]` and its child `a.c` with paths `["src/c/**"]`, and a tracked `src/c/y.rs`
- WHEN `clm check` runs
- THEN it reports `src/c/y.rs` with `check/paths-disjoint`, naming `a` and `a.c`

#### Scenario: disjoint
- GIVEN component `a` with paths `["src/a/**"]`, component `b` with paths `["src/b/**"]`, and tracked files under both
- WHEN `clm check` runs
- THEN it reports no `check/paths-disjoint` violation

#### Scenario: tests inside another component's paths
- GIVEN component `harness` with paths `["tests/**"]`, component `a` with tests `["tests/a/**"]`, and a tracked `tests/a/t.py`
- WHEN `clm check` runs
- THEN it reports no `check/paths-disjoint` violation, and `tests/a/t.py`'s owner is `a`

#### Scenario: two components' tests
- GIVEN components `a` and `b` with tests `["tests/**"]` and `["tests/b/**"]`, and a tracked `tests/b/t.py`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``tests/b/t.py: check/paths-disjoint: owned by `a`, `b` ``

#### Serves
- architecture/path-ownership
- capability/work-concurrently

#### Needs
- model/test-ownership-keys

### Requirement: undeclared-imports <!-- base: f84e6527bc4a9603 -->
`clm check` SHALL report each import edge `init`'s `import_edges` returns from a file of component `a` to a file of component `b`, where `b` is not `a` and `a`'s `depends_on` doesn't list `b`, as `<importing file>: check/undeclared-imports: imports <imported file> of component `b`, and `a` doesn't depend on `b``, where a file's component is its owner (`paths-disjoint`); an edge with an end no component owns SHALL NOT be reported. An edge from a file that `a`'s `tests` match SHALL be judged against `a`'s `depends_on` and `test_depends_on` together (`model/test-ownership-keys`), and reported only when neither lists `b`, with the same line. `clm check --no-detectors` SHALL report it as well, and `run` SHALL NOT. It SHALL also judge the same edges against the graph each `proposed` or `approved` change would leave (`apply/resulting-graph-call`), reporting each edge and component pair that graph leaves undeclared and the current graph doesn't as `<importing file>: check/undeclared-imports: imports <imported file> of component `b`, and `a` wouldn't depend on `b` after <change>`; a change whose deltas don't apply SHALL be skipped.

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
- GIVEN components `app` (paths `["app/main.py"]`, `depends_on: [feat]`) and `feat` (paths `["app/feat.py"]`, tests `["tests/feat/**"]`, `test_depends_on: [app]`), where `tests/feat/test_x.py` imports `app/main.py`
- WHEN `clm check` runs
- THEN it reports no `check/undeclared-imports` violation

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
- model/test-ownership-keys

### Requirement: unowned-files <!-- base: d0c0eb0da6e9b9cc -->
When the root sets `outside` (`model/outside-globs`), even to `[]`, `run` SHALL check path ownership; when the root has no `outside` key it SHALL NOT, and reports no `check/unowned-files` violation. It SHALL read the files the index holds under the CLM repository (`git ls-files --cached`, through the `git` layer), every entry the index lists, symlinks and submodules included; in a directory that isn't a git work tree it SHALL read no file and report no `check/unowned-files` violation; `run_listed` (`run-listed`) SHALL read the files of the listing it is given instead, wherever the directory is. It SHALL leave out the spec paths: every file under `specs/`, `changes/`, `backlog/` or `docs/`, and `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `AGENTS.md`, the same spec paths `gate` names. Of the rest, the *code files*, it SHALL report each that no component owns (`paths-disjoint`), by its `paths` or its `tests`, whatever the component's status, and no glob of the root's `outside` matches, as `<file>: check/unowned-files: in no component's paths; add it to a component's paths, or to outside in specs/SPEC.md until a change adopts it`. It SHALL report each `outside` glob that matches no code file that no component owns, as `specs/SPEC.md: check/unowned-files: outside glob <glob> matches no file outside every component`. A glob matches a file as the git pathspec `:(glob)<glob>` does; matching in memory by the same rules is enough. A code file that a component owns and an `outside` glob matches is the component's, and SHALL NOT be reported. A file the index doesn't hold, untracked or ignored, SHALL NOT be read or reported.

#### Scenario: file in no node
- GIVEN component `a` with `paths: ["crates/a/**"]`, a root with `outside: []`, and tracked files `crates/a/lib.rs` and `crates/b/lib.rs`
- WHEN `clm check` runs
- THEN it exits 1 with the line `crates/b/lib.rs: check/unowned-files: in no component's paths; add it to a component's paths, or to outside in specs/SPEC.md until a change adopts it`, and none for `crates/a/lib.rs`

#### Scenario: owned by tests
- GIVEN component `a` with `paths: ["crates/a/**"]` and `tests: ["tests/a/**"]`, a root with `outside: []`, and tracked files `crates/a/lib.rs` and `tests/a/t.rs`
- WHEN `clm check` runs
- THEN it reports no `check/unowned-files` violation

#### Scenario: not opted in
- GIVEN the same files, and a root with no `outside` key
- WHEN `clm check` runs
- THEN it reports no `check/unowned-files` violation

#### Scenario: declared not yet adopted
- GIVEN the same files, and the root with `outside: ["crates/b/**"]`
- WHEN `clm check` runs
- THEN it reports no `check/unowned-files` violation

#### Scenario: part of an outside directory adopted
- GIVEN component `api` with `paths: ["src/api/**"]`, the root with `outside: ["src/**"]`, and tracked files `src/api/x.rs` and `src/web/y.rs`
- WHEN `clm check` runs
- THEN it reports no `check/unowned-files` violation

#### Scenario: outside glob left behind
- GIVEN components `a` and `b` with `paths` `["crates/a/**"]` and `["crates/b/**"]`, the root with `outside: ["crates/b/**"]`, and tracked files `crates/a/lib.rs` and `crates/b/lib.rs`
- WHEN `clm check` runs
- THEN it exits 1 with the line `specs/SPEC.md: check/unowned-files: outside glob crates/b/** matches no file outside every component`

#### Scenario: spec paths and untracked files
- GIVEN component `a` with `paths: ["crates/a/**"]`, a root with `outside: []`, tracked files `docs/guide/x.md`, `backlog/y.md`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `AGENTS.md` and `crates/a/lib.rs`, and an untracked file `notes/todo.txt`
- WHEN `clm check` runs
- THEN it reports no `check/unowned-files` violation

#### Scenario: extracted tree
- GIVEN a directory that isn't a git work tree, holding `specs/SPEC.md` with `outside: [".ci/**"]` and component `a` with `paths: ["crates/a/**"]`, and the files `crates/b/lib.rs` and `.ci/build.yml`
- WHEN `run_listed` is called on it with the listing `specs/SPEC.md`, `crates/b/lib.rs` and `.ci/build.yml`
- THEN it reports `crates/b/lib.rs` as `check/unowned-files`, and no `outside` glob

#### Scenario: no work tree, no listing
- GIVEN the same directory
- WHEN `run` is called on it
- THEN it reports no `check/unowned-files` violation

#### Serves
- capability/adopt-clm
- architecture/incremental-adoption

#### Needs
- model/outside-globs
- model/test-ownership-keys

### Requirement: node-size <!-- base: 7fb9fb0c85660eed -->
`clm size` SHALL print one line per component, sorted by ID, and `clm size <component>` only that component's line: `<component>`, `spec <tokens>/<budget>` and `code <tokens>/<budget>`, tab-separated, followed by a tab and `oversized` when the component is oversized, and SHALL exit 0. A component is *oversized* when either count exceeds a budget its frontmatter doesn't set for that count, or exceeds a `size_budget` it does set (`oversized-is-a-state`). A count's tokens SHALL be the total bytes of the parts it counts divided by 4, rounded up once. Spec tokens SHALL count the component's `SPEC.md`. Code tokens SHALL count, among the files `git ls-files --cached` lists, except a file with a NUL byte in its first 8000 bytes or whose `linguist-generated` or `linguist-vendored` attribute is set or `true` (any other value, `false` and unset don't exclude it): every part holding a `req:` marker that cites one of the component's requirements, wherever it sits, and every part holding no `req:` marker of a file the component owns (`check.coverage/owner`, so a file its `tests` match among them) (`tests-count-where-cited`). A file SHALL be one part, except a `.rs` file holding a line whose text, trimmed, is exactly `#[cfg(test)]`, which SHALL be two: the bytes from the first such line to the end of the file, and the bytes before it (`inline-tests-count-where-cited`). A part with markers SHALL NOT count toward a component its markers don't cite, even when that component's `paths` hold its file. A marker's component is the part of its ID before the `/`, as `check.coverage/pending-report` reads it, so a marker citing a requirement the component no longer holds still counts toward that component, and a part whose markers name only components that don't exist counts toward none. Each budget SHALL be the component's `size_budget` value for it when set, else the default passed in. `size(root, graph, markers, defaults)` SHALL return each component's counts, budgets and whether it is oversized, for `clm status`, `clm view` and `clm rebase`. `clm size` SHALL exit 1 naming a component that doesn't exist, and SHALL exit 1 printing the error when the settings can't be loaded.

#### Scenario: over its spec budget
- GIVEN defaults of 1000 spec and 1000 code tokens, and component `a` with paths `["src/a/**"]`, a `SPEC.md` of 4001 bytes and one tracked file `src/a/x.rs` of 400 bytes with no markers
- WHEN `clm size a` runs
- THEN it prints `a`, `spec 1001/1000`, `code 100/1000` and `oversized`, tab-separated, and exits 0

#### Scenario: files that don't count
- GIVEN the same component, plus an untracked `src/a/y.rs`, a tracked `src/a/gen.rs` that `.gitattributes` marks `linguist-generated`, a tracked `src/a/vendor/z.js` marked `linguist-vendored`, and a tracked `src/a/logo.png` holding a NUL byte
- WHEN `clm size a` runs
- THEN its code count is still `code 100/1000`

#### Scenario: rounded once
- GIVEN component `a` whose only tracked files are `src/a/p.rs` of 1 byte and `src/a/q.rs` of 3 bytes, neither with markers
- WHEN `clm size a` runs
- THEN its code count is `code 1/1000`

#### Scenario: attribute set to false
- GIVEN component `a` with a tracked `src/a/x.rs` of 400 bytes that `.gitattributes` marks `linguist-generated=false`
- WHEN `clm size a` runs
- THEN its code count is `code 100/1000`

#### Scenario: a test counts where it is cited
- GIVEN components `a` (paths `["src/a/**"]`) and `b` (paths `["src/b/**"]`), a tracked `src/a/tests/b.rs` of 800 bytes whose only marker cites `b/x`, and a tracked `src/a/lib.rs` of 400 bytes with no marker
- WHEN `clm size` runs
- THEN `a`'s code count is `code 100/1000` and `b`'s is `code 200/1000`

#### Scenario: inline tests count where they are cited
- GIVEN components `a` (paths `["src/a/**"]`) and `b` (paths `["src/b/**"]`), and a tracked `src/a/w.rs` of 1200 bytes whose first 400 bytes hold no marker and whose last 800 bytes start with a line `#[cfg(test)]` and hold a marker citing `b/x`
- WHEN `clm size` runs
- THEN `a`'s code count is `code 100/1000` and `b`'s is `code 200/1000`

#### Scenario: inline tests without markers
- GIVEN component `a` with a tracked `src/a/v.rs` of 400 bytes holding a `#[cfg(test)]` line and no marker
- WHEN `clm size a` runs
- THEN its code count is `code 100/1000`

#### Scenario: a test citing two nodes counts for both
- GIVEN the same components and a tracked `tests/both.rs` of 400 bytes, under neither component's `paths`, with markers citing `a/y` and `b/x`
- WHEN `clm size` runs
- THEN both `a`'s and `b`'s code counts include its 100 tokens

#### Scenario: budget raised in the spec
- GIVEN component `a` with a `SPEC.md` of 4001 bytes and 400 bytes of code, whose frontmatter sets `size_budget: { spec_tokens: 2000 }`
- WHEN `clm size a` runs
- THEN it prints `a`, `spec 1001/2000` and `code 100/1000`, with no `oversized`

#### Scenario: budget outgrown
- GIVEN the same component with a `SPEC.md` of 8001 bytes
- WHEN `clm size a` runs
- THEN it prints `spec 2001/2000` and `oversized`

#### Scenario: every node
- GIVEN components `b` and `a`
- WHEN `clm size` runs
- THEN it prints the line for `a`, then the line for `b`

#### Scenario: unknown node
- GIVEN no component `zz`
- WHEN `clm size zz` runs
- THEN it exits 1 naming `zz`

#### Scenario: settings can't be loaded
- GIVEN a repository `clm.toml` with `[component_size]` and `spec_tokens = 0`
- WHEN `clm size` runs
- THEN it exits 1 naming `clm.toml` and `component_size`

#### Serves
- capability/know-health
- architecture/slim-components

#### Needs
- check.coverage/owner
- model/size-budget
- config/component-size-setting

## ADDED Requirements

### Requirement: tests-cite-their-owner
`clm check` SHALL report each `req:` marker, in a file of its scan that a component's `tests` match, that cites a requirement of a component other than that file's owner (`paths-disjoint`), as `<file>:<line>: check/tests-cite-their-owner: cites <requirement> of `b`, but the file is `a`'s test; move the test to `b`'s tests`, and `run` SHALL report it too. A marker citing a capability (`capability/<slug>`) SHALL be read as citing the `capability` component's requirement, which proves it (`human-layer-proof`). A file that only `paths` match SHALL NOT be judged, so a project gives its tests to their components one `tests` glob at a time (`components-own-their-tests`), and a marker that names no existing requirement is left to `dangling-markers`.

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

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

#### Needs
- model/test-ownership-keys
