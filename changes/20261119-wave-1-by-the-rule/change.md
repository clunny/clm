---
status: applied
---

## Intent

Wave 1 of phase 5 of `backlog/a-passing-check-proves-less-than-the-spec-claims`: `vcs`, `model`, `config`, `context`, `apply`, `rename`, `gate.fingerprint`, `agent-harness` and `machine` are each brought, in one pass and one delta, to the rule for a requirement (`requirements-state-what-the-contract-shows`). `vcs/one-git-layer` becomes one check of every crate, so the per-component copies of `git-through-layer` can go in their own waves. The human agreed the cuts on 2026-11-19.

## Problem

Read against the rule and against the code on 2026-11-19, by one reader per component and then against the code by the session:

- **Every Contract was wrong or incomplete.** `vcs` left out `files_to_commit`, `FetchError` and the variants of `Error` that about 30 callers match; `model` about 20 calls other crates use; `apply` five; `context` four, and it said the web Worker uses the bundle, which nothing in the Worker does; `config` three; `agent-harness` the stderr `render` returns and the marker's name; `machine` the three calls the root makes.
- **Requirements contradicted their own tests.** `apply/stale-base` read as accepting a base of 8 digits, which its test refuses; `rename/marker-rewrite`'s scenario "current and behind markers" said `clm pending` lists nothing where its test asserts one line; `rename/refusals` refused for any live change where a test asserts an `applied` one doesn't; `gate.fingerprint/fingerprint` said `clm land` calls `adds_change`, which nothing outside the crate calls; `model/graph-check` listed the checks the graph check runs and left out two; `model`'s Contract called `capability` a refused ID.
- **Behaviour stated only by a Design, or by nothing.** About 60 clauses the code does and tests mostly assert had no requirement: what `clm apply` prints, the refusal of `clm hashes` for an unknown component, `clm context`'s streams, `machine`'s JSON keys and its disk rule, which two requirements pointed at "as the Design says", Codex's hook formats, which the Design deferred to its documentation and a workaround flag called unverified.
- **`vcs/one-git-layer` read only the crates that list `clm-git`**, by two literal calls, so 15 components each held a copy of the rule for their own crate, and two places no copy read ran `git` themselves: the root's build script and `clm-upgrade`'s unit tests.
- **Sentences about tests, source text and code layout**: the four `commands-in-crate` placement sentences, three scans of a crate's string literals for retired words, two `clm:remove-by` sentences, and clauses naming private functions.

## Approaches

- **Taken: one whole check in `vcs`, with the crates that start other programs named.** Every crate under `crates/`, its `src/` and its build script, whether or not it lists `clm-git`. Reading only literal `git` and `gh` calls would miss a wrapper that takes the program as an argument, which eight copies caught by refusing any process; so the check refuses any process outside six named crates. The list is in the requirement: a new crate that starts a program is a spec change, which is the review that rule wants.
- **Taken: route the two offenders through the layer, not an exception.** The build script's fix landed first (`1d2936a7`). `clm-upgrade`'s needs `upgrade` to depend on `vcs`, which this change declares; the fix is `backlog/upgrade-s-unit-tests-start-git-themselves`, which blocks `vcs/one-git-layer`.
- **Taken: Codex's formats were checked, not copied into the spec unverified.** Its source at `d0264b75` and its hooks documentation show the keys, the allow and the block are Claude Code's, and that it reads an `ask` as an error and runs the command. `agent-harness` states them, and the workaround flag is deleted.
- **Taken: this repository's own settings stay requirements of `config`** (`verify-here` and the like), and the rule's decision gains that case. A spec branch can't set `clm.toml`, so without them nothing states a setting or orders the build that switches it on.
- **Taken: a clause that is the root's code stays where it is until the root's wave.** `machine`'s "exit 0 whatever the states" and the command lines of `gate.fingerprint` and `agent-harness` are the root's code. Cutting them here would leave them stated nowhere for five waves; a move puts both sides on one branch.
- **Not taken: merging requirements to bring `model` and `config` under budget.** Most of their IDs are printed as rule labels or named under `#### Needs` and `#### Enforced by`; a merge changes what tests in seven crates assert and saves about 40 tokens a requirement. Both stay over 12,000 spec tokens after the pass.
- **Not taken: cutting `context`'s wasm build and `bundle_from` now.** No code uses either, but `ci` builds the crate for wasm, so the cut goes with `ci` in wave 3.
- **Not taken: moving `model/exemptions`' sentence on a glob that matches other components' files to `check`**, whose behaviour it is: wave 2 holds `check` and moves it with both sides on one branch.

