---
component: view
---

## MODIFIED Requirements

### Requirement: status-report <!-- base: 538240fb449f1ac1 -->
`clm status` SHALL print, for the graph, markers, claim refs, checkout and size report it is handed, which `view.repo` reads from the repository (`view.repo/reads-the-repository`), a summary and then its sections: `components` and `changes` (`check`'s `status-report`), each `components` row gaining the cell `oversized` when the component is oversized (`oversized-is-a-state`), then `claims` (`orchestrator`'s `status-claims`). It SHALL hold no review section: nothing is reviewed after it lands (`yes-no-review`). The summary is the line `CLM status` followed by these lines, each indented two spaces:
- the checkout line, when it is handed a checkout: `commit <hash>`, the checkout's commit, followed by two spaces and `⚠ <n> behind origin/main` when the checkout's count `n` of commits behind `origin/main` is given and non-zero (`views-name-their-commit`);
- `components <n>: <a> active, <s> unbuilt; <current>/<total> requirements current`, counting the capabilities that don't hold `#### Untested` with the requirements (`model/human-layers`), followed by `, architecture <e>/<t> enforced` when `ARCHITECTURE.md` holds entries, `<t>` the entries holding an `#### Enforced by` block and `<e>` those of them `check.coverage`'s `coverage-report` gives as current, then `, <u> untested` when any entry of either layer holds `#### Untested`, an entry holding both blocks counting in `<t>` and in `<u>`, then `, <k> oversized` when any component is oversized (`human-layer-proof`);
- `changes <n> live`, followed by `: ` and `<count> <status>` for each status that has changes outside the archive, in the order `implementing`, `applied`, `approved`, `proposed`, separated by `, `, then `, <count> complete` when any live change is complete;
- `pending <n> components`, followed by `: ` and `<count> <state>` for each of `claimed`, `ready` and `blocked` that has components, separated by `, `, counting the components `queue_from_refs` gives with the default expiry;
- `waiting on you <n>`, followed by `: ` and what waits on the human: every `question` and `human-task` backlog item as `backlog/<slug>`, whether or not it blocks a requirement, and the `proposed` and `approved` changes outside the archive as `changes/<id>`, sorted, separated by `, `, when there are any;
- `backlog <n>`, followed by `: ` and `<count> <kind>` for each kind that has items, in the order bug, deferred, idea, question, human-task, then any other, separated by `, `;
- `claims <n> live`, followed by `, <count> lapsed` and `, <count> merged` when there are any.

A blank line follows the summary and each section. A section is its name on a line, then its rows, each indented two spaces, or four under a group's name, which is itself indented two. The cells of a section's rows are separated by at least two spaces, each column starting at the same position on every row of the section. The command's exit status is `view.repo/arguments-and-exit`'s.

#### Scenario: entries in the summary
- GIVEN component `a` with requirement `a/x` current, `capability/b` cited by nothing, `capability/u` holding `#### Untested`, `architecture/e` enforced by `a/x`, and `architecture/f` holding only `#### Untested`
- WHEN `clm status` runs
- THEN its summary contains `1/2 requirements current, architecture 1/1 enforced, 2 untested`

#### Scenario: summary
- GIVEN the root `clm` (active, no requirements) and component `a` (active, requirements `a/x` current and `a/y` not), change `20260101-c` with status `applied` whose delta adds both, and two `idea` backlog items
- WHEN `clm status` runs, and then again with no change outside the archive, no backlog item, no claim ref and `a/y` current
- THEN its first line is `CLM status`, and its summary contains `components 2: 2 active, 0 unbuilt; 1/2 requirements current`, `changes 1 live: 1 applied`, `pending 1 components: 1 ready` and `backlog 2: 2 idea`; the second summary contains `changes 0 live`, `pending 0 components`, `backlog 0` and `claims 0 live`

#### Scenario: aligned columns
- GIVEN components `a` and `check`, `a` with 10 requirements and `check` with 2
- WHEN `clm status` runs
- THEN the `components` rows start `  a      ` and `  check  `, their status cells starting in the same column

