---
title: Check
status: active
owners: ["@clunny"]
depends_on: [model, apply, vcs, init, check.coverage]
paths: ["crates/clm-check/**"]
successor: "20261021-walking-skeleton"
serves: [capability/catch-drift, architecture/no-silent-drift]
---

## Contract
`clm check` runs every deterministic rule and exits non-zero on any violation, printing one actionable line per violation. Rules include schema, graph integrity, `paths` that two components share (`paths-disjoint`), imports between components that `depends_on` doesn't declare (`undeclared-imports`), code files in no component's `paths` (`unowned-files`), requirement test coverage, skipped tests in files that cite requirements (`skipped-tests`), names of requirements a live change removed (`removed-requirement-names`), references to component assets, stale deltas, and `clm:remove-by` markers whose change isn't live (`expand-contract`). When either human layer holds an entry, every rule must serve one (`check/untraced`, `traced-rules`). Path ownership (`unowned-files`), in a repository whose root sets `outside` (even to `[]`), means every tracked code file is in some component's `paths` or matched by the root's `outside`, the code not yet adopted, and every `outside` glob matches code no component owns (`incremental-adoption`). A root without the key isn't checked for it, so a repository opts in by setting it; `clm init` always does. The `clm check` command also runs the dead-code and unused-dependency detectors the repository's `clm.toml` names (`no-dead-code`, `dead-code-detectors`), each for at most its `timeout` (`detector-timeouts`), unless it is given `--no-detectors`. A caller that knows no code changed, such as `clm land` on code already checked, passes it to skip them.

A test cites the requirement it proves as `req:<id>@<version>`, where the version is the first 8 hex digits of the requirement's content hash (`model/content-hash`, `versioned-markers`). The marker says which text the test was written for. A requirement is *current* when markers cite it and every one cites its current version; *behind* when a marker cites it at another version or without one, even beside one that cites the current version (`stale-markers-keep-behind`); *unbuilt* when nothing cites it; and *obsolete* when a marker cites a requirement that a change outside the archive removed (`removed-requirements-build-out`). The last three are *pending*: the component's code hasn't caught up with its spec. A capability is cited like a rule. An architecture entry is never cited: it is current when every rule its `#### Enforced by` names is current, and an entry or capability holding `#### Untested` is never pending, so neither is in the queue (`human-layer-proof`, `check.coverage/coverage-report`). `clm pending [<component>]` lists them (`check.coverage/pending-report`), and the queue (`orchestrator`) is built on it. A change is *complete* as `check.coverage/status-report` defines it.

The markers and the states are computed by the child component `check.coverage` (crate `clm-coverage`), and `clm-check` re-exports its library, so a component that depends on `check` keeps calling it through `clm-check` (`check-coverage-split`): `markers_in(path, text)` returns the markers `clm coverage` would find in one file (none for Markdown), with their versions; `report(graph, markers)` turns the markers of every file into the coverage `clm coverage` prints; `pending(graph, markers)` derives the pending list from them; `change_progress(graph, markers)` gives each live change's progress, the counts `clm status` prints for it (`check.coverage/change-progress`); `status_report(graph, markers)` and `complete_changes`, built on it, give `clm status`'s sections and the complete changes; and `scan(root)` reads a checkout's markers. The crate builds for `wasm32-unknown-unknown` with its default features disabled (`check-builds-for-wasm`). Which component a file belongs to is available as library calls too, for `check.affected`: `owner`, `path_matches` and `scan` (`path-ownership`).

`clm size [<component>]` reports each component's size in tokens against its budget, marking the components over either (`node-size`, `node-size-budgets`). It is advisory: it exits 0 whatever the sizes (1 only for an unknown component or settings that can't be loaded), and no rule of `clm check` reads them.

## Design
`run` loads the graph (`model`), validates pending changes (`apply`), then runs the coverage, marker and text rules on the markers `check.coverage`'s `scan` finds in the files `git ls-files --cached --others --exclude-standard` lists. A marker without a version cites no version: it counts for `dangling-markers`, and the requirement it names is `behind` while it stays, so a bare marker never makes a requirement current, alone or beside a versioned one. The same scan finds removal markers: text matching the regex `clm:remove-by (\d{8}-[a-z0-9-]+)`. Text that doesn't match, such as `clm:remove-by <change-id>` in help output, isn't a marker. To mention a marker in code without claiming coverage or a removal, such as a test fixture, build the string so the literal never appears whole (e.g. `concat!("req", ":a/x")`); a fixture that holds a skipped test is built the same way.

A removal marker's change is classified from the graph (`model`), in this order: archived (it is under `changes/archive/`, whatever its status), missing (no change with that ID, including a directory without a parseable `change.md`), `done`, `abandoned`, or live (`proposed`, `approved`, `applied`, `implementing`). Violations use the marker's `<file>:<line>` as their path, as `dangling-markers` does, with messages `marker names <id>, which is archived`, `… which doesn't exist`, `… which is done` or `… which is abandoned`.

**Re-exports** (`check-coverage-split`, `nested-splits`). `check` keeps the rules `clm check` runs, the detectors and `clm size`; `check.coverage` keeps what says how far the code has caught up with the spec, which the queue, the views, `clm rebase` and the Worker read. `clm-check` depends on `clm-coverage` and re-exports its public items (`Coverage`, `Marker`, `State`, `markers_in`, `report`, `Pending`, `pending`, `Row`, `StatusReport`, `status_report`, `complete_changes`, `ChangeProgress`, `change_progress`, `removed`, `matches` (also as `path_matches`), `owner`, and with the `checkout` feature `Scan` and `scan`), and its `checkout` feature turns on `clm-coverage`'s. `pending_in`, `status`, `coverage` and `backlog`, the checkout wrappers the `clm` commands call, live in `clm-check`; `pending_in`'s unknown-node violation carries the rule `check.coverage/pending-report`, the requirement it enforces. `run` matches `paths` with `clm-coverage`'s `matches`, as `pending` does. The `dangling-markers` rule (`violations` in `coverage.rs`) lives in `clm-check` and takes the removed IDs from `clm-coverage`'s `removed`, as `removed-requirement-names` does. Tests that call `clm_check::` live in `crates/clm-check/tests`, citing the child's requirements where they prove them, since `clm-coverage` can't depend on `clm-check` (`undeclared-imports`, and a cycle); the child's own tests call `clm-coverage` directly.