## Outcome

- **`vcs`**: `one-git-layer` reads every crate. The Contract offers `files_to_commit`, `FetchError` and `Error`'s variants. `is_ancestor` and `extract_tree` state their errors. "never removable" is proven through `remove_worktree` itself, at its 5 seconds.
- **`model`**: the Contract names what other crates call and says what a violation is. `graph-check` covers every check the code runs. `test-dependencies-retired` is merged into `valid-frontmatter`. 21 scenarios state refusals tests already assert.
- **`config`**: `component-size-from-text` is merged into `component-size-setting`. The Contract offers `load`, `committed_env`, `node_size_from` and `process_env`. The Design loses its restatements and the narration of builds long done.
- **`context`**: `says-component` goes, its scenario and heading clause to `bundle-contents`. `bundle-components` states `bundle_nodes`, `served_capabilities`, `bundle` and `hash`, which `clm rebase` relies on.
- **`apply`**: `stale-base` says a base that isn't a whole hash is stale. `reapply` is a requirement. What `clm apply`, `clm archive` and `clm hashes` print and refuse is stated.
- **`rename`**: the two contradictions are corrected. Exit 2, `--help`, the order of the `rebased` lines and what a replay compares are stated.
- **`gate.fingerprint`**: the bytes hashed are stated exactly, with `added_changes`, `parse_ls_tree` and the call's failure.
- **`agent-harness`**: both agent harnesses' events and decisions are stated in the requirement; the hook event loses its `kind`.
- **`machine`**: the JSON keys, the text lines, the disk rule, the sample's shape and the rows' layout are in the requirements that pointed at the Design.
- **`upgrade`** depends on `vcs`.
- 139 requirements become 138: `model/test-dependencies-retired`, `config/component-size-from-text` and `context/says-component` go, each with its scenarios moved; `context/bundle-components` and `apply/reapply` are added.

## Constraints

- No behaviour changes but three, each agreed: `vcs/one-git-layer` fails for a crate it used to pass over; `agent-harness`'s hook event loses a field no caller reads; `model` no longer says whether a component is optional, which no caller asks.
- No requirement whose ID a violation prints, or another spec or `ARCHITECTURE.md` names, is renamed or merged.
- Every clause newly stated is what the code does today, checked against the source; the ones no test asserts are listed for the builds below.
- `removal-needs-a-yes`: every sentence, scenario and test below was put to the human on 2026-11-19, who agreed them as recommended.

## What the requirements no longer state

Sentences that only say where a test sits, cut with no separate yes (`cuts-are-what-a-program-lists`): the sentence "Every test of `<component>`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `<component>`'s own files…" of `commands-in-crate` in `apply`, `config`, `context` and `rename`.

Requirements that go, each scenario moved under its name:
- `model/test-dependencies-retired`. Its scenario and the refusal of `test_depends_on` are `valid-frontmatter`'s. Lost: "the graph SHALL expose no test dependencies", an absence, and a sentence about a removal marker on a field that no longer exists.
- `config/component-size-from-text`, whole into `component-size-setting`.
- `context/says-component`. The `# Ancestors` heading and the scenario are `bundle-contents`'s. Lost: "the bundle's own headings and text SHALL hold no old word", proven only by a scan of source.

Scenarios that go:
- `model/component-keys` "an unknown component": word for word `parse-backlog`'s "unknown node".
- `config/local-settings` "no local file": the same GIVEN and THEN as `config-get`'s "from the repository"; the sentence it illustrated stays, with its test.

