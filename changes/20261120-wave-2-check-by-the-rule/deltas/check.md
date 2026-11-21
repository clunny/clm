---
component: check
base: { contract: "59c52715aaffd21d", design: "209cdd1690e753af" }
---

## MODIFIED Contract
`clm check` runs every deterministic rule and prints one actionable line per violation; it exits 1 on any violation, 0 on none, and 2 on arguments it can't parse (`report-and-exit`, `commands-in-crate`). Rules include schema, graph integrity, `paths` that two components share (`paths-disjoint`), imports between components that `depends_on` doesn't declare (`undeclared-imports`), code files in no component's `paths` (`unowned-files`), skipped tests in files that cite requirements (`skipped-tests`), names of requirements a live change removed (`removed-requirement-names`), references to component assets, stale deltas, and `clm:remove-by` markers whose change isn't live (`expand-contract`). When either human layer holds an entry, every rule must serve one (`check/untraced`, `traced-rules`). Path ownership (`unowned-files`), in a repository whose root sets `outside` (even to `[]`), means every tracked code file is in some component's `paths` or matched by the root's `outside`, the code not yet adopted, and every `outside` glob matches code no component owns (`incremental-adoption`). A root without the key isn't checked for it, so a repository opts in by setting it; `clm init` always does. The `clm check` command also runs the dead-code and unused-dependency detectors the repository's `clm.toml` names (`no-dead-code`, `dead-code-detectors`), each for at most its `timeout` (`detector-timeouts`) and none whose `requires` names a path the repository doesn't hold (`detectors`), unless it is given `--no-detectors`. A caller that knows no code changed, such as `clm land` on code already checked, passes it to skip them. After its own violations it reports those of the rules other components decide that its caller passes, such as the ledger's audit of `main` (`report-and-exit`). A caller that checks a commit's files extracted outside a git work tree, such as `clm wrap-up`, passes the commit's file listing to `run_listed`, so the rules that read the tracked files see what `clm check` sees in a checkout of that commit (`run-listed`).

A test cites the requirement it proves as `req:<id>@<version>`, where the version is the first 8 hex digits of the requirement's content hash (`model/content-hash`, `versioned-markers`). The marker says which text the test was written for. A marker counts where a test can run it: in a Rust, TypeScript, JavaScript or Python file, inside a test or a named group of tests or in the comment directly above one, and anywhere in a file of another language (`check.coverage/markers-in-tests`). A requirement is *current* when markers that count cite it, every one cites its current version and no `bug` item names it under `unproven` at that version (`mutation-after-land`); *behind* when a marker cites it at another version or without one, even beside one that cites the current version (`stale-markers-keep-behind`); *unbuilt* when no marker that counts cites it, or such an item names it; and *obsolete* when a marker cites a requirement that a change outside the archive removed (`removed-requirements-build-out`), or a capability that such a change lists under `removes_capabilities` and `CAPABILITIES.md` no longer holds (`removed-capabilities-build-out`). The last three are *pending*: the component's code hasn't caught up with its spec. A capability is cited like a rule. An architecture entry is never cited: it is current when every rule its `#### Enforced by` names is current, and an entry or capability holding `#### Untested` is never pending, so neither is in the queue (`human-layer-proof`, `check.coverage/coverage-report`). `clm pending [<component>]` lists them (`check.coverage/pending-report`), and the queue (`orchestrator`) is built on it. A change is *complete* as `check.coverage/status-report` defines it.

The markers and the states are computed by the child component `check.coverage` (crate `clm-coverage`). A component whose code reads them imports `clm-coverage`, depends on `check.coverage` and reads that Contract. `clm-check` re-exports none of that library's items (`check-coverage-split`). The crate builds for `wasm32-unknown-unknown` with its default features disabled (`check-builds-for-wasm`).

`clm size [<component>]` reports each component's size in tokens against its budget, marking the components over either (`node-size`, `node-size-budgets`). It is advisory: it exits 0 whatever the sizes and whatever violations the spec holds, 1 only for an unknown component, settings that can't be loaded or tracked files that can't be listed, and 2 on arguments it can't parse; no rule of `clm check` reads the sizes.

`clm-check` offers these calls to other crates:
- `run(root)`, every rule of `clm check` but `undeclared-imports`, the detectors and the rules its caller passes, on the checkout at `root`, and `run_listed(root, files)`, the same on a listing its caller passes (`run-listed`), each returning the violations, `model`'s `Violation`, which the crate offers again; `clm rebase` and `clm health` call `run`, and `clm wrap-up` calls `run_listed`.
- `size(root, graph, markers, defaults)`, each component's `Size` for the working tree at `root` (`node-size`): its `node`, a `spec` and a `code` `Count` of `tokens` against a `budget`, and `oversized()`, `defaults` being the `Budgets` a component's frontmatter doesn't set; `clm health`, `clm effort` and `clm view` read it. `size_of(graph, files, markers, defaults)` gives the same from a map of each file to its `Length` (`Length::of(path, text)`, or `Length::from(bytes)` for a file of one part, its test part a `TestPart`), and `linguist_excluded(attributes, path)` says whether `.gitattributes` texts mark a path generated or vendored (`size-of-listing`, `linguist-attributes`); both build for wasm, and the web app's `replica` and `view.repo` use them on a listing with no working tree.
- `pending_for(graph, markers, component)` and `pending_in(root, component)` (`pending-narrows-on-a-graph`).
- `status(root)`, `coverage(root)` and `backlog(root)`, which give, for the checkout at `root`, `check.coverage/status-report`'s report, each requirement's coverage (`check.coverage/coverage-report`) and the backlog items in `clm backlog`'s order (`backlog-report`), or the violations loading the spec reports when there are any.

`clm-check` offers each of `clm check`, `clm size`, `clm coverage` and `clm backlog` as a call (`commands-in-crate`, `commands-live-in-their-crates`), `check_command`, `size_command`, `coverage_command` and `backlog_command`, whose arguments `CheckArgs`, `SizeArgs`, `CoverageArgs` and `BacklogArgs` the `clm` binary's parser reads: each takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `check_command` also takes `CheckInputs`: a loader of the detectors the settings name for a root, each a `Detector { name, command, hint, timeout, requires }`, or the violation saying why they can't be loaded, and a source of the violations of rules other components decide. `size_command` also takes a loader of the `Budgets` a component doesn't set, or the violation saying why they can't be loaded. The `clm` binary routes each command to its call and adds nothing to what it writes.

## MODIFIED Design
`run` loads the graph (`model`), validates pending changes (`apply`), then runs the coverage, marker and text rules on the markers `check.coverage`'s `scan` finds in the files `git ls-files --cached --others --exclude-standard` lists. A marker without a version cites no version: it counts for `dangling-markers`, and the requirement it names is `behind` while it stays, so a bare marker never makes a requirement current, alone or beside a versioned one. The same scan finds removal markers: text matching the regex `clm:remove-by (\d{8}-[a-z0-9-]+)`. Text that doesn't match, such as `clm:remove-by <change-id>` in help output, isn't a marker. To mention a marker in code without claiming coverage or a removal, such as a test fixture, build the string so the literal never appears whole (e.g. `concat!("req", ":a/x")`); a fixture that holds a skipped test is built the same way.

A removal marker's change is classified from the graph (`model`), in this order: archived (it is under `changes/archive/`, whatever its status), missing (no change with that ID, including a directory without a parseable `change.md`), `done`, `abandoned`, or live (`proposed`, `approved`, `applied`, `implementing`). Its violation uses the marker's `<file>:<line>` as its path, as `dangling-markers` does (`remove-by-markers`).