**Shared paths** (`paths-disjoint`). `run` lists the files `git ls-files --cached` lists, Markdown included, itself through `git` (outside a git work tree, none). A file belongs to one component, the one whose builds own it and whose size it counts toward, so `nested-splits` keeps `paths` disjoint, parents and children included.

**Undeclared imports** (`undeclared-imports`, `undeclared-imports-check`, `tree-sitter-analysis`). The import edges come from `init`'s `import_edges(root)`, the same tree-sitter extraction `clm survey` uses: Rust workspace path dependencies and `mod`/`use crate::` paths, TypeScript relative imports and workspace packages, and Python imports that resolve to files in the repository. Tree-sitter is C, which doesn't build for `wasm32-unknown-unknown`, so `run` never reports this rule; the `clm check` command (crate `clm`) does, after `run` and before the detectors, from the graph it loaded, and `--no-detectors` doesn't skip it, since it needs no tool. A file's components are the components whose `paths` match it, each judged (two of them is `paths-disjoint`'s report). Only a direct `depends_on` counts, and a parent or child gets no exemption, since `depends_on` is separate from ownership (`nested-splits`). `import_edges` returns a `Result`: outside a git work tree it returns no edges, as `scan` finds no markers there, so a fixture that isn't a repository passes; when `git` fails inside one, the command reports `<root>: check/undeclared-imports: <error>` and goes on. `clm rebase` calls `run`, so its `rebase ok` doesn't cover this rule; `clm land` runs `clm check` and does.

**Imports before apply** (`undeclared-imports-before-apply`). A change's deltas can rewrite a component's `depends_on` and `paths`, so `undeclared_imports` also judges the same edges against the graph each `proposed` or `approved` change would leave, which `apply`'s `resulting(graph, change)` gives (`apply/resulting-graph-call`), as `removed-names-before-apply` does for names. A pair both graphs leave undeclared is reported once, by the current graph's line. Each change is judged alone against the current graph, so two pending changes that drop the same dependency each report it, naming themselves. A change whose deltas don't apply has no resulting graph: `stale-deltas` reports it. The edges are the code's as it is, read once. So a draft whose delta drops a `depends_on` entry its component's code uses meets the report in its own `clm check`, before `clm apply`, not first at `clm land`. A pair is its importing file, imported file and the IDs of `a` and `b`. A pair a change makes newly judged, by adding component `a` or `b` or by changing `paths` so a file falls in one, is reported the same way. The pending changes are those `is_pending` gives, as for `run`'s scan. The lines come after the current graph's, change by change in ID order, each in the order of the edges. The judging of resulting graphs sits behind `clm-check`'s `checkout` feature, which brings in `clm-apply` for `run`'s own before-apply scan: `undeclared_imports` is public and ungated, and without the feature it judges the current graph alone. `clm rebase` and `clm gate` don't run the rule: once the change is applied the current graph's line reports it, and `clm land` runs `clm check`.

**Skipped tests** (`skipped-tests`, `skipped-tests-in-cited-files`). A marker counts once the suite passes, so a test that never runs would prove a requirement it doesn't check. CLM reads no test runner's output, so rather than judge each marker's test, `run` reports a skipped test anywhere in a file that holds at least one `req:` marker; a skipped test belongs in a file that cites nothing. A file is read by its extension. For TypeScript and JavaScript the skip patterns are `(it|test|describe)\.(skip|skipIf|todo)\b` and `\bx(it|test|describe)\(`, not preceded by a letter, digit, `_` or `.`. Markdown is never read, as for markers.

**Removed requirement names** (`removed-requirement-names`). A requirement ID is *removed* when `check.coverage`'s `removed` gives it; a change not yet applied removes nothing from `specs/` (its removals are judged on the graph it would leave, below), and a requirement added back isn't removed. `run` reads each component's `SPEC.md` below its frontmatter, skipping fenced code blocks. The path is `specs/<component path>/SPEC.md:<line>`. The rule reports only in component specs: it reads `ARCHITECTURE.md` and `CAPABILITIES.md` for their entries' slugs alone (the model loads them), and never reports in them, `CLAUDE.md`, decisions or changes, whose text names decisions and entries rather than component requirements.

**Before apply** (`removed-names-before-apply`). For each `proposed` or `approved` change, the changes `run` validates, `run` also takes the graph the change would leave from `apply`'s `resulting(graph, change)` (`apply/resulting-graph-call`) and runs the same scan on it. The removed IDs are computed from that change's deltas and the two graphs, not from `removed`, and each is attributed to that change alone: when two pending changes remove the same ID, each one's scan reports its spans, naming itself. Only IDs listed under `## REMOVED Requirements` count, as for `removed`, so a component the change removes whole adds none of its requirements. `validate` builds the same graph again; the cost is one in-memory parse of the components the change writes, so the two aren't shared. So a draft sees a name its removal leaves stale before `clm apply`, and a name the change's own deltas rewrite is never reported. The path is the component's `SPEC.md` path with the line in the text the change leaves, which is the file's own line when the change doesn't write the component. An ID the current graph doesn't hold is the applied scan's, so no span is reported by both. A change whose deltas don't apply has no resulting graph (`resulting` gives none): `stale-deltas` reports it.

**Detectors** are separate from `run`. `detectors(root, list)` takes a plain list of `Detector { name, command, hint, timeout }` and spawns each, so `clm-check` doesn't depend on `clm-config`. The spawning code sits behind the default feature `detectors`, so the crate builds for wasm with default features disabled. The `clm check` command (crate `clm`) loads the settings (`config`) and passes their `detectors` in, each with its `timeout`. It calls `detectors` after `run`, whether or not `run` found violations. If the settings can't be loaded, it reports `<file or variable>: check/detectors: <error>` as a violation, skips the detectors and reports `run`'s violations. `clm rebase`, `clm wrap-up` (on an extracted tree) and the `web` Worker call `run`, which has no project toolchain, so they never run detectors.

Each detector runs without a shell, from the repository root, one after another in `clm.toml`'s order. Its stdout and stderr are piped and copied, line by line as they arrive, to `clm check`'s stderr, one thread per stream, so stdout keeps one line per violation and a detector that hangs has already shown what it printed (`detector-timeouts`). On Unix the detector starts in a process group of its own; when its `timeout` passes before it exits, the whole group is killed (`SIGKILL`, through `libc::killpg` or an equivalent call, not a spawned `kill`), so a `sh -c` wrapper's children stop with it. Elsewhere the process alone is killed, and a grandchild holding a pipe can keep the copying threads, and `clm check`, waiting past the timeout; CLM is released for no such platform (`no-windows-support`). `clm check` then waits for the copying threads and goes on to the next detector. A detector with no exit code whose timeout didn't pass is reported as `<name> failed (killed by a signal)`.

**Size** (`node-size`, `oversized-is-a-state`, `tests-count-where-cited`, `inline-tests-count-where-cited`). A token is estimated as four bytes, rounded up, with no tokenizer, so the count is deterministic and cheap. A component's *spec tokens* count its `SPEC.md` (`specs/SPEC.md` for the root); its assets don't count, since a context bundle lists them rather than including them. Its *code tokens* count what a build of the component reads: among the files `clm-git`'s `tracked_files` lists, each once even mid-merge, every part of a file whose `req:` markers cite one of its requirements, wherever it sits, and every part with no markers among `clm-git`'s `tracked_matching` of its `paths`, the same list `clm survey --components` reads, so the two agree on which files a component owns (`tracked-files-in-git-layer`). `size` hands those paths back to `git check-attr --stdin -z` as their bytes (`OsStrExt::as_bytes` on Unix). A file is one part, except a Rust file (`.rs`) holding a line whose text, trimmed, is exactly `#[cfg(test)]`: from the first such line to the end of the file is its *test part*, and what comes before it is its *source part*, since Rust keeps unit tests in a module at the end of the file they test. A test file, or a test part, counts toward each component it cites and not toward the component whose `paths` merely hold it, so `crates/clm/tests/run.rs` is `orchestrator.run`'s cost, not the root's, while the source part of `crates/clm-orchestrator/src/queue.rs` stays its owner's whatever its inline tests cite. Ignored files such as `node_modules/` and `target/` never count. Two kinds of tracked file are left out, being nothing a session reads: a binary file, one with a NUL byte in its first 8000 bytes as git decides, and a file whose `linguist-generated` or `linguist-vendored` attribute is set (`git check-attr`; `set` or `true`, not `false` or unset), the `.gitattributes` convention GitHub uses for lockfiles, built bundles and vendored code (`reuse-first`). The bytes of a count's parts are summed and rounded up once. Each count's budget is the component's `size_budget` for it (`model/size-budget`), else the default the caller passes in. A component is *oversized* when a count exceeds its budget; a `size_budget` that covers the count is the exception a human chose in a spec PR, and one the component has outgrown isn't. Nothing fails for it: `size` reports the state and `view` and `check.health` show it; nothing files an item for it (`oversized-shown-not-filed`). `size(root, graph, markers, defaults)` reads the working tree and sits behind the default feature `detectors`, so the crate builds for wasm. The `clm size` command (crate `clm`) loads the settings (`config`) and passes `component_size` in, as it does the detectors for `clm check`.

The counting itself is `size_of(graph, files, markers, defaults)` (`size-of-listing`), built without features so the web app's `replica`, which has a Trees API listing but no working tree, counts as `clm size` does (`replica/snapshot-size`). `files` maps each tracked file that may count to its length in bytes, with binary and linguist-excluded files already left out; each component's `SPEC.md` length is read from it, and one missing from it counts 0. `size` lists and probes the working tree as above, puts each component's `SPEC.md` length in from disk (tracked or not), and calls `size_of`, so the two can't drift. `size_of` matches `paths` with `glob::matches`, as `pending` does. The Worker can't run `git check-attr`, so `linguist_excluded(attributes, path)` (`linguist-attributes`) decides the linguist exclusion from the texts of the tree's `.gitattributes` files, each with its directory: patterns as gitattributes reads them (relative to their file's directory, a pattern without `/` matching at any depth below it, `**`, a trailing `/` matching no file), the last matching line deciding, deeper files over shallower ones, `-attr` and `!attr` unsetting, `attr=false` not setting, and `[attr]<name>` macro lines in the root `.gitattributes` expanded where a later line names the macro (git's built-in `binary` macro sets no linguist attribute, so it changes nothing here). It reads no `.git/info/attributes` or `core.attributesFile`, which a hosted listing doesn't have; `size` keeps asking `git check-attr`, which also honours those, so a clone's local attribute files can make `clm size` differ from the Worker, and only for them. An existing crate that matches gitattributes without git, such as `gix-attributes`, is preferred to writing the matcher (`reuse-first`) if it builds for wasm and keeps the Worker inside its size and CPU limits (`web-on-free-plan`); otherwise the matcher is written here.

## Requirements

### Requirement: report-and-exit
`clm check` SHALL print one line per violation as `<path>: <rule>: <message>` and exit 1 if there are any violations, 0 otherwise. It SHALL report all violations, not just the first.

#### Scenario: two violations
- GIVEN a repo with a dependency cycle and a duplicate requirement slug
- WHEN `clm check` runs
- THEN both are printed and the exit code is 1

#### Scenario: clean repo
- GIVEN this repository's own `specs/`
- WHEN `clm check` runs
- THEN it exits 0

### Requirement: stale-deltas
`clm check` SHALL report every violation `apply` validation finds for each change whose status is `proposed` or `approved`. Changes in other statuses are history and are not validated against current specs.

#### Scenario: stale pending change
- GIVEN a `proposed` change whose delta's contract base no longer matches
- WHEN `clm check` runs
- THEN it exits 1 naming the delta and the stale element

### Requirement: dangling-markers
`clm check` SHALL report every `req:<id>` marker in a scanned file whose `<id>` names no existing requirement or human-layer entry (`model/human-layers`), giving the file and line; and every marker that cites an architecture entry, or an entry holding `#### Untested`, with the message `architecture entries are proven by the rules their #### Enforced by names, not by tests` or `<id> is untested`, since a test never proves either (`human-layer-proof`); unless a change outside the archive lists `<id>` under `REMOVED Requirements` in one of its deltas: that code is obsolete and is removed when the component is next built (`removed-requirements-build-out`). Once the change is archived, the marker SHALL be reported as before. An `abandoned` change is archived (`archive-done`), so it grants no exemption; a requirement another change adds back under the same ID is a new requirement once that change applies, and the marker then cites it as usual.

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

#### Scenario: removing change archived
- GIVEN the same files, with change `x` under `changes/archive/`
- WHEN `clm check` runs
- THEN it exits 1 reporting `tests/t.rs:3` and `a/gone`

#### Needs
- model/human-layers

### Requirement: archive-done
`clm check` SHALL report a `done` or `abandoned` change that is not under `changes/archive/`, and an archived change in any other status.

#### Scenario: finished but not archived
- GIVEN `changes/x/` with `status: done`
- WHEN `clm check` runs
- THEN it exits 1 telling the author to run `clm archive x`

#### Serves
- capability/catch-drift
- architecture/spec-is-current

### Requirement: backlog-report
`clm backlog` SHALL print one line per backlog item, sorted by priority (`P1` first), then kind, then slug: `<slug> <kind> <priority> [<components>] <title>`. It SHALL exit 0.

#### Scenario: two items
- GIVEN items `b` (idea, components `[sync]`) and `a` (deferred, no components), neither with a priority
- WHEN `clm backlog` runs
- THEN it prints `a deferred P5 [] <a's title>` then `b idea P5 [sync] <b's title>`

#### Scenario: priority first
- GIVEN items `a` (bug, no priority) and `b` (idea, `priority: P2`)
- WHEN `clm backlog` runs
- THEN it prints `b idea P2 [] <b's title>` then `a bug P5 [] <a's title>`

#### Serves
- capability/keep-a-backlog
- capability/see-project-state

### Requirement: asset-references
`clm check` SHALL fail when a component's Contract or Design mentions, in a code span, a concrete path of the form `` `assets/<path>` `` that isn't one of that component's assets, naming the component and the path. A path is concrete when it contains no `<`, `>` or `*` and doesn't end in `/`. Mentions in requirements, scenarios and frontmatter never count.

#### Scenario: missing mockup
- GIVEN component `a` whose Design mentions `` `assets/m.html` `` and no file `specs/a/assets/m.html`
- WHEN `clm check` runs
- THEN it exits non-zero naming `a` and `assets/m.html`

#### Scenario: placeholders and directories
- GIVEN component `a` whose Contract mentions `` `assets/<path>` ``, `` `assets/<node-id>/**` `` and `` `assets/mockups/` ``, and no assets
- WHEN `clm check` runs
- THEN it reports no asset reference for `a`

#### Scenario: scenario examples
- GIVEN component `a` with no assets, whose only mention of `` `assets/m.html` `` is in a scenario
- WHEN `clm check` runs
- THEN it reports no asset reference for `a`

### Requirement: check-builds-for-wasm
The `clm-check` crate SHALL build for `wasm32-unknown-unknown` with its default features disabled.

#### Scenario: wasm build
- GIVEN the workspace with the `wasm32-unknown-unknown` target installed
- WHEN `cargo build -p clm-check --no-default-features --target wasm32-unknown-unknown` runs
- THEN it succeeds

#### Serves
- capability/follow-from-browser
- architecture/web-optional

### Requirement: detectors
`clm check` SHALL run each detector in the repository's `clm.toml` from the repository root, copying its stdout and stderr to its own stderr as they arrive, and report a violation `clm.toml: check/detectors: <name> failed (exit <code>)` for each that exits non-zero. A detector still running when its `timeout` has passed SHALL be stopped, together with the processes it started, and reported as `clm.toml: check/detectors: <name> timed out after <n>s`, and `clm check` SHALL go on to the next detector. A detector whose command can't be started SHALL be reported as `clm.toml: check/detectors: <name> could not run: <error>`, followed by `; install with: <hint>` when it has a hint. `run` SHALL NOT run detectors. `clm check --no-detectors` SHALL run every other rule as `clm check` does, but run no detector and report no `check/detectors` violation, so it needs neither the detectors' tools nor loadable settings.

#### Scenario: failing detector
- GIVEN a `clm.toml` with detectors `ok` (`["true"]`) and `bad` (`["sh", "-c", "echo unused-dep; exit 3"]`)
- WHEN `clm check` runs
- THEN stderr contains `unused-dep`, stdout has the line `clm.toml: check/detectors: bad failed (exit 3)` and nothing for `ok`, and the exit code is 1

#### Scenario: hanging detector
- GIVEN a `clm.toml` with a detector `hang` (`["sh", "-c", "echo started; sleep 30 & echo $! > child.pid; wait"]`, `timeout = 1`) followed by a detector `bad` (`["sh", "-c", "exit 3"]`)
- WHEN `clm check` runs
- THEN it exits 1 well before 30 seconds, stderr contains `started`, stdout has the lines `clm.toml: check/detectors: hang timed out after 1s` and `clm.toml: check/detectors: bad failed (exit 3)`, and the process whose ID `child.pid` holds is no longer running

#### Scenario: tool not installed
- GIVEN a detector `machete` with command `["no-such-tool-xyz"]` and hint `cargo install cargo-machete`
- WHEN `clm check` runs
- THEN it exits 1 with a line starting `clm.toml: check/detectors: machete could not run:` and ending `; install with: cargo install cargo-machete`

#### Scenario: run skips detectors
- GIVEN the same failing `clm.toml`
- WHEN `clm_check::run` is called on the repository
- THEN it returns no `check/detectors` violation

#### Scenario: no detectors
- GIVEN a repository whose `clm.toml` names no detectors, or that has no `clm.toml`
- WHEN `clm check` runs
- THEN no `check/detectors` violation is reported

#### Scenario: detectors skipped on request
- GIVEN a clean repository whose `clm.toml` has a detector `bad` (`["sh", "-c", "touch ran; exit 3"]`)
- WHEN `clm check --no-detectors` runs
- THEN it exits 0, prints no `check/detectors` line, and no file `ran` exists

#### Serves
- capability/no-dead-weight
- architecture/no-dead-code

#### Needs
- config/detector-timeout-setting

### Requirement: remove-by-markers
`clm check` SHALL report every `clm:remove-by <change-id>` marker in a scanned file whose change is not live, with the marker's `<file>:<line>` as the path and a message naming the change and whether it is archived, missing, `done` or `abandoned`. A change is live when it is not archived and its status is `proposed`, `approved`, `applied` or `implementing`.

#### Scenario: contract change finished
- GIVEN `src/old.rs` line 4 holds a marker for `20261023-drop-old`, whose `change.md` has status `done`
- WHEN `clm check` runs
- THEN it exits 1 with the line `src/old.rs:4: check/remove-by-markers: marker names 20261023-drop-old, which is done`

#### Scenario: contract change archived
- GIVEN the same marker, and `20261023-drop-old` only under `changes/archive/` with status `done`
- WHEN `clm check` runs
- THEN it exits 1 reporting `src/old.rs:4` and that `20261023-drop-old` is archived

#### Scenario: no contract change recorded
- GIVEN a marker naming `20261023-nowhere`, which has no directory under `changes/`
- WHEN `clm check` runs
- THEN it exits 1 reporting the marker and that `20261023-nowhere` doesn't exist

#### Scenario: pending contract change
- GIVEN a marker naming `20261023-drop-old`, whose status is `applied`
- WHEN `clm check` runs
- THEN no `check/remove-by-markers` violation is reported

#### Scenario: not markers
- GIVEN a Markdown file with a marker for a missing change, and a Rust file containing the text `clm:remove-by <change-id>`
- WHEN `clm check` runs
- THEN no `check/remove-by-markers` violation is reported

#### Serves
- capability/no-dead-weight
- architecture/no-dead-code
- architecture/expand-contract

### Requirement: priority-caps
`clm check` SHALL report a `check/priority-caps` violation for each priority that has more holders than its cap: at most 1 `P1`, 2 `P2`, 3 `P3` and 4 `P4`, and any number of `P5` (`priority-caps`). Backlog items and changes share the caps. The holders are:
- each change outside the archive whose status isn't `abandoned`, at its priority;
- each backlog item at its priority, unless a change that holds a priority names it in `resolves`. That change carries the work, so it isn't counted twice. An item promoted with `from` is deleted in the same spec PR, so its priority moves to the change.

The violation names the priority, its cap, and every holder, sorted, as `changes/<id>` or `backlog/<slug>`, so the author can see what to demote. A demotion edits an existing item or `change.md`, so it is a spec change like any other.

#### Scenario: within the caps
- GIVEN change `x` with `priority: P1`, backlog items `a` and `b` with `priority: P2`, and items with no priority
- WHEN `clm check` runs
- THEN it reports no `check/priority-caps` violation

#### Scenario: two P1s
- GIVEN change `x` with `priority: P1` and backlog item `a` with `priority: P1`
- WHEN `clm check` runs
- THEN it exits 1 with a `check/priority-caps` violation naming `P1`, cap 1, `backlog/a` and `changes/x`

#### Scenario: resolved item not counted twice
- GIVEN change `x` with `priority: P1` and `resolves: [a]`, and backlog item `a` with `priority: P1`
- WHEN `clm check` runs
- THEN it reports no `check/priority-caps` violation

#### Scenario: archived and abandoned changes
- GIVEN change `x` with `priority: P1` under `changes/archive/`, change `y` with `priority: P1` and `status: abandoned`, and backlog item `a` with `priority: P1`
- WHEN `clm check` runs
- THEN it reports no `check/priority-caps` violation

#### Serves
- capability/keep-a-backlog

### Requirement: node-size
`clm size` SHALL print one line per component, sorted by ID, and `clm size <component>` only that component's line: `<component>`, `spec <tokens>/<budget>` and `code <tokens>/<budget>`, tab-separated, followed by a tab and `oversized` when the component is oversized, and SHALL exit 0. A component is *oversized* when either count exceeds a budget its frontmatter doesn't set for that count, or exceeds a `size_budget` it does set (`oversized-is-a-state`). A count's tokens SHALL be the total bytes of the parts it counts divided by 4, rounded up once. Spec tokens SHALL count the component's `SPEC.md`. Code tokens SHALL count, among the files `git ls-files --cached` lists, except a file with a NUL byte in its first 8000 bytes or whose `linguist-generated` or `linguist-vendored` attribute is set or `true` (any other value, `false` and unset don't exclude it): every part holding a `req:` marker that cites one of the component's requirements, wherever it sits, and every part holding no `req:` marker of a file that matches one of the component's `paths` (`tests-count-where-cited`). A file SHALL be one part, except a `.rs` file holding a line whose text, trimmed, is exactly `#[cfg(test)]`, which SHALL be two: the bytes from the first such line to the end of the file, and the bytes before it (`inline-tests-count-where-cited`). A part with markers SHALL NOT count toward a component its markers don't cite, even when that component's `paths` hold its file. A marker's component is the part of its ID before the `/`, as `check.coverage/pending-report` reads it, so a marker citing a requirement the component no longer holds still counts toward that component, and a part whose markers name only components that don't exist counts toward none. Each budget SHALL be the component's `size_budget` value for it when set, else the default passed in. `size(root, graph, markers, defaults)` SHALL return each component's counts, budgets and whether it is oversized, for `clm status`, `clm view` and `clm rebase`. `clm size` SHALL exit 1 naming a component that doesn't exist, and SHALL exit 1 printing the error when the settings can't be loaded.

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
- model/size-budget
- config/component-size-setting

### Requirement: git-through-layer
`clm-check` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `gh`, `run` or repo queries (`git-plumbing-layer`), and its own wrapper is gone (`no-dead-code`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-check` reads `crates/clm-check/Cargo.toml` and every file under `crates/clm-check/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")` or `Command::new("gh")`

#### Serves
- architecture/reuse-first
- architecture/no-dead-code

### Requirement: backlog-entry-shape
For each change outside the archive, whatever its status, `clm check` SHALL report a `check/backlog-entry-shape` violation, with the path `changes/<id>/change.md`, for:
- each `resolves` entry that isn't a bare slug: one that is empty, contains `/`, or ends in `.md`;
- a `from` that isn't `backlog/<slug>` with a bare slug.

The suggested slug is the entry with one leading `backlog/` removed, then one trailing `.md` removed; it is used only when it is a bare slug. The message SHALL name the change and the entry, and say what to write instead: ``change `c` resolves `backlog/x`: `resolves` entries are bare slugs; write `x` `` for `resolves`, and ``change `c` has `from: x`: `from` is `backlog/<slug>`; write `backlog/x` `` for `from`. When the suggested slug isn't a bare slug, the message SHALL end instead with `; name a top-level backlog item` and suggest nothing. Abandoned changes are tested like the rest. The rule tests the entries' shape only, never whether the item exists: a `from` item is deleted in the spec PR that promotes it, and the build that fixes a `resolves` item deletes it before the change is archived (`task-deletes-fixed-items`), so whether an entry ever named an item stays `gate/resolves-named-items`'s to report. Changes under `changes/archive/` SHALL NOT be tested (`archive-immutable`).

#### Scenario: resolves written as a claim path
- GIVEN change `c` outside the archive, with `status: proposed` and `resolves: [backlog/x, y]`, and `backlog/x.md` and `backlog/y.md`
- WHEN `clm check` runs
- THEN it exits 1 with one `check/backlog-entry-shape` violation at `changes/c/change.md`, naming `c` and `backlog/x` and saying to write `x`

#### Scenario: resolves written as a file name
- GIVEN change `c` outside the archive with `status: applied` and `resolves: [x.md]`
- WHEN `clm check` runs
- THEN it exits 1 with a `check/backlog-entry-shape` violation naming `c` and `x.md` and saying to write `x`

#### Scenario: from written as a bare slug
- GIVEN change `c` outside the archive with `from: x`
- WHEN `clm check` runs
- THEN it exits 1 with a `check/backlog-entry-shape` violation naming `c` and `x` and saying to write `backlog/x`

#### Scenario: nested path
- GIVEN change `c` outside the archive with `resolves: [backlog/sub/x]`
- WHEN `clm check` runs
- THEN it exits 1 with a `check/backlog-entry-shape` violation naming `c` and `backlog/sub/x`, saying the entry must name a top-level backlog item, and suggesting no slug

#### Scenario: well-formed entries naming deleted items
- GIVEN change `c` outside the archive with `from: backlog/a` and `resolves: [b]`, and neither `backlog/a.md` nor `backlog/b.md`
- WHEN `clm check` runs
- THEN it reports no `check/backlog-entry-shape` violation

#### Scenario: archived change
- GIVEN change `c` under `changes/archive/` with `resolves: [backlog/x]`
- WHEN `clm check` runs
- THEN it reports no `check/backlog-entry-shape` violation

#### Serves
- capability/keep-a-backlog
- architecture/no-silent-drift

### Requirement: paths-disjoint
`clm check` SHALL report each file of its scan that the `paths` of two or more components match, as `<file>: check/paths-disjoint: owned by <components>`, the components in backticks, sorted by ID and comma-separated, once per file, and `run` SHALL report it too.

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

#### Serves
- architecture/path-ownership
- capability/work-concurrently

### Requirement: undeclared-imports
`clm check` SHALL report each import edge `init`'s `import_edges` returns from a file of component `a` to a file of component `b`, where `b` is not `a` and `a`'s `depends_on` doesn't list `b`, as `<importing file>: check/undeclared-imports: imports <imported file> of component `b`, and `a` doesn't depend on `b``, where a file's component is the component whose `paths` match it; an edge with an end no component's `paths` match SHALL NOT be reported. `clm check --no-detectors` SHALL report it as well, and `run` SHALL NOT. It SHALL also judge the same edges against the graph each `proposed` or `approved` change would leave (`apply/resulting-graph-call`), reporting each edge and component pair that graph leaves undeclared and the current graph doesn't as `<importing file>: check/undeclared-imports: imports <imported file> of component `b`, and `a` wouldn't depend on `b` after <change>`; a change whose deltas don't apply SHALL be skipped.

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

### Requirement: skipped-tests
`clm check` SHALL report, in each scanned file that holds a `req:` marker, each line that skips a test: in a `.rs` file a line starting `#[ignore` after leading whitespace; in a `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` or `.cjs` file a call of `it`, `test` or `describe` with `.skip`, `.skipIf` or `.todo`, or of `xit`, `xtest` or `xdescribe`; and in a `.py` file a line starting `@pytest.mark.skip` or `@unittest.skip` after leading whitespace, or holding `pytest.skip(`; as `<file>:<line>: check/skipped-tests: a skipped test in a file that cites requirements; run it, or move it to a file that cites none`. A file without markers, and a Markdown file, SHALL NOT be reported.

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

#### Serves
- capability/prove-by-tests
- architecture/proven-by-tests

### Requirement: removed-requirement-names
`clm check` SHALL report each inline code span outside fenced code blocks in a component's `SPEC.md`, below its frontmatter, whose whole text is a removed requirement ID `<component>/<slug>`, in any component's `SPEC.md`, or the bare `<slug>` of one, in the `SPEC.md` of `<component>` itself unless a requirement of any component has that slug, `docs/decisions/<slug>.md` exists or a human-layer entry has that slug; as `<spec file>:<line>: check/removed-requirement-names: names `<id>`, which <change> removes`. An ID is *removed* when `check.coverage`'s `removed` gives it: a change outside the archive lists the slug under `REMOVED Requirements` in its delta for the component and no component holds the ID; with several such changes, the message SHALL name the first by ID. It SHALL also report the same spans in the graph each `proposed` or `approved` change would leave (`apply/resulting-graph-call`), for the IDs that change's deltas list under `REMOVED Requirements` and the current graph holds, with shared slugs judged on that graph, as `<spec file>:<line>: check/removed-requirement-names: names `<id>`, which <change> would remove`, the line being that of the text the change leaves; a change whose deltas don't apply is skipped.

#### Scenario: requirement names a removed sibling
- GIVEN component `a` holding requirement `keep`, whose text names `` `gone` `` on line 30 of `specs/a/SPEC.md`, and an `applied` change `20260101-x` whose delta for `a` lists `gone` under `REMOVED Requirements`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``specs/a/SPEC.md:30: check/removed-requirement-names: names `a/gone`, which 20260101-x removes``

#### Scenario: another node names the full ID
- GIVEN the same change, and component `b` whose Design names `` `a/gone` ``
- WHEN `clm check` runs
- THEN it reports that line of `specs/b/SPEC.md` with `check/removed-requirement-names`

#### Scenario: shared slug
- GIVEN the same change, component `a`'s text naming `` `gone` ``, and a file `docs/decisions/gone.md`
- WHEN `clm check` runs
- THEN it reports no `check/removed-requirement-names` violation for the bare `` `gone` ``

#### Scenario: slug of a human-layer entry
- GIVEN the same change, component `a`'s text naming `` `gone` ``, and an `ARCHITECTURE.md` whose `## Constraints` holds `### Constraint: gone`
- WHEN `clm check` runs
- THEN it reports no `check/removed-requirement-names` violation for the bare `` `gone` ``

#### Scenario: not applied yet
- GIVEN component `a` holding requirement `keep`, a `proposed` change `20260101-y` whose delta for `a` lists `keep` under `REMOVED Requirements` with its base and which has no delta for `b`, and component `b` whose Design names `` `a/keep` `` on line 12 of `specs/b/SPEC.md`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``specs/b/SPEC.md:12: check/removed-requirement-names: names `a/keep`, which 20260101-y would remove``

#### Scenario: not applied yet, name rewritten
- GIVEN component `a` holding requirements `keep` and `other`, whose text names `` `keep` ``, and a `proposed` change whose delta for `a` removes `keep` and modifies `other` to name it no more, both with their bases
- WHEN `clm check` runs
- THEN it reports no `check/removed-requirement-names` violation

#### Scenario: not applied yet, stale delta
- GIVEN the change of `not applied yet`, its delta for `a` holding a stale base for `keep`
- WHEN `clm check` runs
- THEN it reports `apply/stale-base` for the delta and no `check/removed-requirement-names` violation

#### Scenario: change archived
- GIVEN the first scenario's files with change `20260101-x` under `changes/archive/`
- WHEN `clm check` runs
- THEN it reports no `check/removed-requirement-names` violation

#### Scenario: fenced example
- GIVEN the first scenario's change, and `` `gone` `` named only inside a fenced code block in `specs/a/SPEC.md`
- WHEN `clm check` runs
- THEN it reports no `check/removed-requirement-names` violation

#### Serves
- capability/catch-drift
- architecture/spec-is-current

#### Needs
- apply/resulting-graph-call
- model/human-layers

### Requirement: size-of-listing
`clm_check::size_of(graph, files, markers, defaults)` SHALL return each component's counts, budgets and oversized state by the rules `node-size` states, from `files`, a map from each tracked file that may count to its `Length`, rather than from a working tree: its bytes and, for a `.rs` file `node-size` counts as two parts, its test part's first line and bytes (`Length::of(path, text)` splits a file's text; `Length::from(bytes)` is a file of one part, which is all a listing without contents can give), and SHALL build for `wasm32-unknown-unknown` with the crate's default features off. `size` SHALL compute its result by calling `size_of` with the files it lists and probes.

#### Scenario: same as the working tree
- GIVEN a repository with component `a` whose `SPEC.md` is 4001 bytes and whose frontmatter sets `size_budget: { spec_tokens: 1000 }`, so it is oversized, component `b` with defaults and a 100-byte file under its `paths`, a 400-byte file under `a`'s `paths`, a test file elsewhere citing `a`'s requirement, a binary file under `a`'s `paths` and a lockfile marked `linguist-generated`
- WHEN `size_of` is called with the tracked files' lengths, leaving out the binary file and the lockfile, and `size` is called on the working tree
- THEN both return the same counts, budgets and oversized states for every component: `a` oversized with spec `1001/1000`, `b` not

#### Serves
- capability/follow-from-browser
- capability/know-health
- architecture/slim-components

### Requirement: linguist-attributes
`clm_check::linguist_excluded(attributes, path)` SHALL say whether `path` has `linguist-generated` or `linguist-vendored` set or `true`, given the texts of the tree's `.gitattributes` files, each with its directory, and SHALL agree with `git check-attr linguist-generated linguist-vendored` in a repository holding those files and no other attribute sources. It SHALL build for `wasm32-unknown-unknown` with the crate's default features off.

#### Scenario: agrees with git
- GIVEN a root `.gitattributes` holding `[attr]gen linguist-generated`, `Cargo.lock gen`, `vendor/** linguist-vendored` and `*.min.js linguist-generated`, and `web/.gitattributes` holding `package-lock.json linguist-generated=true` and `keep.min.js -linguist-generated`
- WHEN `linguist_excluded` is asked about `Cargo.lock`, `crates/x/Cargo.lock`, `vendor/a/b.rs`, `web/package-lock.json`, `web/keep.min.js`, `web/app.min.js` and `src/main.rs`
- THEN it answers as `git check-attr` does for each path in that repository: excluded for all but `web/keep.min.js` and `src/main.rs`

#### Serves
- capability/know-health
- architecture/slim-components

### Requirement: path-ownership
`clm-check` SHALL offer, as public library calls re-exported from `clm-coverage` (`check-coverage-split`), `owner(graph, path)` (`check.coverage/owner`), returning the component whose `paths` hold `path` (the deepest such component, then the first by ID), or none when no component's `paths` do; and `path_matches(pattern, path)`, which is `clm-coverage`'s `matches`, matching `path` against one glob as `pending` matches `paths` globs (git's `:(glob)` pathspec semantics). `scan(root)` stays public as it is. `size`'s counting is unchanged.

#### Scenario: owned and unowned
- GIVEN components `a` with `paths: ["crates/a/**"]` and `a.b` with `paths: ["crates/b/**"]`
- WHEN `owner` runs on `crates/a/src/lib.rs`, `crates/b/x.rs` and `tools/x.sh`
- THEN it returns `a`, `a.b` and none

#### Scenario: one glob
- GIVEN the pattern `crates/*/tests/**`
- WHEN `path_matches` runs on `crates/a/tests/t.rs` and `crates/a/src/t.rs`
- THEN it returns true and false

#### Serves
- architecture/path-ownership
- capability/work-concurrently

#### Needs
- check.coverage/owner

### Requirement: unowned-files
When the root sets `outside` (`model/outside-globs`), even to `[]`, `run` SHALL check path ownership; when the root has no `outside` key it SHALL NOT, and reports no `check/unowned-files` violation. It SHALL read the files the index holds under the CLM repository (`git ls-files --cached`, through the `git` layer), every entry the index lists, symlinks and submodules included; in a directory that isn't a git work tree, such as the tree `clm wrap-up` extracts, it SHALL read every file under it as the coverage scan's walk does. It SHALL leave out the spec paths: every file under `specs/`, `changes/`, `backlog/` or `docs/`, and `ARCHITECTURE.md`, `CAPABILITIES.md` and `CLAUDE.md`, the same spec paths `gate` names. Of the rest, the *code files*, it SHALL report each that no component's `paths` match, whatever the component's status, and no glob of the root's `outside` matches, as `<file>: check/unowned-files: in no component's paths; add it to a component's paths, or to outside in specs/SPEC.md until a change adopts it`. It SHALL report each `outside` glob that matches no code file that no component's `paths` match, as `specs/SPEC.md: check/unowned-files: outside glob <glob> matches no file outside every component`. A glob matches a file as the git pathspec `:(glob)<glob>` does; matching in memory by the same rules is enough. A code file that a component's `paths` and an `outside` glob both match is the component's, and SHALL NOT be reported. A file the index doesn't hold, untracked or ignored, SHALL NOT be read or reported.

#### Scenario: file in no node
- GIVEN component `a` with `paths: ["crates/a/**"]`, a root with `outside: []`, and tracked files `crates/a/lib.rs` and `crates/b/lib.rs`
- WHEN `clm check` runs
- THEN it exits 1 with the line `crates/b/lib.rs: check/unowned-files: in no component's paths; add it to a component's paths, or to outside in specs/SPEC.md until a change adopts it`, and none for `crates/a/lib.rs`

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
- GIVEN component `a` with `paths: ["crates/a/**"]`, a root with `outside: []`, tracked files `docs/guide/x.md`, `backlog/y.md`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `crates/a/lib.rs`, and an untracked file `notes/todo.txt`
- WHEN `clm check` runs
- THEN it reports no `check/unowned-files` violation

#### Scenario: extracted tree
- GIVEN a directory that isn't a git work tree, holding `specs/SPEC.md` with `outside: []` and component `a` with `paths: ["crates/a/**"]`, and the file `crates/b/lib.rs`
- WHEN `run` is called on it
- THEN it reports `crates/b/lib.rs` as `check/unowned-files`

#### Serves
- capability/adopt-clm
- architecture/incremental-adoption

#### Needs
- model/outside-globs

### Requirement: size-tracked-through-git
`size` SHALL list the tracked files it counts through `clm-git`'s `tracked_files`, and the files a component's `paths` match through `clm-git`'s `tracked_matching` (`tracked-files-in-git-layer`). `crates/clm-check/src/size.rs` SHALL NOT run `git ls-files` itself (`no-dead-code`).

#### Scenario: no copy of its own
- GIVEN the workspace
- WHEN a test under `crates/clm-check/tests` reads `crates/clm-check/src/size.rs`, comments included, looking for the text `ls-files` built from parts so the test doesn't match itself
- THEN it doesn't hold it

#### Serves
- architecture/reuse-first
- architecture/no-dead-code

#### Needs
- vcs/tracked-files

### Requirement: untraced
When either human layer holds an entry, `clm check` SHALL report each rule whose effective serves is empty as `<spec>: check/untraced: <id> serves nothing; give component <component> a serves list or the rule a #### Serves block`. Entries are never reported, and a repository whose layers hold no entries isn't checked (`traced-rules`).

#### Scenario: untraced rule
- GIVEN a capability entry, and component `a` without `serves` holding rule `a/x`
- WHEN `clm check` runs
- THEN it reports `specs/a/SPEC.md: check/untraced: a/x serves nothing; give component a a serves list or the rule a #### Serves block`

#### Scenario: rule block
- GIVEN the same, and rule `a/y` with a `#### Serves` block naming the entry
- WHEN `clm check` runs
- THEN it reports `a/x` and not `a/y`

#### Scenario: no entries
- GIVEN no `CAPABILITIES.md`, an `ARCHITECTURE.md` with neither a `## Constraints` nor a `## Requirements` section, and rule `a/x` serving nothing
- WHEN `clm check` runs
- THEN it reports no `check/untraced` violation

#### Serves
- architecture/traced-rules

#### Needs
- model/human-layers

### Requirement: unenforced
When `ARCHITECTURE.md` holds entries, `clm check` SHALL report each architecture entry that holds neither an `#### Enforced by` block nor an `#### Untested` block as `ARCHITECTURE.md: check/unenforced: architecture/<slug> names no enforcing rule and no reason it is untested`, in file order (`human-layer-proof`). An entry holding either passes; whether the rules it names are enough is the architect's review, not a program's.

#### Scenario: entry with neither
- GIVEN an `ARCHITECTURE.md` with entries `a`, holding an `#### Enforced by` block, `b`, holding an `#### Untested` block, and `c`, holding neither
- WHEN `clm check` runs
- THEN it reports `ARCHITECTURE.md: check/unenforced: architecture/c names no enforcing rule and no reason it is untested`, and nothing for `a` or `b`

#### Scenario: no architecture entries
- GIVEN an `ARCHITECTURE.md` with neither a `## Constraints` nor a `## Requirements` section
- WHEN `clm check` runs
- THEN it reports no `check/unenforced` violation

#### Needs
- model/human-layers

#### Serves
- architecture/deterministic-enforcement
- architecture/proven-by-tests

### Requirement: says-component
The messages `clm-check` gives SHALL say component and components, never the old words, as `component-words` defines them: those `undeclared-imports`, `unowned-files` and `untraced` quote, ``component `<id>` mentions `assets/<path>`, which isn't in <dir>/assets/`` (`asset-references`), and ``unknown component `<id>` `` for `clm pending` given an ID no component has. The rules a violation names are requirement IDs and keep their spelling. A test reads the string literals in `crates/clm-check/src/` and fails naming the file, the line and the word for each old word `component-words` doesn't keep.

#### Scenario: an asset that isn't there
- GIVEN component `a` whose Design mentions `assets/mockups/m.html`, which `specs/a/assets/` doesn't hold
- WHEN `clm check` runs
- THEN it reports a message holding ``component `a` mentions `assets/mockups/m.html` ``

#### Serves
- architecture/plain-terms

### Requirement: detector-relative-command
A detector command whose first word is a relative path holding a `/` SHALL be spawned with the repository root joined onto it, not left to the platform's lookup, so `["node_modules/.bin/knip"]` runs the root's installed tool from wherever `clm check` runs; when nothing is there it SHALL be reported as `detectors` reports a command that can't be started (`could not run`, with its hint).

#### Scenario: relative tool not installed
- GIVEN a clean repository with no `node_modules/`, whose `clm.toml` has a detector `knip` (`["node_modules/.bin/knip"]`, hint `npm ci`)
- WHEN `clm check` runs from a subdirectory
- THEN it exits 1 with a line starting `clm.toml: check/detectors: knip could not run:` and ending `; install with: npm ci`

#### Scenario: relative tool installed
- GIVEN that repository with an executable `node_modules/.bin/knip` that exits 0
- WHEN `clm check` runs from that subdirectory
- THEN no `check/detectors` line is printed

#### Serves
- capability/no-dead-weight
- architecture/no-dead-code