SHALL sentences and clauses that go:
- `vcs/one-git-layer`: "and SHALL NOT scan a crate that doesn't list `clm-git`". The check now reads them all.
- `vcs/repo-queries`: "`fetch(root)` SHALL run `git fetch origin`." and `vcs/remote-name`: "and `fetch(root)` SHALL fetch the remote `REMOTE` names". `fetch-tries-again` states both.
- `vcs/fetch-tries-again`: "Its retries SHALL be `fetch_retrying`'s loop, not a second copy of it." and `vcs/worktrees-tries-again`: "Its retries SHALL be the loop `fetch` and `fetch_retrying` run, given its own waits, not a second copy of it." How the code is laid out; no test can observe it.
- `vcs/git-isolated`: "The variables SHALL be the ones git lists, not a list written in the code." It is in the Design.
- `vcs/remove-worktree`: "the limit SHALL be an argument of a function the crate doesn't export…". `vcs/tree-blobs`: "with one `git ls-tree` and one `git cat-file --batch`". `vcs/git-follows-its-clone`: that the work tree is "found only when the environment sets `GIT_DIR` or `GIT_WORK_TREE` and found again only when either variable's value has changed since"; that it follows the variables as they are at each call stays.
- `model/optional-nodes`: "The model SHALL expose no frontmatter check that takes no component ID…", an absence whose point `apply/declared-frontmatter-keys` states; and "It SHALL expose whether each component is optional.", which nothing outside the crate asks.
- `model/graph-check`: "Loading SHALL report the same violations as now, from the same rules, for the same files, and keep the same components and items; only where in its code the rules run changes." The narration of a refactor.
- `model/component-keys` and `config/component-size-setting`: "No `clm:remove-by 20261102-rename-node-to-component-contract` marker SHALL be left." A file's text, which `check/remove-by-markers` reports.
- `config/component-size-setting`: "`clm_config::node_size_from(text)` SHALL read `[component_size]`, and fail for a text that sets the old name." The statement that moves in says it.
- `config/retired-terms-detector`: "With this repository as it stands, the detector SHALL exit 0." `clm/retired-terms` proves the same scan.
- `context/bundle-carries-constraints`: "`bundle_from` SHALL print the same, from the graph." `bundle-from-text` states it.
- `apply/answered-questions` and `apply/promoted-item`: "The deletion SHALL be one of the writes `atomic` holds back: when the change has a violation, the item stays." `atomic` states it, and takes both scenarios.
- `apply/resulting-graph-call`: "The graph SHALL hold no item `clm apply` deletes with the change and every item it keeps." Its first sentence says it.
- `apply/hash-command`: "Every message CLM prints that points to the command…" is now "Every message `clm apply` prints…". Lost: an `apply` rule over the triage prompts' text.
- `rename/refusals`: "It SHALL take the change's status, `renames` and `resolves` from the change `clm_model::load` loads, and SHALL write the change's `change.md` only by its `status:` line." `finish` states the second half; the first is how.
- `rename/component-key`: "and `clm rename` SHALL rewrite a backlog item's `components` values wherever `references` rewrites an item's components". `references` states it.
- `gate.fingerprint/fingerprint`: "and `clm-fingerprint` SHALL run `git` only through `clm-git`". `vcs/one-git-layer` states it for every crate.
- `agent-harness/abilities`: "No agent harness's `confirm` SHALL be true unless its Design row says so." The first sentence fixes each value. `agent-harness/hook-events`: the event's "kind".
- `machine/disk-room`: "It SHALL be a pure function of the same readings, record, places and `max_builds` the disk's state is judged on." `machine-report` says it of `judge`, which returns the room. `machine/build-measure`: "and be written through a temporary file and a rename".
- The sentences "A test reads the string literals in `crates/<crate>/src/`…" of `says-component` in `apply` and `model`, which hold no SHALL.