#### Scenario: changes grouped
- GIVEN change `20260101-c` with status `implementing` whose deltas add three requirements, one current
- WHEN `clm status` runs
- THEN the `changes` section has the line `  implementing` followed by `    20260101-c  1/3 requirements`

#### Scenario: waiting on you
- GIVEN backlog items `q` of kind `question` and `h` of kind `human-task`
- WHEN `clm status` runs
- THEN its summary contains `waiting on you 2: backlog/h, backlog/q`

#### Scenario: claims counted
- GIVEN remote-tracking refs `origin/impl/b` committed 1 hour ago and `origin/impl/d` committed 5 hours ago
- WHEN `clm status` runs
- THEN its summary contains `claims 1 live, 1 lapsed`

#### Scenario: oversized node
- GIVEN a size report in which component `a` is oversized and components `b` and `c` are not
- WHEN the status is built with it
- THEN its `components` summary line ends `, 1 oversized`, `a`'s row holds the cell `oversized`, and neither `b`'s row nor `c`'s holds such a cell

#### Scenario: checkout line
- GIVEN a checkout two commits behind `origin/main`
- WHEN the status is built with it, then with a checkout whose count is 0, then with one that has no count, and then with no checkout
- THEN the first's line after `CLM status` is `  commit `, the checkout's commit and `  ⚠ 2 behind origin/main`, the second's is `  commit ` and the commit and nothing more, the third's starts `  commit ` and holds no `behind`, and the last's starts `  components `

#### Scenario: pending states
- GIVEN component `a` with an uncited requirement, component `b` with an uncited requirement claimed by a branch pushed an hour ago, and component `x` whose only pending requirement is in the `blocks` of `backlog/q`
- WHEN `clm status` runs
- THEN its summary contains `pending 3 components: 1 claimed, 1 ready, 1 blocked`, and no line of the summary names a `waiting` state

#### Serves
- capability/see-project-state
- architecture/slim-components

#### Needs
- check.coverage/coverage-report
- check.coverage/status-report
- plugin.panel/component-inspector
- model/parse-backlog
- check.coverage/state-names
- plugin.hooks/session-status
- view.repo/reads-the-repository
- view.repo/arguments-and-exit

### Requirement: view-command <!-- base: 0fb3094764ed659e -->
`clm view <name>` SHALL print the `spec`, `flight`, `blockers` or `queue` view of the graph, markers, claim refs and checkout it is handed, which `view.repo` reads from the repository (`view.repo/reads-the-repository`). `--format` SHALL select the form: `text` (the default), a chart in box-drawing characters with no escape sequences; `ansi`, the same characters with ANSI colour escapes added; `svg`, a standalone SVG document; and `json`, the view's model. In `text` and `ansi`, no line SHALL be wider than the columns `--width` gives it, and every state the chart shows SHALL be written as a word or symbol, not only as a colour. When it is handed a checkout, the `text` and `ansi` charts SHALL begin with the checkout line as `status-report` defines it, the `svg` document SHALL hold the same words, and the `json` model SHALL carry a top-level `checkout` with the line's `commit` and, when the checkout's count is given, that count as `behind`, 0 included (`views-name-their-commit`). When it is handed a commit in place of a checkout, as `clm view <name> --at <rev>` hands it (`view.repo/view-at-rev`), the `text` and `ansi` charts SHALL begin instead with `<rev> <commit>`, then ` · fetched <age> ago` when the time of a fetch is given, `<age>` the time since it as `<n>m` under an hour and `<n>h` from then on, then ` · checkout <branch>`, or ` · checkout detached` when no branch is given, and no `⚠`; the `svg` document SHALL hold the same words, and the `json` model SHALL carry a top-level `at` with `rev`, `commit`, and `fetched` and `branch` when given, and no `checkout`. The command's default width, its usage errors and its exit status are `view.repo/arguments-and-exit`'s.

#### Scenario: plain chart
- GIVEN a repository with components `a` and `b`, `b` depending on `a`
- WHEN `clm view spec` runs
- THEN it exits 0 and its output contains `a` and `b` and no escape character