**The split** (`check-coverage-split`, `nested-splits`). `check` keeps the rules `clm check` runs, the detectors and `clm size`; `check.coverage` keeps what says how far the code has caught up with the spec, which the queue, the views, `clm rebase` and the Worker read. `clm-check` depends on `clm-coverage` and re-exports none of its items, and its `checkout` feature turns on `clm-coverage`'s. `pending_in`, `status`, `coverage` and `backlog`, which read a checkout, live in `clm-check`; `pending_in`'s unknown-node violation carries the rule `check/pending-narrows-on-a-graph`, the requirement that states the refusal. `run` matches `paths` with `clm-coverage`'s `matches`, as `pending` does. The `dangling-markers` rule (`violations` in `dangling.rs`) lives in `clm-check` and takes the removed IDs from `clm-coverage`'s `removed`, as `removed-requirement-names` does. The tests in `crates/clm-check/tests` cite `check`'s requirements only; the child's tests sit in its own crate and call `clm-coverage`.

**Shared paths** (`paths-disjoint`). `run` lists the files `git ls-files --cached` lists, Markdown included, itself through `git` (outside a git work tree, none); `run_listed` reads the listing its caller passes instead (`run-listed`). A file belongs to one component, the one whose builds own it and whose size it counts toward, so `nested-splits` keeps `paths` disjoint, parents and children included. A component's `tests` carve its test files out of whatever `paths` match them (`components-own-their-tests`): the file is the `tests` component's, so a suite's directory can belong to a harness component while each feature owns its own tests in it. `paths-disjoint`, `unowned-files` and `undeclared-imports` all read a file's component as its owner, the one place that rule is written. **Exemptions** (`temporary-exemptions`): `undeclared-imports`, `tests-cite-their-owner` and `skipped-tests` drop a violation in a file whose owner's `exemptions` name that rule for it, and nothing else, reading only the owner's entries; whether an exemption is still needed is `check.health`'s report, never a violation, so the build that fixes the code still lands.

**Undeclared imports** (`undeclared-imports`, `undeclared-imports-check`, `tree-sitter-analysis`). The import edges come from `init`'s `import_edges(root)` (`init/import-edges`), the same tree-sitter extraction `clm survey` uses: Rust workspace path dependencies and `mod`/`use crate::` paths, TypeScript relative imports and workspace packages, and Python imports that resolve to files in the repository. Tree-sitter is C, which doesn't build for `wasm32-unknown-unknown`, so `run` never reports this rule; `check_command` does, after `run` and before the detectors, from the graph it loaded, and `--no-detectors` doesn't skip it, since it needs no tool. A file's component is its owner (`paths-disjoint`), test files and production files alike judged against its `depends_on`. Only a direct `depends_on` counts, and a parent or child gets no exemption, since `depends_on` is separate from ownership (`nested-splits`). `import_edges` gives no edges outside a git work tree, so a fixture that isn't a repository passes, and an error when `git` fails inside one, which the rule reports in place of the edges. `clm rebase` calls `run`, so its `rebase ok` doesn't cover this rule; `clm land` runs `clm check` and does.

**Imports before apply** (`undeclared-imports-before-apply`). A change's deltas can rewrite a component's `depends_on` and `paths`, so `undeclared_imports` also judges the same edges against the graph each `proposed` or `approved` change would leave, which `apply`'s `resulting(graph, change)` gives (`apply/resulting-graph-call`), as `removed-names-before-apply` does for names. A pair both graphs leave undeclared is reported once, by the current graph's line. Each change is judged alone against the current graph, so two pending changes that drop the same dependency each report it, naming themselves. A change whose deltas don't apply has no resulting graph: `stale-deltas` reports it. The edges are the code's as it is, read once. So a draft whose delta drops a `depends_on` entry its component's code uses meets the report in its own `clm check`, before `clm apply`, not first at `clm land`. A pair is its importing file, imported file and the IDs of `a` and `b`. A pair a change makes newly judged, by adding component `a` or `b` or by changing `paths` so a file falls in one, is reported the same way. The pending changes are those `is_pending` gives, as for `run`'s scan. The lines come after the current graph's, change by change in ID order, each in the order of the edges. The judging of resulting graphs sits behind `clm-check`'s `checkout` feature, which brings in `clm-apply` for `run`'s own before-apply scan: `undeclared_imports` is public and ungated, and without the feature it judges the current graph alone. `clm rebase` and `clm gate` don't run the rule: once the change is applied the current graph's line reports it, and `clm land` runs `clm check`.

**Skipped tests** (`skipped-tests`, `skipped-tests-in-cited-files`). A marker counts once the suite passes, so a test that never runs would prove a requirement it doesn't check. CLM reads no test runner's output, so rather than judge each marker's test, `run` reports a skipped test anywhere in a file that holds at least one `req:` marker that counts; a skipped test belongs in a file that cites nothing. A file is read by its extension. For TypeScript and JavaScript the skip patterns are `(it|test|describe)\.(skip|skipIf|todo)\b` and `\bx(it|test|describe)\(`, not preceded by a letter, digit, `_` or `.`. Markdown is never read, as for markers.

**Removed requirement names** (`removed-requirement-names`). A requirement ID is *removed* when `check.coverage`'s `removed` gives it; a change not yet applied removes nothing from `specs/` (its removals are judged on the graph it would leave, below), and a requirement added back isn't removed. `run` reads each component's `SPEC.md` below its frontmatter, reading code spans as `asset-references` does, with the reader both rules share (`spans.rs`). The path is `specs/<component path>/SPEC.md:<line>`. The rule reports only in component specs: it reads `ARCHITECTURE.md` and `CAPABILITIES.md` for their entries' slugs alone (the model loads them), and never reports in them, `CLAUDE.md`, decisions or changes, whose text names decisions and entries rather than component requirements.

**Before apply** (`removed-names-before-apply`). For each `proposed` or `approved` change, the changes `run` validates, `run` also takes the graph the change would leave from `apply`'s `resulting(graph, change)` (`apply/resulting-graph-call`) and runs the same scan on it. The removed IDs are computed from that change's deltas and the two graphs, not from `removed`, and each is attributed to that change alone: when two pending changes remove the same ID, each one's scan reports its spans, naming itself. Only IDs listed under `## REMOVED Requirements` count, as for `removed`, so a component the change removes whole adds none of its requirements. `validate` builds the same graph again; the cost is one in-memory parse of the components the change writes, so the two aren't shared. So a draft sees a name its removal leaves stale before `clm apply`, and a name the change's own deltas rewrite is never reported. The path is the component's `SPEC.md` path with the line in the text the change leaves, which is the file's own line when the change doesn't write the component. An ID the current graph doesn't hold is the applied scan's, so no span is reported by both. A change whose deltas don't apply has no resulting graph (`resulting` gives none): `stale-deltas` reports it.