Scenario outcomes that change under their names:
- `vcs/one-git-layer` "a crate that hasn't migrated": it now fails.
- `vcs/remove-worktree` "never removable": through `remove_worktree`, after at least 5 seconds.
- `rename/marker-rewrite` "current and behind markers": `clm pending a` lists `a/y` as `behind`, as its test has always asserted.
- `model/optional-nodes` "optional depends on required" and "inherited from an ancestor": without the query that goes.
- Reworded to name a requirement in place of a function, or to drop where a test sits: `rename`'s "a reworded Contract another change modifies", "a flagged requirement reworded", "a flagged requirement renamed" and "the archived record loads"; `model/term-lists` "lists match the types"; `config/config-list` "a key get knows"; and "a command called in a test" and "arguments it can't parse" of `commands-in-crate` in `apply`, `config`, `context` and `rename`. `context/bundle-errors` "unknown node" gains the exact line.

Tests the builds delete are in `deleted-tests.txt`, 19 lines, each with its reason: seven tests of the three scans and their scanners, eight duplicates or subsets, two that prove a rule or a query that goes, the fingerprint's source scan, and a unit test the 5-second scenario replaces.

## What no test asserts today

A floor: each build's review looks for more. Each is a clause the code already does.
- `vcs`: the new scenarios of `one-git-layer`; `Error::Failed`'s `status` and `stderr`; what `FetchError::Offline` displays; "never removable" at 5 seconds.
- `model`: the graph check's enforcement and flag checks; the parse of one `change.md` alone; a frontmatter section with no fence; a missing `change.md`; `renames` that isn't a list; the path of "no root spec".
- `config`: no `clm.toml` read outside a CLM repository; `build` and `verify` whole from one layer; the fallback when `commondir` can't be read; an array of more than one string printed; a variable that isn't UTF-8, end to end.
- `context`: `served_capabilities`' order; `bundle` for an unknown component; no `# ARCHITECTURE.md` heading and no `# Dependencies` section through the command; the streams of "invalid graph".
- `apply`: the label for a component that is gone; "no such section", "a list item in the delta", "no frontmatter section", "no such change" twice; the line `clm archive` prints; `reapply`'s "an answered question is left" and "a stale delta"; the printed `removed backlog/<slug>.md`.
- `rename`: exit 2 when git fails; "nothing committed"; "a tree that is no checkout"; a replay of a rename that refuses; what a replay leaves uncompared; both fenced-code scenarios.
- `gate.fingerprint`: what `added_changes` returns; "no merge base"; the exact hash of "paths are bytes".
- `agent-harness`: "the reason comes first"; that a block's stderr is the reason alone.
- `machine`: "outside a git repository"; `disk.state`, `disk.path`, `disk.available` and `memory.available` in the JSON; the text of an unknown memory or disk reading; adjacent cells of one state as one run.

## Open questions

- **`model` and `config` stay over budget**, at about 21,000 and 15,500 spec tokens against 12,000. `model` splits along its record kinds (changes, the human layers, the backlog), each needing a crate of its own (`splits-import-one-way`); `config` could give this repository's own settings to a child. Or the human sets each a `size_budget`. For after the waves.
- **Found and filed, not mended here:** `backlog/the-gate-s-placement-rule-skips-a-behaviour-sentence-that-holds`, `backlog/an-i-o-failure-while-clm-apply-writes-leaves-the-files-already`, `backlog/the-fingerprint-covers-only-the-clm-root-s-subdirectory-and-the`, `backlog/a-change-md-s-gate-key-is-rejected-by-loading-and-accepted-by`, and the idea `backlog/codex-can-name-a-subagent-s-model-and-abilities-says-it-can-t`.
- **For later waves**, recorded in the plan item when this lands: `context`'s wasm build with `ci`; `model/exemptions`' glob sentence, `Priority::cap` and the backlog item's printed line to `check`; the root's command clauses, `config/settings-guide-covers-keys` with the root's guide scans, and the `shared-plumbing` family.