#### Scenario: coloured chart
- GIVEN the same repository
- WHEN `clm view spec --format ansi` runs
- THEN its output with the ANSI escapes removed is the output of `clm view spec`

#### Scenario: svg
- GIVEN the same repository
- WHEN `clm view spec --format svg` runs
- THEN its output parses as XML with root element `svg` and contains the text `a` and `b`

#### Scenario: checkout behind on the chart
- GIVEN a checkout three commits behind `origin/main`
- WHEN the `flight` view is rendered with it as `text`, `svg` and `json`, and then the `spec` view as `json` with a checkout whose count is 0
- THEN the chart's first line starts `commit ` and ends `⚠ 3 behind origin/main`, the SVG contains the text `3 behind origin/main`, the first JSON's `checkout` has `behind` 3 and the checkout's `commit`, and the last JSON's `checkout` has `behind` 0

#### Scenario: at a commit
- GIVEN the commit `abc1234` named `origin/main`, fetched 200 seconds ago, with the branch `impl/a` checked out
- WHEN the `flight` view is rendered with it as `text`, `ansi`, `svg` and `json`, and then as `text` with a fetch 3 hours and 59 minutes ago and no branch, and with no fetch time and no branch
- THEN the first chart's first line is `origin/main abc1234 · fetched 3m ago · checkout impl/a`, the `ansi` chart's first line holds no `⚠`, the SVG holds the same words, and the JSON has no `checkout` and an `at` with `rev` `origin/main`, `commit` `abc1234`, that fetch time as `fetched` and `branch` `impl/a`; the later charts' first lines are `origin/main abc1234 · fetched 3h ago · checkout detached` and `origin/main abc1234 · checkout detached`

#### Serves
- capability/see-project-state

#### Needs
- plugin.panel/views-panel
- view.repo/reads-the-repository
- view.repo/arguments-and-exit

### Requirement: spec-view <!-- base: 380d5f8ac7f33f16 -->
The `spec` view SHALL hold every component exactly once, nested under its parent by ID, with its title, status, owners, `depends_on`, and its requirements counted by coverage state: `current`, `behind`, `unbuilt` and `total` for the requirements the component holds, the counts of the component the capabilities are pending on, which `spec_view` takes from the model's `Graph::capability_tests_node` (`model/capability-tests-component`) rather than deciding itself, including the capabilities that don't hold `#### Untested` (`check.coverage/pending-report`), and no component's counting the architecture entries, and `obsolete` for those a live change removed from it that a marker still cites, together with each requirement removed with its component that a marker in a file the component's `paths` own still cites, or, for a marker in a file no component's `paths` own, the root (`node-removal`). Its chart SHALL show each component below its parent, indented one level further, and its SVG inside its parent's box, each with those facts, its requirements as a bar segmented by state with `<current>/<total>` and `✕<obsolete>` when there are any, and, when the component isn't `converged`, the queue state the `flight` model gives it, coloured as the `flight` chart colours it. When a size report is given, each component SHALL carry `oversized`, true when the report says so, and the chart SHALL show `⚠ oversized` on such a component's line and the SVG `⚠` in its label with an amber outline unless a queue state colours the outline; without a report, `oversized` SHALL be absent and nothing marked (`oversized-is-a-state`).

#### Scenario: entries on the root
- GIVEN root `clm` with no requirements, `capability/b` cited by nothing, and `architecture/a` enforced by a rule of another component
- WHEN the `spec` view is built
- THEN the root's counts are 1 `unbuilt` of 1 `total`, and no component `capability` or `architecture` appears

#### Scenario: nested nodes
- GIVEN components `clm` (the root), `check` and `check.drift`, where `check.drift` depends on `model`, owned by `@a`, with 2 requirements, one current and one cited only at an older version
- WHEN `clm view spec --format json` runs
- THEN `check.drift` has parent `check`, `check` has parent `clm`, and `check.drift` has owners `["@a"]`, `depends_on` `["model"]`, current 1, behind 1, unbuilt 0, obsolete 0 and total 2, and no other count