**Detectors** are separate from `run`. `detectors(root, list, err)` takes a plain list of `Detector { name, command, hint, timeout, requires }` and the stream to copy their output to, and spawns each, so `clm-check` doesn't depend on `clm-config`. The spawning code sits behind the default feature `detectors`, so the crate builds for wasm with default features disabled. `check_command` asks its caller's loader for the detectors of the root it checks (the `clm` binary's loads them from `config`, each with its `timeout` and `requires`), leaves out each whose `requires` names nothing there, and calls `detectors` after `run` and `undeclared-imports`, whether or not they found violations. `clm rebase`, `clm health` and `clm wrap-up` call `run` or `run_listed` and have no project toolchain, so they never run detectors.

Each detector runs without a shell, from the repository root, one after another in the order the list gives. Its stdout and stderr are piped and copied, line by line as they arrive, to `clm check`'s stderr, one thread per stream, so stdout keeps one line per violation and a detector that hangs has already shown what it printed (`detector-timeouts`). On Unix the detector starts in a process group of its own; when its `timeout` passes before it exits, the whole group is killed (`SIGKILL`, through `libc::killpg` or an equivalent call, not a spawned `kill`), so a `sh -c` wrapper's children stop with it. Elsewhere the process alone is killed, and a grandchild holding a pipe can keep the copying threads, and `clm check`, waiting past the timeout; CLM is released for no such platform (`no-windows-support`). `clm check` then waits for the copying threads and goes on to the next detector.

**Size** (`node-size`, `oversized-is-a-state`, `tests-count-where-cited`, `inline-tests-count-where-cited`, `size-tracked-through-git`). A token is estimated as four bytes, rounded up, with no tokenizer, so the count is deterministic and cheap. A component's *spec tokens* count its `SPEC.md` (`specs/SPEC.md` for the root); its assets don't count, since a context bundle lists them rather than including them. Its *code tokens* count what a build of the component reads: among the files `clm-git`'s `tracked_files` lists, each once even mid-merge, every part of a file whose `req:` markers cite one of its requirements, wherever it sits, and every part with no markers among `clm-git`'s `tracked_matching` of its `paths`, the same list `clm survey --components` reads, so the two agree on which files a component owns (`tracked-files-in-git-layer`). `size` hands those paths back to `git check-attr --stdin -z` as their bytes (`OsStrExt::as_bytes` on Unix). A file is one part, except a Rust file (`.rs`) holding a line whose text, trimmed, is exactly `#[cfg(test)]`: from the first such line to the end of the file is its *test part*, and what comes before it is its *source part*, since Rust keeps unit tests in a module at the end of the file they test. A test file, or a test part, counts toward each component it cites and not toward the component whose `paths` merely hold it, which differ only while a project's tests don't yet sit with the components they cite (`tests-cite-their-owner`), while a Rust file's source part with no markers stays its owner's whatever its test part cites. Ignored files such as `node_modules/` and `target/` never count. Two kinds of tracked file are left out, being nothing a session reads: a binary file, one with a NUL byte in its first 8000 bytes as git decides, and a file whose `linguist-generated` or `linguist-vendored` attribute is set (`git check-attr`; `set` or `true`, not `false` or unset), the `.gitattributes` convention GitHub uses for lockfiles, built bundles and vendored code (`reuse-first`). The bytes of a count's parts are summed and rounded up once. Each count's budget is the component's `size_budget` for it (`model/size-budget`), else the default the caller passes in. A component is *oversized* when a count exceeds its budget; a `size_budget` that covers the count is the exception a human chose in a spec PR, and one the component has outgrown isn't. Nothing fails for it: `size` reports the state and `view` and `check.health` show it; nothing files an item for it (`oversized-shown-not-filed`). `size(root, graph, markers, defaults)` reads the working tree and sits behind the default feature `detectors`, so the crate builds for wasm. `size_command` asks its caller for the budgets a component doesn't set; the `clm` binary passes `config`'s `component_size`, as it passes the detectors to `check_command`.

The counting itself is `size_of(graph, files, markers, defaults)` (`size-of-listing`), built without features so the web app's `replica`, which has a Trees API listing but no working tree, counts as `clm size` does (`replica/snapshot-size`). `files` maps each tracked file that may count to its length in bytes, with binary and linguist-excluded files already left out; each component's `SPEC.md` length is read from it, and one missing from it counts 0. `size` lists and probes the working tree as above, puts each component's `SPEC.md` length in from disk (tracked or not), and calls `size_of`, so the two can't drift. `size_of` decides a file's component with `check.coverage`'s `owner`, as `pending` does. The Worker can't run `git check-attr`, so `linguist_excluded(attributes, path)` (`linguist-attributes`) decides the linguist exclusion from the texts of the tree's `.gitattributes` files, each with its directory: patterns as gitattributes reads them (relative to their file's directory, a pattern without `/` matching at any depth below it, `**`, a trailing `/` matching no file), the last matching line deciding, deeper files over shallower ones, `-attr` and `!attr` unsetting, `attr=false` not setting, and `[attr]<name>` macro lines in the root `.gitattributes` expanded where a later line names the macro (git's built-in `binary` macro sets no linguist attribute, so it changes nothing here). It reads no `.git/info/attributes` or `core.attributesFile`, which a hosted listing doesn't have; `size` keeps asking `git check-attr`, which also honours those, so a clone's local attribute files can make `clm size` differ from the Worker, and only for them. An existing crate that matches gitattributes without git, such as `gix-attributes`, is preferred to writing the matcher (`reuse-first`) if it builds for wasm and keeps the Worker inside its size and CPU limits (`web-on-free-plan`); otherwise the matcher is written here.

## MODIFIED Requirements

### Requirement: report-and-exit <!-- base: 2528ee97fc5c23e9 -->
`clm check` SHALL print one line per violation as `<path>: <rule>: <message>` and exit 1 if there are any violations, 0 otherwise. It SHALL report all violations, not just the first. Its lines SHALL come in this order: the violations of the rules `run` runs, then those of `undeclared-imports`, then the detectors' or the one its caller gives when the settings can't be loaded (`detectors`), then the violations of rules other components decide that its caller passes (`commands-in-crate`).

#### Scenario: two violations
- GIVEN a repo with a dependency cycle and a duplicate requirement slug
- WHEN `clm check` runs
- THEN both are printed and the exit code is 1

#### Scenario: clean repo
- GIVEN this repository's own `specs/`
- WHEN `clm check` runs
- THEN it exits 0

#### Scenario: lines in order
- GIVEN a git repository whose component `c` repeats a requirement slug, whose component `a` has a path dependency on component `b` that its `depends_on` doesn't list, and whose `clm.toml` has a detector `bad` (`["sh", "-c", "exit 3"]`), and a caller whose further violations are one line
- WHEN `clm-check`'s call for `clm check` runs
- THEN it exits 1 printing, one line each and in this order, the duplicate slug's line, the `check/undeclared-imports` line, `clm.toml: check/detectors: bad failed (exit 3)` and the caller's line

### Requirement: dangling-markers <!-- base: 5bc0f97fc01b25de -->
`clm check` SHALL report every `req:<id>` marker in a scanned file whose `<id>` names no existing requirement or human-layer entry (`model/human-layers`), giving the file and line; and every marker that cites an architecture entry, or an entry holding `#### Untested`, with the message `architecture entries are proven by the rules their #### Enforced by names, not by tests` or `<id> is untested`, since a test never proves either (`human-layer-proof`); unless a change outside the archive lists `<id>` under `REMOVED Requirements` in one of its deltas, or `<id>` is a capability that a change outside the archive lists under `removes_capabilities` and `CAPABILITIES.md` doesn't hold (`check.coverage/removed-capabilities`): that code is obsolete and is removed when the component is next built (`removed-requirements-build-out`, `removed-capabilities-build-out`). A marker whose `<id>` names nothing SHALL be reported as ``<file>:<line>: check/dangling-markers: marker cites `<id>`, which is not a requirement``. The message for a marker whose `<id>` is `capability/<slug>` and names no entry, with no such change, SHALL end `; a change that removes the capability lists it under removes_capabilities`, so the author of a branch that removes a cited capability learns how to land it. Once the change is archived, the marker SHALL be reported as before. An `abandoned` change is archived (`archive-done`), so it grants no exemption; a requirement another change adds back under the same ID is a new requirement once that change applies, and the marker then cites it as usual. It SHALL judge a marker outside a test (`check.coverage/test-spans`) as it judges one in a test, each marker once by its file, line and ID, so a marker that proves nothing still can't name what doesn't exist.

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
- THEN it exits 1 with the line ``tests/t.rs:3: check/dangling-markers: marker cites `a/gone`, which is not a requirement``

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

### Requirement: archive-done <!-- base: 8c5d478b6c403934 -->
`clm check` SHALL report a `done` or `abandoned` change that is not under `changes/archive/`, and an archived change in any other status. The violation's path SHALL be the change's `change.md`, and its message ``change `<id>` is <status>; run `clm archive <id>` `` for a finished change outside the archive and ``archived change `<id>` is <status>; only done or abandoned changes belong in the archive`` for an archived one, `<status>` being the word `change.md` writes for it.

#### Scenario: finished but not archived
- GIVEN `changes/x/` with `status: done`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``changes/x/change.md: check/archive-done: change `x` is done; run `clm archive x` ``

#### Scenario: archived and not finished
- GIVEN `changes/archive/y/` with `status: applied`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``changes/archive/y/change.md: check/archive-done: archived change `y` is applied; only done or abandoned changes belong in the archive``

#### Serves
- capability/catch-drift
- architecture/spec-is-current

### Requirement: backlog-report <!-- base: 392ec9fd93c8368f -->
`clm backlog` SHALL print one line per backlog item, sorted by priority (`P1` first), then by kind in the order `bug`, `deferred`, `idea`, `question`, `human-task`, `workaround`, then by slug: `<slug> <kind> <priority> [<components>] <title>`, the components joined by `, `. It SHALL exit 0, except that when loading the spec reports a violation it SHALL print each violation in place of the items, one line each as `clm check` prints one, and exit 1.

#### Scenario: two items
- GIVEN items `b` (idea, components `[sync]`) and `a` (deferred, no components), neither with a priority
- WHEN `clm backlog` runs
- THEN it prints `a deferred P5 [] <a's title>` then `b idea P5 [sync] <b's title>`

#### Scenario: priority first
- GIVEN items `a` (bug, no priority) and `b` (idea, `priority: P2`)
- WHEN `clm backlog` runs
- THEN it prints `b idea P2 [] <b's title>` then `a bug P5 [] <a's title>`

#### Scenario: kinds and components
- GIVEN components `p` and `q`, and items with no priority: `a` (workaround, flagging a requirement of `p` at its current version), `b` (human-task), `c` (question), `d` (idea, components `[p, q]`), `e` (deferred) and `f` (bug)
- WHEN `clm backlog` runs
- THEN it prints the lines of `f`, `e`, `d`, `c`, `b` and `a`, in that order, `d`'s reading `d idea P5 [p, q] <d's title>`

#### Scenario: spec that doesn't load
- GIVEN backlog item `a` and a component whose `SPEC.md` has no frontmatter
- WHEN `clm backlog` runs
- THEN it exits 1, printing that spec's violation as `<path>: <rule>: <message>` and no line for `a`

#### Serves
- capability/keep-a-backlog
- capability/see-project-state

### Requirement: asset-references <!-- base: 3f70faba43c1ec5c -->
`clm check` SHALL fail when a component's Contract or Design mentions, in a code span, a concrete path of the form `` `assets/<path>` `` that isn't one of that component's assets, naming the component and the path. A path is concrete when it contains no `<`, `>` or `*` and doesn't end in `/`. Mentions in requirements, scenarios and frontmatter never count. The violation's path SHALL be the component's `SPEC.md` and its message ``component `<id>` mentions `assets/<path>`, which isn't in <dir>/assets/``, `<dir>` being the directory that holds that `SPEC.md`, once for each component and path however many times its Contract and Design mention the path. A code span SHALL be read as CommonMark reads one within a block of lines between blank lines: a run of backticks opens a span that closes at the next run of the same length in the block, and opens none when no such run follows; outside a span a backslash escapes the one character after it, and inside a span it escapes nothing; a span that starts and ends with a space and holds something other than white space loses one space at each end; and the lines of a fenced code block hold no span, the block running from a line that starts, after any indentation, with three or more backticks or tildes (a backtick fence whose line holds no further backtick) to a line holding, after any indentation, only a fence of the same character at least as long, or to the end of the text. A span SHALL count only when its whole text starts `assets/`.

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

#### Scenario: an asset that isn't there
- GIVEN component `a` whose Design mentions `assets/mockups/m.html`, which `specs/a/assets/` doesn't hold
- WHEN `clm check` runs
- THEN it reports a message holding ``component `a` mentions `assets/mockups/m.html` ``

#### Scenario: code spans, and one line per path
- GIVEN component `a` with no assets, and in turn each of these Contracts or Designs
- WHEN `clm check` runs on each
- THEN it reports `assets/<path>` for `a` exactly as listed:
  - a block holding a lone backtick, then a block mentioning `` `assets/m.html` ``, then a block holding an escaped backtick (`` \` ``) and `` `assets/n.html` ``: `m.html` and `n.html`, one line each;
  - a span `` `C:\` `` followed by `` `assets/m.html` ``: `m.html`;
  - a backslash followed by two backticks, then ` b `, then `` `assets/x.html` ``, then ` c` and two backticks, and in another block two backslashes directly before `` `assets/y.html` ``: `y.html` only, since the first backslash escapes one backtick and the second backtick pairs with the one before `assets/x.html`, while an escaped backslash escapes no backtick;
  - `` `assets/a.html` `` inside a block fenced by four backticks that holds a line of three, and `` `assets/b.html` `` inside a block fenced by `~~~` that holds the line `~~~ not a close`, then `` `assets/m.html` `` after both: `m.html` only;
  - ``` `` assets/m.html `` ``` and `` ` assets/n.html` ``: `m.html` only;
  - `` `x/assets/m.html` ``, `` `assets` `` and `assets/m.html` without backticks: nothing;
  - a Contract mentioning `` `assets/m.html` `` and a Design mentioning it twice: `m.html`, on one line

### Requirement: detectors <!-- base: 4851be3136a1f561 -->
`clm check` SHALL run each detector in the repository's `clm.toml` from the repository root, copying its stdout and stderr to its own stderr as they arrive, and report a violation `clm.toml: check/detectors: <name> failed (exit <code>)` for each that exits non-zero. The detectors SHALL run one at a time, in `clm.toml`'s order, each finishing before the next starts. A detector still running when its `timeout` has passed SHALL be stopped, together with the processes it started, and reported as `clm.toml: check/detectors: <name> timed out after <n>s`, and `clm check` SHALL go on to the next detector. A detector that ends without an exit code before its `timeout` has passed, killed by a signal, SHALL be reported as `clm.toml: check/detectors: <name> failed (killed by a signal)`. A detector whose command can't be started SHALL be reported as `clm.toml: check/detectors: <name> could not run: <error>`, followed by `; install with: <hint>` when it has a hint; an empty command is one that can't be started, its `<error>` being `empty command`. A detector whose `requires` names a path that doesn't exist under the repository root SHALL NOT run: `clm check` SHALL write the line `<name> skipped: no <path>` to its stderr and report no `check/detectors` violation for it; one whose `requires` exists, or that has none, runs as this requirement says. `run` SHALL NOT run detectors. `clm check --no-detectors` SHALL run every other rule as `clm check` does, but run no detector and report no `check/detectors` violation, so it needs neither the detectors' tools nor loadable settings. When its caller can't load the settings, `clm check` SHALL report the violation the caller gives for the failure, run no detector, and still report every other rule's violations.

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

#### Scenario: settings that can't be loaded
- GIVEN a repository with a component spec that doesn't parse and a `clm.toml` whose detector `bad` exits 3, and a caller whose loading of the settings fails with a violation
- WHEN `clm-check`'s call for `clm check` runs
- THEN it exits 1 and prints that violation and the spec's, one line each, and no `bad failed` line

#### Scenario: detectors that end oddly
- GIVEN in turn each of these lists of detectors in a clean repository
- WHEN `clm check` runs
- THEN it reports exactly these `check/detectors` lines:
  - `first` (`["sh", "-c", "sleep 0.2; touch done; exit 1"]`) then `second` (`["test", "-f", "done"]`): `clm.toml: check/detectors: first failed (exit 1)` alone, since `second` starts after `first` has written `done`;
  - `killed` (`["sh", "-c", "kill -9 $$"]`): `clm.toml: check/detectors: killed failed (killed by a signal)`;
  - `empty`, whose command is `[]`: a line starting `clm.toml: check/detectors: empty could not run: empty command`

#### Scenario: a detector whose required path is missing
- GIVEN a clean repository with no `package.json` whose `clm.toml` has a detector `knip` (`["sh", "-c", "touch ran; exit 3"]`, `requires = "package.json"`)
- WHEN `clm check` runs
- THEN it exits 0, stdout is empty, stderr is exactly the line `knip skipped: no package.json`, and no file `ran` exists; with a file `package.json` in the repository, it exits 1 with the line `clm.toml: check/detectors: knip failed (exit 3)` and `ran` exists

#### Serves
- capability/no-dead-weight
- architecture/no-dead-code

#### Needs
- config/detector-timeout-setting

### Requirement: remove-by-markers <!-- base: 4ca98d5ac0371fa2 -->
`clm check` SHALL report every `clm:remove-by <change-id>` marker in a scanned file whose change is not live, as `<file>:<line>: check/remove-by-markers: marker names <change-id>, which <why>`, `<why>` being `is archived` for a change under `changes/archive/`, whatever its status, `doesn't exist` for an ID no change has, a directory without a `change.md` that loads included, `is done` for a `done` change and `is abandoned` for an `abandoned` one. A change is live when it is not archived and its status is `proposed`, `approved`, `applied` or `implementing`.

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

### Requirement: priority-caps <!-- base: 7f8bcea13975f8b5 -->
`clm check` SHALL report a `check/priority-caps` violation for each priority that has more holders than its cap: at most 1 `P1`, 2 `P2`, 3 `P3` and 4 `P4`, and any number of `P5` (`priority-caps`). Backlog items and changes share the caps. The holders are:
- each change outside the archive whose status isn't `abandoned`, at its priority;
- each backlog item at its priority, unless a change that holds a priority names it in `resolves`. That change carries the work, so it isn't counted twice. An item promoted with `from` is deleted in the same spec PR, so its priority moves to the change.

The violation's path SHALL be the first holder's file, `backlog/<slug>.md` or `changes/<id>/change.md`, and its message `<n> holders of <priority>, cap <cap>: <holders>; demote some`, `<n>` being how many there are and the holders written `changes/<id>` or `backlog/<slug>`, sorted as text and joined by `, `, so the author can see what to demote. A demotion edits an existing item or `change.md`, so it is a spec change like any other.

#### Scenario: within the caps
- GIVEN change `x` with `priority: P1`, backlog items `a` and `b` with `priority: P2`, and items with no priority
- WHEN `clm check` runs
- THEN it reports no `check/priority-caps` violation

#### Scenario: two P1s
- GIVEN change `x` with `priority: P1` and backlog item `a` with `priority: P1`
- WHEN `clm check` runs
- THEN it exits 1 with the line `backlog/a.md: check/priority-caps: 2 holders of P1, cap 1: backlog/a, changes/x; demote some`

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

### Requirement: node-size <!-- base: a70a52007df7aeb8 -->
`clm size` SHALL print one line per component, sorted by ID, and `clm size <component>` only that component's line: `<component>`, `spec <tokens>/<budget>` and `code <tokens>/<budget>`, tab-separated, followed by a tab and `oversized` when the component is oversized, and SHALL exit 0. A component is *oversized* when either count exceeds a budget its frontmatter doesn't set for that count, or exceeds a `size_budget` it does set (`oversized-is-a-state`). A count's tokens SHALL be the total bytes of the parts it counts divided by 4, rounded up once. Spec tokens SHALL count the component's `SPEC.md`. Code tokens SHALL count, among the files `git ls-files --cached` lists, except a file with a NUL byte in its first 8000 bytes or whose `linguist-generated` or `linguist-vendored` attribute is set or `true` (any other value, `false` and unset don't exclude it): every part holding a `req:` marker that cites one of the component's requirements, wherever it sits, and every part holding no `req:` marker of a file the component owns (`check.coverage/owner`, so a file its `tests` match among them) (`tests-count-where-cited`). A file SHALL be one part, except a `.rs` file holding a line whose text, trimmed, is exactly `#[cfg(test)]`, which SHALL be two: the bytes from the first such line to the end of the file, and the bytes before it (`inline-tests-count-where-cited`). A part with markers SHALL NOT count toward a component its markers don't cite, even when that component's `paths` hold its file. A marker's component is the part of its ID before the `/`, as `check.coverage/pending-report` reads it, so a marker citing a requirement the component no longer holds still counts toward that component, and a part whose markers name only components that don't exist counts toward none. Each budget SHALL be the component's `size_budget` value for it when set, else the default passed in. `size(root, graph, markers, defaults)` SHALL return each component's counts, budgets and whether it is oversized, for the callers that show sizes, such as `clm health` and `clm view`. `clm size <component>` SHALL print ``specs/<component path>/SPEC.md: check/node-size: unknown component `<component>` ``, `<component path>` being the ID with each `.` written `/`, and exit 1 when no component has that ID. When its caller can't load the settings, `clm size` SHALL print the violation the caller gives for the failure and nothing else, whatever component it is given, and exit 1. When the files the index holds can't be listed, as in a directory in no git repository, it SHALL print `<root>: check/node-size: <error>`, `<root>` being the root it reads as it was given, and exit 1. Violations in the spec SHALL NOT stop it: it prints the lines of the components that load and exits 0.

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
- THEN it exits 1 printing ``specs/zz/SPEC.md: check/node-size: unknown component `zz` ``

#### Scenario: settings can't be loaded
- GIVEN a repository `clm.toml` with `[component_size]` and `spec_tokens = 0`
- WHEN `clm size` runs, and `clm size zz` with no component `zz`
- THEN each exits 1 printing one line, which names `clm.toml` and `component_size`

#### Scenario: spec violations
- GIVEN component `a` as in `over its spec budget`, and component `b` whose `depends_on` names a component that doesn't exist
- WHEN `clm size a` runs
- THEN it prints `a`'s line as in `over its spec budget` and exits 0

#### Scenario: no work tree
- GIVEN a directory in no git repository holding component `a` with paths `["src/a/**"]` and a file `src/a/x.rs`
- WHEN `clm size --root <that directory>` runs
- THEN it exits 1 printing one line, which starts `<that directory>: check/node-size: `

#### Serves
- capability/know-health
- architecture/slim-components

#### Needs
- check.coverage/owner
- model/size-budget
- config/component-size-setting

### Requirement: undeclared-imports <!-- base: 5d13552f836c110b -->
`clm check` SHALL report each import edge `init/import-edges` gives for the repository from a file of component `a` to a file of component `b`, where `b` is not `a` and `a`'s `depends_on` doesn't list `b`, as `<importing file>: check/undeclared-imports: imports <imported file> of component `b`, and `a` doesn't depend on `b``, where a file's component is its owner (`paths-disjoint`); an edge with an end no component owns SHALL NOT be reported. A file `a`'s `tests` match SHALL be judged against `a`'s `depends_on` as any of its files is. An edge from a file that an exemption of `a` names for `undeclared-imports` (`model/exemptions`) SHALL NOT be reported, judged by the exemptions of the graph being judged, the current one or the one a change would leave. `clm check --no-detectors` SHALL report it as well, and `run` SHALL NOT. It SHALL also judge the same edges against the graph each `proposed` or `approved` change would leave (`apply/resulting-graph-call`), reporting each edge and component pair that graph leaves undeclared and the current graph doesn't as `<importing file>: check/undeclared-imports: imports <imported file> of component `b`, and `a` wouldn't depend on `b` after <change>`; a change whose deltas don't apply SHALL be skipped. When the import edges can't be read, `init/import-edges` giving an error, `clm check` SHALL report `<root>: check/undeclared-imports: <error>`, `<root>` being the root it checks as it was given, and go on with the detectors.

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

#### Scenario: another component's exemption
- GIVEN the same components, and `app`, not `feat`, with `exemptions: [{rule: undeclared-imports, files: ["tests/feat/**"], until: backlog/x}]`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``tests/feat/test_x.py: check/undeclared-imports: imports app/main.py of component `app`, and `feat` doesn't depend on `app` ``

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

#### Scenario: edges that can't be read
- GIVEN the declared workspace in a git work tree whose `.git/index` holds bytes that aren't an index, and a `clm.toml` with a detector `bad` (`["sh", "-c", "exit 3"]`)
- WHEN `clm check --root <its root>` runs
- THEN it exits 1 printing a line that starts `<its root>: check/undeclared-imports: ` and the line `clm.toml: check/detectors: bad failed (exit 3)`

#### Serves
- capability/catch-drift
- architecture/ancestor-scoped-context

#### Needs
- init/import-edges
- model/exemptions

### Requirement: skipped-tests <!-- base: 347f43dcc82df907 -->
`clm check` SHALL report, in each scanned file that holds a `req:` marker, each line that skips a test: in a `.rs` file a line starting `#[ignore` after leading whitespace; in a `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs` or `.cjs` file a call of `it`, `test` or `describe` with `.skip`, `.skipIf` or `.todo`, or of `xit`, `xtest` or `xdescribe`; and in a `.py` file a line starting `@pytest.mark.skip` or `@unittest.skip` after leading whitespace, or holding `pytest.skip(`; as `<file>:<line>: check/skipped-tests: a skipped test in a file that cites requirements; run it, or move it to a file that cites none`. A file without markers, a Markdown file, and a file that an exemption of its owner names for `skipped-tests` (`model/exemptions`) SHALL NOT be reported. A file whose every `req:` marker stands outside a test (`check.coverage/test-spans`) SHALL NOT be reported either, since such a marker proves nothing.

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

#### Scenario: a marker outside a test
- GIVEN `src/lib.rs` whose only marker cites `a/x` in the body of a function with no test attribute, and which holds `#[ignore]` above a test on another line
- WHEN `clm check` runs
- THEN it reports no `check/skipped-tests` violation

#### Serves
- capability/prove-by-tests
- architecture/proven-by-tests

#### Needs
- model/exemptions

### Requirement: removed-requirement-names <!-- base: 2a727e8ae7eddbd7 -->
`clm check` SHALL report each code span, read as `asset-references` reads one, in a component's `SPEC.md`, below its frontmatter, whose whole text is a removed requirement ID `<component>/<slug>`, in any component's `SPEC.md`, or the bare `<slug>` of one, in the `SPEC.md` of `<component>` itself unless a requirement of any component has that slug, `docs/decisions/<slug>.md` exists or a human-layer entry has that slug; as `<spec file>:<line>: check/removed-requirement-names: names `<id>`, which <change> removes`. An ID is *removed* when a change outside the archive lists the slug under `REMOVED Requirements` in its delta for the component and no component holds the ID, or lists a capability's slug under `removes_capabilities` and `CAPABILITIES.md` doesn't hold it, the ID being `capability/<slug>` and its component, for the bare slug, the graph's component `capability` (`check.coverage/removed-capabilities`); with several such changes, the message SHALL name the first by ID. It SHALL also report the same spans in the graph each `proposed` or `approved` change would leave (`apply/resulting-graph-call`), for the IDs that change's deltas list under `REMOVED Requirements` and the current graph holds, with shared slugs judged on that graph, as `<spec file>:<line>: check/removed-requirement-names: names `<id>`, which <change> would remove`, the line being that of the text the change leaves; a change whose deltas don't apply is skipped.

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

#### Scenario: removed capability named
- GIVEN a `CAPABILITIES.md` with no entry `gone`, a `proposed` change `20260101-x` with `removes_capabilities: [gone]`, and component `b` whose Design names `` `capability/gone` `` on line 12 of `specs/b/SPEC.md`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``specs/b/SPEC.md:12: check/removed-requirement-names: names `capability/gone`, which 20260101-x removes``

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
- check.coverage/removed-capabilities

### Requirement: size-of-listing <!-- base: 38e607b536b362de -->
`clm_check::size_of(graph, files, markers, defaults)` SHALL return each component's counts, budgets and oversized state by the rules `node-size` states, from `files`, a map from each tracked file that may count to its `Length`, rather than from a working tree: its bytes and, for a `.rs` file `node-size` counts as two parts, its test part's first line and bytes (`Length::of(path, text)` splits a file's text; `Length::from(bytes)` is a file of one part, which is all a listing without contents can give), and SHALL build for `wasm32-unknown-unknown` with the crate's default features off. `size` SHALL return what `size_of` returns given each file of the working tree that `node-size` and `size-tracked-through-git` let count, with its length, and each component's `SPEC.md` as it is on disk.

#### Scenario: same as the working tree
- GIVEN a repository with component `a` whose `SPEC.md` is 4001 bytes and whose frontmatter sets `size_budget: { spec_tokens: 1000 }`, so it is oversized, component `b` with defaults and a 100-byte file under its `paths`, a 400-byte file under `a`'s `paths`, a test file elsewhere citing `a`'s requirement, a binary file under `a`'s `paths` and a lockfile marked `linguist-generated`
- WHEN `size_of` is called with the tracked files' lengths, leaving out the binary file and the lockfile, and `size` is called on the working tree
- THEN both return the same counts, budgets and oversized states for every component: `a` oversized with spec `1001/1000`, `b` not

#### Serves
- capability/follow-from-browser
- capability/know-health
- architecture/slim-components

### Requirement: unowned-files <!-- base: 788ce525c480ee3d -->
When the root sets `outside` (`model/outside-globs`), even to `[]`, `run` SHALL check path ownership; when the root has no `outside` key it SHALL NOT, and reports no `check/unowned-files` violation. It SHALL read the files the index holds under the CLM repository (`git ls-files --cached`), every entry the index lists, symlinks and submodules included; in a directory that isn't a git work tree it SHALL read no file and report no `check/unowned-files` violation; when the index's files can't be listed inside a git work tree, it SHALL report `<root>: check/unowned-files: <error>`, `<root>` being the root it was given, and judge no file; `run_listed` (`run-listed`) SHALL read the files of the listing it is given instead, wherever the directory is. It SHALL leave out the spec paths: every file under `specs/`, `changes/`, `backlog/` or `docs/`, and `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `AGENTS.md`, the same spec paths `gate` names. Of the rest, the *code files*, it SHALL report each that no component owns (`paths-disjoint`), by its `paths` or its `tests`, whatever the component's status, and no glob of the root's `outside` matches, as `<file>: check/unowned-files: in no component's paths; add it to a component's paths, or to outside in specs/SPEC.md until a change adopts it`. It SHALL report each `outside` glob that matches no code file that no component owns, as `specs/SPEC.md: check/unowned-files: outside glob <glob> matches no file outside every component`. A glob matches a file as the git pathspec `:(glob)<glob>` does. A code file that a component owns and an `outside` glob matches is the component's, and SHALL NOT be reported. A file the index doesn't hold, untracked or ignored, SHALL NOT be read or reported.

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

#### Scenario: listing that fails
- GIVEN a git work tree whose root sets `outside: []`, holding a tracked `crates/b/lib.rs` that no component owns, and whose `.git/index` then holds bytes that aren't an index
- WHEN `run` is called on it
- THEN it reports one `check/unowned-files` violation, whose line starts `<the root it was given>: check/unowned-files: `

#### Serves
- capability/adopt-clm
- architecture/incremental-adoption

#### Needs
- model/outside-globs
- model/test-ownership-keys

### Requirement: size-tracked-through-git <!-- base: 551f5d199fd12732 -->
`clm size` and `size` SHALL count code tokens only from the files the index holds, as `vcs/tracked-files` lists them: each once, even while it is in conflict, and a file whose name isn't UTF-8 among them; and of those only a regular file of the working tree, so a tracked symbolic link, a tracked path that is a directory in the working tree and a tracked file missing from the working tree SHALL NOT count. A file the index doesn't hold SHALL NOT count, even one whose markers cite a component.

#### Scenario: only tracked regular files
- GIVEN components `a` (paths `["src/a/**"]`) and `b` (paths `["src/b/**"]`), and a tracked `src/a/x.rs` of 400 bytes with no marker
- WHEN `clm size` runs with, in turn, each of the following
- THEN `a`'s code count is `code 100/1000` each time, and with the untracked file `b`'s is `code 0/1000`:
  - a tracked symbolic link `src/a/link` to a tracked 4000-byte file outside both components' `paths`;
  - a tracked path `src/a/sub` that is a directory in the working tree;
  - a tracked `src/a/deleted.rs` of 4000 bytes removed from the working tree;
  - `src/a/x.rs` held in the index at stages 1, 2 and 3 of a conflict in place of stage 0;
  - an untracked `src/a/t.rs` of 800 bytes whose marker, directly above a test, cites `b/x`;
  - and, in place of `src/a/x.rs`, a tracked 400-byte file under `src/a/` whose name holds the byte `0xE9`, which isn't UTF-8

#### Serves
- architecture/reuse-first
- architecture/no-dead-code

#### Needs
- vcs/tracked-files

### Requirement: pending-narrows-on-a-graph <!-- base: 201e1738fb85a49d -->
`clm-check` SHALL export `pending_for(graph, markers, component)`, which returns the pending list `check.coverage/pending-report` states for that graph and those markers, narrowed to `component`'s entries when one is given, or, when `component` names no component of `graph`, one a live change removed included, a violation of the rule `check/pending-narrows-on-a-graph`, this requirement, at that component's `SPEC.md` path whose message is ``unknown component `<component>` ``, the same violation `pending_in` returns for it; `pending_in(root, component)` SHALL load the graph and markers from the working tree and return what `pending_for` returns for them, holding no narrowing of its own, so a caller that loads a graph and markers elsewhere, such as from a commit's tree (`view.repo/pending-names-its-commit`), narrows them the same way.

#### Scenario: same as the working tree
- GIVEN a repository with components `a` and `b`, each with an uncited requirement
- WHEN `pending_in(root, Some("a"))`, `pending_for` on the graph and markers loaded from that root with `Some("a")`, and both with `Some("nope")` run
- THEN the first two return the same list, holding `a`'s requirement and not `b`'s, and the last two return the same violation naming `nope`

#### Scenario: a removed component is unknown
- GIVEN no component `g`, an `applied` change whose delta for `g` has `remove: true` and lists `g/x` under `REMOVED Requirements`, and a test citing `g/x` at `t.rs:1`, a file no component's `paths` match
- WHEN `pending_in(root, Some("g"))`, `pending_in(root, Some("nope"))` and `pending_in(root, Some("clm"))` run
- THEN the first two return a violation, each of the rule `check/pending-narrows-on-a-graph`, and the third lists `g/x obsolete t.rs:1`

#### Serves
- capability/see-project-state

### Requirement: tests-cite-their-owner <!-- base: 97a93dfaf2e76719 -->
`clm check` SHALL report each `req:` marker, in a file of its scan that a component's `tests` match, or, once the root sets `wiring` (`model/wiring-globs`), in any file of its scan that a component owns, that cites a requirement of a component other than that file's owner (`paths-disjoint`), as `<file>:<line>: check/tests-cite-their-owner: cites <requirement> of `b`, but the file is `a`'s; move the test to `b`'s files`, and `run` SHALL report it too. A marker citing a capability (`capability/<slug>`) SHALL be read as citing the `capability` component's requirement, which proves it (`human-layer-proof`). While the root sets no `wiring`, a file that only `paths` match SHALL NOT be judged, so a project gives its tests to their components one `tests` glob at a time (`components-own-their-tests`) before it names its `wiring`; once it does, a test of a component's requirement sits in that component's own files, the root's `wiring` files included though every build may edit them: `clm land` refuses a build's edit to another component's file (`gate/own-files`), so a marker there could never be re-cited by the build of the component it cites, and a test in a file every build shares is no component's to keep. A file no component owns SHALL NOT be judged, and a marker that names no existing requirement is left to `dangling-markers`. A marker outside a test (`check.coverage/test-spans`), and one citing an architecture entry, SHALL NOT be judged. A marker in a file that an exemption of its owner names for `tests-cite-their-owner` (`model/exemptions`) SHALL NOT be reported.

#### Scenario: another component's requirement
- GIVEN components `a` with tests `["tests/a/**"]` and `b` with requirement `y`, and `tests/a/t.rs` whose line 4 cites `b/y`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``tests/a/t.rs:4: check/tests-cite-their-owner: cites b/y of `b`, but the file is `a`'s; move the test to `b`'s files``

#### Scenario: its own requirement
- GIVEN the same components, and `tests/a/t.rs` citing only `a`'s requirements
- WHEN `clm check` runs
- THEN it reports no `check/tests-cite-their-owner` violation

#### Scenario: a file only paths own
- GIVEN a root that sets no `wiring`, component `a` with paths `["tests/**"]` and no `tests`, and `tests/t.rs` citing `b/y`
- WHEN `clm check` runs
- THEN it reports no `check/tests-cite-their-owner` violation

#### Scenario: a file only paths own, once the root names its wiring
- GIVEN the same components and file, and a root with `wiring: []`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``tests/t.rs:<line>: check/tests-cite-their-owner: cites b/y of `b`, but the file is `a`'s; move the test to `b`'s files``

#### Scenario: a wiring file
- GIVEN a root with `wiring: ["src/main.rs"]` that owns `src/main.rs`, whose test cites `b/y`
- WHEN `clm check` runs
- THEN it exits 1 with a `check/tests-cite-their-owner` line naming `src/main.rs` and `b/y`

#### Scenario: a capability test under wiring
- GIVEN a root with `wiring: []`, a component `capability` owning `tests/capability/see.rs`, which cites `capability/see`, and no other marker
- WHEN `clm check` runs
- THEN it reports no `check/tests-cite-their-owner` violation

#### Scenario: exempt until split
- GIVEN the file of `another component's requirement`, and `a` with an exemption from `tests-cite-their-owner` for `tests/a/t.rs` until `backlog/x`
- WHEN `clm check` runs
- THEN it reports no `check/tests-cite-their-owner` violation

#### Scenario: markers it doesn't judge
- GIVEN a root with `wiring: []`, component `a` with paths `["crates/a/**"]`, component `b` with requirement `y`, an `ARCHITECTURE.md` with entry `git-is-truth`, and `crates/a/x.rs` holding a marker citing `b/y` in the body of a function with no test attribute and a test citing `architecture/git-is-truth`
- WHEN `clm check` runs
- THEN it reports no `check/tests-cite-their-owner` violation

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

#### Needs
- model/test-ownership-keys
- model/exemptions
- model/wiring-globs
- land/approval-policy
- land/approval-from-base
- land/changes-requested-holds
- land/signed-land
- land.approval/request-refusal

### Requirement: coverage-lists <!-- base: 963db7401661e1a2 -->
After the obsolete lines `clm coverage` SHALL print one line `<id> uncounted <file>:<line>[ <file>:<line>...]` for each requirement or capability `check.coverage/test-lists` gives as cited by markers outside every test in the checkout, with those markers, so a build finds the markers to move. `clm coverage --obsolete-tests` SHALL print, in place of the coverage, the line `# tests to delete: <n>; tests that only lose a marker: <m>` and then one line per test `check.coverage/test-lists` gives as holding a marker of a removed requirement or capability in the checkout, in its order: `DELETE <file>::<name>`, a tab and `cites only removed: <id>[, <id>...]` when every marker on the test's lines cites a removed requirement, and otherwise `UNCITE <file>::<name>`, a tab and `loses: <id>[, <id>...]`. `clm coverage --deleted-tests <base>` SHALL print `# tests deleted since <base>: <n>` and then one line `DELETED <file>::<name>`, a tab and the IDs it cited, per test `check.coverage/test-lists` counts as gone from the files of the revision `<base>` against the working tree's, the revision's files being those under the CLM root and both sides' named from the CLM root, as the working tree's are and as `clm land` names them (`land/deleted-tests-approved`), so the two give one verdict in a repository whose CLM root is a subdirectory of its git repository too. The working tree's files that `--obsolete-tests` and `--deleted-tests` read SHALL be those `check.coverage/markers-from-files` states `scan` reads, where `git ls-files` fails included, so each list reads the files the coverage reads on the same tree. Each SHALL exit 0; `--deleted-tests` SHALL exit 1 naming the revision when `<base>` resolves to no commit; the two flags together SHALL exit 2 as a usage error; and a graph that doesn't load SHALL fail as `clm coverage` fails.

#### Scenario: an uncounted marker
- GIVEN `a/x`, and `src/lib.rs` holding `req:a/x` at its current version in the body of a function with no test attribute, cited nowhere else
- WHEN `clm coverage` runs
- THEN it prints `a/x unbuilt`, and after the obsolete lines `a/x uncounted src/lib.rs:<line>`

#### Scenario: a removed requirement's tests
- GIVEN an `applied` change that removes `a/x`, a test `both` citing `a/x` and `a/y` and after it a test `only_x` citing `a/x` alone, both in `tests/t.rs`
- WHEN `clm coverage --obsolete-tests` runs
- THEN it prints `# tests to delete: 1; tests that only lose a marker: 1`, then `UNCITE tests/t.rs::both` with `loses: a/x`, then `DELETE tests/t.rs::only_x` with `cites only removed: a/x`; and in a directory with no `.git` holding the same files with `tests/t.rs` moved to `.ci/t.rs`, it prints the same lines naming `.ci/t.rs`

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

### Requirement: commands-in-crate <!-- base: ed866c99a156d930 -->
`clm-check` SHALL offer each of `clm check`, `clm size`, `clm coverage` and `clm backlog` as a call of its own that takes the command's arguments as the command line gives them, the repository root its caller found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2, and for `--help` it SHALL write to the output stream exactly the help the `clm` binary prints for that command's `--help`, and return 0. The call SHALL take what the command reads of the settings and of other components' results as further arguments: for `clm check`, a loader of the detectors the settings name for the root it checks, which gives the violation saying why when they can't be loaded, and a source of the violations of rules other components decide for that root, which it reports after its own (`report-and-exit`); for `clm size`, a loader of the budgets a component doesn't set, which gives the violation saying why when they can't be loaded. What `clm coverage` prints of each requirement's state is stated by `check.coverage/coverage-report`: the call SHALL write those lines for the repository unchanged (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository
- WHEN `clm-check`'s call for `clm check` is given its arguments and two buffers
- THEN the buffers hold what `clm check` prints to standard output and to standard error there, and the call returns the status `clm check` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN it is given an option `clm check` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the calling process goes on

#### Scenario: lines from the crate below
- GIVEN a fixture repository
- WHEN the call is given the arguments of `clm coverage`
- THEN the output buffer holds exactly the lines `check.coverage/coverage-report` states for that repository, and the call returns 0

#### Scenario: help
- GIVEN the calls for `clm check`, `clm size`, `clm coverage` and `clm backlog`
- WHEN each is given `--help`
- THEN each returns 0 with, in the output buffer, exactly what `clm <command> --help` prints on standard output, `Usage: clm <command>` among it, and nothing in the error buffer

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: obsolete-tests-elsewhere <!-- base: ad5010b7502ad63f -->
Once the root sets `wiring` (`model/wiring-globs`), `clm check` SHALL report each test (`check.coverage/test-spans`, groups left out) in a file of its scan that a component owns (`check.coverage/owner`) and no `wiring` glob matches, whose every marker cites a requirement that a change outside the archive removed, or a removed capability (`check.coverage/removed-capabilities`), whose ID names a component other than the file's owner, whether or not the spec graph still holds that component, a capability's ID naming the component `model/capability-tests-component` gives, when no approved list names the test, as ``<file>:<line>: check/obsolete-tests-elsewhere: test <name> cites <requirement>, which <change> removed, from `c`'s file; list it in that change's deleted-tests.txt, and prove what stays in the files of the component that states it``, `<line>` the test's first line as `test-spans` gives it, `<requirement>` the first such ID, `<change>` the first of its removing changes and `c` the file's owner; `run` SHALL report it too. An approved list here is a `changes/<id>/deleted-tests.txt` or `changes/archive/<id>/deleted-tests.txt` among the files `clm check` reads in the tree it checks, and it names a test by a list line (`gate/cuts`) whose file is the test's and whose name agrees with the test's once every `req:` marker, with the space before it, is left out of both, as `check.coverage/test-lists` compares names. It SHALL NOT report a test one of whose removed requirements a move (`check.coverage/move-targets`) takes to a requirement of the file's owner, which that owner's build re-cites; a test that also holds any other marker, citing a requirement the spec holds, a requirement of the owner's own that was removed, a capability `CAPABILITIES.md` holds, or a removed capability when the owner builds the capability tests, whose markers for the other components' removed requirements and capabilities alone go; a marker outside a test; or a file no component owns. No exemption excuses it, and while the root sets no `wiring` it SHALL report nothing.

#### Scenario: a test nobody listed
- GIVEN a root with `wiring: []`, components `b` and `c` with `paths` `["crates/b/**"]` and `["crates/c/**"]`, an `applied` change `20260101-x` that removed `b/gone`, and a test `old_rule` whose first line is line 8 of `crates/c/t.rs`, citing `b/gone` alone
- WHEN `clm check` runs
- THEN it exits 1 with the line ``crates/c/t.rs:8: check/obsolete-tests-elsewhere: test old_rule cites b/gone, which 20260101-x removed, from `c`'s file; list it in that change's deleted-tests.txt, and prove what stays in the files of the component that states it``

#### Scenario: a listed test
- GIVEN the same, with `changes/20260101-x/deleted-tests.txt` holding `DELETE crates/c/t.rs::old_rule`
- WHEN `clm check` runs
- THEN it reports no `check/obsolete-tests-elsewhere` violation

#### Scenario: a removed component's test
- GIVEN a root with `wiring: []`, no component `g`, an `applied` change `20260101-x` that removed `g/x`, and a test in `crates/c/t.rs` citing `g/x` alone
- WHEN `clm check` runs
- THEN it reports that test under `check/obsolete-tests-elsewhere`

#### Scenario: moved into the file's owner
- GIVEN the same, the change adding `c/x`, the move of `g/x`
- WHEN `clm check` runs
- THEN it reports no `check/obsolete-tests-elsewhere` violation

#### Scenario: its own component's file, and a test that cites a live requirement too
- GIVEN the first scenario's repository with a test citing `b/gone` in `crates/b/t.rs`, and the test in `crates/c/t.rs` also citing `c`'s requirement `c/y`
- WHEN `clm check` runs
- THEN it reports no `check/obsolete-tests-elsewhere` violation

#### Scenario: a removed capability's test
- GIVEN a root with `wiring: []`, components `capability` and `c` with `paths` `["crates/clm/tests/capability/**"]` and `["crates/c/**"]`, a `CAPABILITIES.md` with no entry `gone`, an `applied` change `20260101-x` with `removes_capabilities: [gone]`, a test `old_cap` whose first line is line 8 of `crates/c/t.rs`, citing `capability/gone` alone, and a test citing `capability/gone` in `crates/clm/tests/capability/gone.rs`
- WHEN `clm check` runs
- THEN it exits 1 with the line ``crates/c/t.rs:8: check/obsolete-tests-elsewhere: test old_cap cites capability/gone, which 20260101-x removed, from `c`'s file; list it in that change's deleted-tests.txt, and prove what stays in the files of the component that states it``, and reports nothing for `crates/clm/tests/capability/gone.rs`

#### Scenario: a wiring file and an unowned file
- GIVEN the first scenario's repository with `wiring: ["crates/c/shared.rs"]`, the test in `crates/c/shared.rs`, and another test citing `b/gone` in `old/t.rs`, which no component owns
- WHEN `clm check` runs
- THEN it reports no `check/obsolete-tests-elsewhere` violation

#### Scenario: not switched on
- GIVEN the first scenario's repository with a root that sets no `wiring`
- WHEN `clm check` runs
- THEN it reports no `check/obsolete-tests-elsewhere` violation

#### Serves
- architecture/removal-needs-a-yes
- architecture/path-ownership

#### Needs
- model/capability-tests-component
- gate/cuts
- check.coverage/move-targets

## REMOVED Requirements

### Requirement: git-through-layer <!-- base: 3909a13319f272f7 -->

### Requirement: says-component <!-- base: 8d7d2aadc18b0876 -->