#### Scenario: tree chart
- GIVEN the same components
- WHEN `clm view spec` runs
- THEN the line for `check.drift` follows the line for `check` and is indented further, and holds a bar of `█` cells then `▓` cells, `1/2`, `@a` and `model`

#### Scenario: a node behind its spec is not full
- GIVEN component `a` with 4 requirements, 3 current and 1 cited only at an older version
- WHEN `clm view spec` runs
- THEN the line for `a` holds `3/4` and a bar with at least one `▓` cell and no `░` cell

#### Scenario: queue states on the chart
- GIVEN component `a` with one uncited requirement and a live claim ref `impl/a`, component `b` whose only pending requirement is in the `blocks` of `backlog/q`, and component `c` with every requirement current
- WHEN `clm view spec` runs
- THEN the line for `a` holds `claimed`, the line for `b` holds `blocked`, and the line for `c` holds no queue state

#### Scenario: obsolete markers
- GIVEN component `a` with one current requirement, and a change outside the archive whose delta removed `a/old`, which a test still cites
- WHEN `clm view spec --format json` runs, and then `clm view spec`
- THEN `a` has current 1, total 1 and obsolete 1, and its chart line holds `1/1` and `✕1`

#### Scenario: a removed node's markers
- GIVEN component `a` whose `paths` are `src/a/**`, and a change outside the archive whose delta removed component `gone` with its requirement `gone/x`, which a test in `src/a/t.rs` still cites
- WHEN `clm view spec --format json` runs, and then `clm view spec`
- THEN `a` has obsolete 1 and its chart line holds `✕1`, and the model holds no component `gone`

#### Scenario: oversized on the chart
- GIVEN a size report in which component `a` is oversized and component `b` is not
- WHEN the `spec` view is built with it and rendered as `json`, and then as `text`
- THEN `a` has `oversized` true and `b` false, and `a`'s chart line holds `⚠ oversized` while `b`'s holds no `⚠`

#### Scenario: oversized without sizes
- GIVEN the `spec` model computed by the library without a size report
- WHEN the tests read it
- THEN every component's `oversized` is absent, and the chart and SVG show no `⚠`

#### Needs
- check.coverage/pending-report
- check.coverage/state-names
- view.repo/reads-the-repository

### Requirement: view-library <!-- base: b5ceab6355330a1d -->
The `spec`, `flight` and `blockers` models SHALL be library calls of the `clm-view` crate that take the spec graph, the markers, the claim refs with their tip times and whether each is merged into the default branch, the current time and the expiry, and touch no file, process or network. A merged ref isn't a claim: it is left out of what the queue sees, as `merged-claims-released` decides. The models SHALL carry no checkout: the renderer adds the one it is handed at the top level of the JSON (`view-command`), and that `clm view <name> --format json` prints the library's model for the repository is `view.repo/arguments-and-exit`'s. Every test of `view`'s requirements SHALL sit in `view`'s own files and call `clm-view`: none is left in `crates/clm/tests/status.rs` or `crates/clm/tests/view.rs`, where what only the commands show is `view.repo`'s to prove (`commands-live-in-their-crates`), and `scenario_groups_in_order`, which asserts the `changes` section and the summary line this crate lays out, sits among `view`'s tests citing `status-report`. With its default features disabled, `clm-view` SHALL build for `wasm32-unknown-unknown`, models and renderers alike.

#### Scenario: the model carries no checkout
- GIVEN the `flight` model of a graph with a pending component and a claim ref
- WHEN it is serialised, and rendered as `json` with a checkout
- THEN the first has no `checkout`, and the second is the first with the `checkout` key added

#### Scenario: wasm build
- GIVEN the workspace
- WHEN `cargo build -p clm-view --no-default-features --target wasm32-unknown-unknown` runs
- THEN it succeeds

#### Serves
- capability/watch-beside-chat
- capability/follow-from-browser

#### Needs
- view.repo/reads-the-repository
- view.repo/arguments-and-exit

