---
title: View
status: active
owners: ["@clunny"]
depends_on: [model, check, orchestrator, context, vcs]
paths: ["crates/clm-view/**"]
successor: ""
serves: [capability/see-project-state]
size_budget: { code_tokens: 55000 }
---

## Contract
Views of a CLM repository for people to read (`views-rendered-by-clm`). `clm view <name>` prints one of three views:
- **`spec`**: the architecture. Every component nested under its parent, with its status, its requirements counted by coverage state and drawn as a bar segmented by state with `n/m` current of total, owners and `depends_on` edges; its chart and SVG also show each pending component's queue state, and mark a component that is over its size budget (`oversized-is-a-state`).
- **`flight`**: the work in flight. Every component with pending work and its queue state (`ready`, `claimed`, `blocked`), who holds its claim and for how long, and its pending requirements; then every live change with its requirements current of those its deltas add or modify, and whether it is complete.
- **`blockers`**: what holds work up. Every component that can't start, grouped under the blocker at the end of its chain of waits, ranked by how many components that blocker holds up, with what waits on the human first.

`--format text` (the default) draws a chart in box-drawing characters for a terminal or the chat; `--format ansi` draws the same chart coloured with ANSI escapes; `--format svg` prints a standalone SVG document; `--format json` prints the view's model. `--width <columns>` fits the chart to a width. `clm status` prints a grouped summary of the repository: totals first, then components, changes and claims.

Both read the working tree and the local refs only, never fetch, and exit 0, or print the spec graph's violations and exit 1. Since `clm next` reads `origin/main` instead, both name the commit they describe and warn when it is behind `origin/main` as of the last fetch (`views-name-their-commit`): `clm status`'s summary and each chart begin with the checkout line `status-report` defines, and the JSON carries the same as `checkout`. The models and renderers are library calls without git or a checkout, taking what `queue_from_refs` takes, so the `web` Worker computes the same views, and the crate builds for `wasm32-unknown-unknown` (`view-library`). Reading the repository for the two commands, the working tree, the checkout and the refs, is `view.repo`'s, and so is `clm view <name> --at <rev>`, which describes `<rev>`'s commit instead of the working tree (`view.repo/view-at-rev`); the CLM pane always passes `--at origin/main` (`plugin.panel`).

## Design
Crate `clm-view`, with a model per view and a renderer per format. The commands are `view.repo`'s crate, `clm-view-repo`, which reads the repository and hands this crate's models and renderers what it read: `crates/clm` dispatches `clm view` and `clm status` to it, and the `clm-hooks` session-start hook prints its status. `clm_check` gives the `components` and `changes` rows of `clm status` as cells, and `clm_orchestrator` the `claims` rows, each proven by tests in its own crate; `clm-view` adds the summary and lays the sections out.

**Models.** `spec_view`, `flight_view` and `blockers_view` take the graph and markers as `clm_check::report` does, and the component states and pending requirements from `clm_orchestrator::queue_from_refs`; `status` is built from all three. The `spec` model is a function of the graph and markers alone, so the `web` app stores it in a snapshot; the queue state its chart and SVG show is the `flight` model's, computed from the same refs and handed to the renderer beside the model (a component the `flight` model doesn't list is `converged`), so `clm view spec` and `clm view flight` never disagree about a component. JSON is the serde form of the model, with snake_case keys and a top-level `"view"` naming the view; fields may be added, never renamed or removed without a spec change, since the `web` app and the plugin's panel read them.

**Checkout** (`views-name-their-commit`). `view.repo` reads the checkout through `clm-git` (`git-plumbing-layer`): `HEAD`'s hash is cut to 7 hex digits rather than taken from `--short`, which can lengthen. The line is cut to `--width` like the chart's own lines, its `⚠` part amber in `ansi` and the SVG (`colour-meanings`).

**Oversized components** (`oversized-is-a-state`). `spec_view` and `status` take an optional size report, `check`'s `size(...)` result, computed as `clm size` computes it; when the settings can't be loaded the command passes none and prints `size not checked: <error>` on stderr. The web app's `replica` passes none until it can compute one (`replica/snapshot-size`).

**Component detail** (`component-detail`, `panel-inspector-contract`). The inspector's fields ride on the `spec` model so the panel draws them from it rather than reading `specs/`, `changes/` or `backlog/` itself (`views-rendered-by-clm`). All three are functions of the graph and markers, so the model stays one the `web` app can store in a snapshot, at the cost of one more coverage pass in `spec_view`. The `web` Worker's shape check (`web.worker`'s `current_shape` in `crates/clm-web/src/snapshot.rs`) lists the fields, so it sees a snapshot stored before them as stale and rebuilds it; nothing deserialises the model, so they carry no serde default. The spec branches on `origin` that touch the component aren't here: `clm view` reads the working tree and the local refs, not each branch's tree.

**Blockers** (`blockers-view`). A component's waits are the edges `queue_from_refs`'s `blocked { by }` gives it; `depends_on` gives none (`needs-alone-order`). A need's edge goes on to the needed requirement's own blockers when that requirement is blocked, so the root is never a `ready` component whose session would leave the needed requirement out. A component waiting on several things appears under each root its waits reach. The `flight` model's components carry no `paths` and no `waiting_on`, and the view shows no `waiting` state.

**Charts.** Long names are cut with `…`. The spec chart is an indented tree drawn with `├─ └─ │`, each component's line holding, in order, its ID, status, queue state, `⚠ oversized`, bar and `n/m`, owners, and `→` with its dependencies. The bar is ten cells shared in proportion among the component's current (`█`), behind (`▓`) and unbuilt (`░`) requirements, each non-zero count getting at least one cell and the rest going by largest remainder, ties in the order current, behind, unbuilt. `spec_view` takes the obsolete counts from `clm_check::pending` rather than from the requirement's ID, so the spec chart, the `flight` view and `clm next` agree on which component holds a removed requirement (`node-removal`). The flight chart draws each component's pending requirements as a bar of behind, unbuilt and obsolete, and each change's requirements as a bar with `complete` where it applies. The pinned line shapes (`flight-component-lines`, `spec-component-lines`, `blockers-root-lines`) exist because the `clm-panel` plugin finds a component's line by its prefix to add its own rows beneath and presses lines by position, the nth such line being the model's nth component or root (`plugin.panel`'s `component-inspector` and `blocker-chains`); they change only with this spec. Colours, in `ansi` and SVG, are `colour-meanings`, shared with the `web` app. The SVG lays components out in layers, dependents above their dependencies, with children inside their parent's box; it embeds its own styles and fonts fall back to the system's. Each box holds its label, its bar as coloured segments and `n/m`; a `converged` component keeps the outline of its status.

**Size budget** (`slim-components`, `node-size-budgets`). The code budget is 55000 tokens, over the default 40000, because the three views and `clm status` share one chart and SVG shell (`chart.rs`, `svg.rs` and the head line every form begins with), which a split by view would leave in the parent while each child held a third of its callers.

## Requirements

### Requirement: status-report
`clm status` SHALL print, from the working tree and the local refs only and without fetching, a summary and then its sections: `components` and `changes` (`check`'s `status-report`), each `components` row gaining the cell `oversized` when the component is oversized (`oversized-is-a-state`), then `claims` (`orchestrator`'s `status-claims`). It reads no ledger: nothing is reviewed after it lands (`yes-no-review`). The summary is the line `CLM status` followed by these lines, each indented two spaces:
- the checkout line, when `HEAD` can be read: `commit <hash>`, the first 7 hex digits of `HEAD`'s hash, followed by two spaces and `⚠ <n> behind origin/main` when `refs/remotes/origin/main` exists and has `n` commits, `n` non-zero, that `HEAD` doesn't contain (`views-name-their-commit`);
- `components <n>: <a> active, <s> unbuilt; <current>/<total> requirements current`, counting the capabilities that don't hold `#### Untested` with the requirements (`model/human-layers`), followed by `, architecture <e>/<t> enforced` when `ARCHITECTURE.md` holds entries, `<t>` the entries holding an `#### Enforced by` block and `<e>` those of them `check.coverage`'s `coverage-report` gives as current, then `, <u> untested` when any entry of either layer holds `#### Untested`, an entry holding both blocks counting in `<t>` and in `<u>`, then `, <k> oversized` when any component is oversized (`human-layer-proof`);
- `changes <n> live`, followed by `: ` and `<count> <status>` for each status that has changes outside the archive, in the order `implementing`, `applied`, `approved`, `proposed`, separated by `, `, then `, <count> complete` when any live change is complete;
- `pending <n> components`, followed by `: ` and `<count> <state>` for each of `claimed`, `ready` and `blocked` that has components, separated by `, `, counting the components `queue_from_refs` gives with the default expiry;
- `waiting on you <n>`, followed by `: ` and what waits on the human: every `question` and `human-task` backlog item as `backlog/<slug>`, whether or not it blocks a requirement, and the `proposed` and `approved` changes outside the archive as `changes/<id>`, sorted, separated by `, `, when there are any;
- `backlog <n>`, followed by `: ` and `<count> <kind>` for each kind that has items, in the order bug, deferred, idea, question, human-task, then any other, separated by `, `;
- `claims <n> live`, followed by `, <count> lapsed` and `, <count> merged` when there are any.

A blank line follows the summary and each section. A section is its name on a line, then its rows, each indented two spaces, or four under a group's name, which is itself indented two. The cells of a section's rows are separated by at least two spaces, each column starting at the same position on every row of the section. It SHALL exit 0, or print the violations and exit 1 when the spec graph has violations.

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
- GIVEN component `a` whose code is over its budget with no `size_budget` set, component `b` within its budgets, and component `c` whose code is over the default budget but within the `size_budget` its frontmatter sets
- WHEN `clm status` runs
- THEN its `components` summary line ends `, 1 oversized`, `a`'s row holds the cell `oversized`, and neither `b`'s row nor `c`'s holds such a cell

#### Scenario: checkout line
- GIVEN a repository whose `HEAD` is two commits behind its remote-tracking ref `origin/main`, and an `origin` that can't be reached
- WHEN `clm status` runs, then again with `HEAD` at `origin/main`, and then with no remote-tracking ref `origin/main`
- THEN each exits 0; the first's line after `CLM status` starts `  commit ` and ends `⚠ 2 behind origin/main`, the second's is `  commit ` followed by `HEAD`'s hash cut to its first 7 hex digits and nothing more, and the third's starts `  commit ` and holds no `behind`

#### Scenario: pending states
- GIVEN component `a` with an uncited requirement, component `b` with an uncited requirement claimed by a branch pushed an hour ago, and component `x` whose only pending requirement is in the `blocks` of `backlog/q`
- WHEN `clm status` runs
- THEN its summary contains `pending 3 components: 1 claimed, 1 ready, 1 blocked`, and no line of the summary names a `waiting` state

#### Serves
- capability/see-project-state
- architecture/views-name-their-commit
- architecture/slim-components

#### Needs
- check.coverage/coverage-report
- check.coverage/status-report
- plugin.panel/component-inspector
- model/parse-backlog
- check.coverage/state-names
- plugin.hooks/session-status

### Requirement: view-command
`clm view <name>` SHALL print the `spec`, `flight` or `blockers` view of the repository, from the working tree and the local refs only and without fetching, and exit 0, or print the spec graph's violations and exit 1. `--format` SHALL select the form: `text` (the default), a chart in box-drawing characters with no escape sequences; `ansi`, the same characters with ANSI colour escapes added; `svg`, a standalone SVG document; and `json`, the view's model. In `text` and `ansi`, no line SHALL be wider than `--width` columns (default 100), and every state the chart shows SHALL be written as a word or symbol, not only as a colour. When `HEAD` can be read, the `text` and `ansi` charts SHALL begin with the checkout line as `status-report` defines it, the `svg` document SHALL hold the same words, and the `json` model SHALL carry a top-level `checkout` with the line's `commit` and, when `refs/remotes/origin/main` exists, its count as `behind`, 0 included (`views-name-their-commit`). An unknown name or format, `png` among them, SHALL exit 2 with usage.

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

#### Scenario: offline
- GIVEN a repository whose `origin` is unreachable, with pending component `x` and a remote-tracking ref `origin/impl/x` committed 1 hour ago
- WHEN `clm view flight` runs
- THEN it exits 0 and shows `x` as claimed

#### Scenario: unknown view
- WHEN `clm view everything` runs, and then `clm view spec --format png`
- THEN each exits 2

#### Scenario: checkout behind on the chart
- GIVEN a repository whose `HEAD` is three commits behind its remote-tracking ref `origin/main`, and an `origin` that can't be reached
- WHEN `clm view flight`, `clm view flight --format svg` and `clm view flight --format json` run, and then `clm view spec --format json` with `HEAD` at `origin/main`
- THEN each exits 0; the chart's first line starts `commit ` and ends `⚠ 3 behind origin/main`, the SVG contains the text `3 behind origin/main`, the first JSON's `checkout` has `behind` 3 and `commit` the hash of `HEAD` cut to its first 7 hex digits, and the last JSON's `checkout` has `behind` 0

#### Serves
- capability/see-project-state
- architecture/views-name-their-commit

#### Needs
- plugin.panel/views-panel

### Requirement: spec-view
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
- GIVEN component `a` whose spec is over its budget with no `size_budget` set, and component `b` within its budgets
- WHEN `clm view spec --format json` runs, and then `clm view spec`
- THEN `a` has `oversized` true and `b` false, and `a`'s chart line holds `⚠ oversized` while `b`'s holds no `⚠`

#### Scenario: oversized without sizes
- GIVEN the `spec` model computed by the library without a size report
- WHEN the tests read it
- THEN every component's `oversized` is absent, and the chart and SVG show no `⚠`

#### Needs
- check.coverage/pending-report
- check.coverage/state-names

### Requirement: flight-view
The `flight` view SHALL hold every component that isn't `converged`, sorted by effective priority, highest first, then by ID, each with the state `queue_from_refs` gives it, for a claimed component the hours since its claim's last push and the hours until the claim lapses, and its pending requirements with their states; and every change outside the archive whose status isn't `done` or `abandoned`, grouped by status in the order `implementing`, `applied`, `approved`, `proposed`, each group sorted by priority, highest first, then by ID, each with its requirements current out of the requirements its deltas add or modify, the markers still citing requirements it removed, and whether it is complete, each exactly as `check.coverage/change-progress` gives it for that change: an `applied` or `implementing` change leaves out requirements the spec no longer holds, and the markers it lists cite only removed requirements the spec no longer holds. It SHALL also hold totals: pending components by state, live changes, and requirements still to make current.

#### Scenario: claimed node
- GIVEN component `a` with two uncited requirements, claimed by a branch last pushed an hour ago
- WHEN `clm view flight --format json` runs
- THEN `a` is `claimed`, pushed 1 hour ago and lapsing in 3 hours, with its two requirements `unbuilt`

#### Scenario: change progress
- GIVEN an `applied` change `d` whose delta adds requirements `n/x` and `n/y`, `n/x` current and `n/y` cited at an older version
- WHEN `clm view flight --format json` runs
- THEN `d` is in the `applied` group with 1 of 2 requirements current and not complete, and the totals count 1 requirement still to do

#### Scenario: requirement removed later
- GIVEN an `applied` change `d` whose delta adds `n/x` and `n/y`, `n/x` current, and no component holding `n/y`
- WHEN `clm view flight --format json` runs
- THEN `d` has 1 of 1 requirements current, no markers listed, and is complete

#### Scenario: order
- GIVEN pending components `a` at effective priority `P5` and `z` at `P2`, and an `applied` change `a1` and an `implementing` change `z9`
- WHEN `clm view flight` runs
- THEN `z` is listed before `a`, and `z9` before `a1`

#### Scenario: removed requirement still cited
- GIVEN an `applied` change `d` whose delta removes `n/x`, no component holding `n/x`, and a test citing `n/x` at `t.rs:3`
- WHEN `clm view flight --format json` runs
- THEN `d` lists the marker `t.rs:3` and is not complete, and `clm status`'s row for `d` has no `complete` cell

#### Needs
- check.coverage/change-progress
- check.coverage/state-names

### Requirement: blockers-view
The `blockers` view SHALL hold every component `queue_from_refs` reports as `blocked`, grouped under each root blocker its chain of waits reaches: a component that is `ready` or `claimed`, or a backlog item. A component blocked by a requirement it needs (`requirement-needs`) waits on that requirement's component when the requirement is unblocked, and on the requirement's own blockers when it is blocked, and the chain continues from there. For each root it SHALL hold its weight, the number of distinct components it holds up directly or through other components, and the components between it and each one. Roots that wait on the human (a `question` or `human-task` backlog item) SHALL come first, then the rest, each by weight, highest first, then by name.

#### Scenario: chain weight
- GIVEN component `c` whose only pending requirement needs `b/y`, component `b` whose only pending requirement `b/y` needs `a/x`, and `a/x` uncited on claimed component `a`
- WHEN `clm view blockers --format json` runs
- THEN the only root is `a`, with weight 2, holding up `b` and `c`

#### Scenario: human first
- GIVEN component `a` whose only pending requirement is in the `blocks` of `backlog/q` of kind `question`, and components `e`, `f`, `g` and `h`, each with one pending requirement, which needs `d/x`, uncited on claimed component `d`
- WHEN `clm view blockers` runs
- THEN `backlog/q`, weight 1, is listed before `d`, weight 4

#### Scenario: dependency holds nothing
- GIVEN component `a` whose only pending requirement is blocked by `backlog/q`, and component `b` depending on `a` with an unblocked pending requirement
- WHEN `clm view blockers --format json` runs
- THEN `b` appears under no root, since `b` is ready

#### Scenario: need reaches its node
- GIVEN component `a` whose only pending requirement needs `b/y`, uncited and unblocked on claimed component `b`
- WHEN `clm view blockers --format json` runs
- THEN the only root is `b`, with weight 1, holding up `a`

#### Scenario: need reaches the needed requirement's item
- GIVEN component `a` whose only pending requirement needs `b/y`, uncited on component `b` and in the `blocks` of `backlog/q` of kind `question`, where `b` has another unblocked pending requirement
- WHEN `clm view blockers --format json` runs
- THEN `a` is under `backlog/q`, weight 1, and not under `b`, which is `ready`

#### Needs
- plugin.panel/component-inspector
- model/parse-backlog

### Requirement: view-library
The `spec`, `flight` and `blockers` models SHALL be library calls of the `clm-view` crate that take the spec graph, the markers, the claim refs with their tip times and whether each is merged into the default branch, the current time and the expiry, and touch no file, process or network. A merged ref isn't a claim: it is left out of what the queue sees, as `merged-claims-released` decides. `clm view <name> --format json` SHALL print the library's model for the repository, serialised, with the `checkout` the command reads added at the top level (`view-command`); the models SHALL carry no checkout. With its default features disabled, `clm-view` SHALL build for `wasm32-unknown-unknown`, models and renderers alike.

#### Scenario: command matches library
- GIVEN a repository with a pending component and a claim ref
- WHEN `clm view flight --format json` runs, and a test calls the library with the same graph, markers, refs and time
- THEN both give the same JSON once the command's `checkout` key is removed

#### Scenario: wasm build
- GIVEN the workspace
- WHEN `cargo build -p clm-view --no-default-features --target wasm32-unknown-unknown` runs
- THEN it succeeds

#### Serves
- capability/watch-beside-chat
- capability/follow-from-browser

### Requirement: blockers-root-lines
In `text` and `ansi`, the blockers chart SHALL hold a line whose text is `blockers`, followed by the single line `nothing is blocked` when there is no root, or else, for each root in the view's order, the root's line and then the lines of its tree. A root's line SHALL start at the first column with the root's name (a component ID or `backlog/<slug>`) followed by a space or by the end of the line, or a non-empty prefix of it followed by `…` when it doesn't fit; every line of its tree SHALL start with `├`, `└`, `│` or a space. No other line SHALL follow the heading. With ANSI escapes removed, the `ansi` form SHALL have the same lines.

#### Scenario: root lines
- GIVEN component `c` whose only pending requirement needs `b/y`, component `b` whose only pending requirement `b/y` needs `a/x`, `a/x` uncited on claimed component `a`, and component `d` whose only pending requirement is in the `blocks` of `backlog/q` of kind `question`
- WHEN `clm view blockers --format ansi` runs
- THEN with escapes removed, the lines after `blockers` are `backlog/q`'s line, one line starting `└` naming `d`, `a`'s line, and the lines `├─ b` and `└─ c`, in that order: `c`'s need `b/y` is itself blocked, so `c` reaches `a` directly and both hang off `a`

#### Scenario: nothing blocked
- GIVEN no component blocked
- WHEN `clm view blockers` runs
- THEN the line after `blockers` is `nothing is blocked`, and no line follows it

#### Needs
- plugin.panel/component-inspector

### Requirement: says-component
The text of `clm status` and of the charts and SVG documents `clm view` draws SHALL say component and components, never the old words, as `component-words` defines them: the summary lines and headings `status-report` and `flight-component-lines` quote, and the flight chart's summary, `<n> components pending`. The models' JSON fields and values and the SVG's class names are data the `web` app and the panel read, and keep their names (`component-words`).

#### Scenario: the flight summary
- GIVEN pending components `a` and `b`
- WHEN `clm view flight` runs
- THEN its summary holds `2 components pending`

#### Needs
- plugin.panel/component-heading

#### Serves
- architecture/plain-terms

### Requirement: flight-component-lines
In `text` and `ansi`, the flight chart SHALL hold a line whose text is `components`, followed by exactly one line per pending component, in the view's order, each starting with two spaces and then the component's ID followed by a space, or a non-empty prefix of the ID followed by `…` when the ID doesn't fit, or by the single line `  none` when no component is pending; no other line SHALL come between `components` and the heading `changes`, and every line under `changes` SHALL be a status group name at two spaces, the line `  none`, or indented by more than two spaces. With ANSI escapes removed, the `ansi` form SHALL have the same lines.

#### Scenario: node lines
- GIVEN pending components `a` and `b.c`, and an `applied` change `20260101-d`
- WHEN `clm view flight --format ansi` runs
- THEN with escapes removed, the lines after `components` and before `changes` are exactly two, starting `  a ` and `  b.c `, and the change's line starts with four spaces

#### Scenario: long ID
- GIVEN a pending component whose ID is 60 characters long
- WHEN `clm view flight --width 40` runs
- THEN its line starts with two spaces and the first characters of its ID followed by `…`, and no line is wider than 40 columns

#### Scenario: nothing pending
- GIVEN no pending component and no live change
- WHEN `clm view flight` runs
- THEN the line after `components` is `  none`, the line after `changes` is `  none`, and no other line lies between or after them

#### Needs
- plugin.panel/component-heading

### Requirement: spec-component-lines
In `text` and `ansi`, the spec chart SHALL hold, after its first line when it begins with the checkout line or, with `--at`, the line naming the commit (`view-at-rev`), exactly one line per component, in the `spec` model's order, and no other line. Each SHALL start with its tree prefix, three columns per level below the root made only of `│`, `├`, `└`, `─` and spaces (none for the root), and then the component's ID followed by a space, or a non-empty prefix of the ID followed by `…` when the ID doesn't fit. With ANSI escapes removed, the `ansi` form SHALL have the same lines.

#### Scenario: spec lines
- GIVEN a root `r` with children `a` and `b`, and `a` with a child `a.c`, in a repository with a commit at `origin/main`
- WHEN `clm view spec --format ansi` runs, and then `clm view spec --at origin/main --format ansi`
- THEN with escapes removed, the lines of each after its first are exactly four, starting `r `, `├─ a `, `│  └─ a.c ` and `└─ b `

#### Scenario: spec long ID
- GIVEN a child of the root whose ID is 60 characters long
- WHEN `clm view spec --width 40` runs
- THEN its line starts `├─ ` or `└─ ` and the first characters of its ID followed by `…`, and no line is wider than 40 columns

### Requirement: component-detail
Each component of the `spec` model SHALL also carry `contract`, the text of its Contract section with trailing whitespace trimmed; `changes`, each live change (outside the archive, status not `done` or `abandoned`) with a delta for the component, once per change, sorted by ID, with its `change`, `status`, and its `current`, `total` and `complete` exactly as `check.coverage/change-progress` gives them; and `backlog`, each backlog item whose `components` name the component, in kind order (bug, deferred, idea, question, human) then by slug, with its `slug`, `kind` and `title` (`panel-inspector-contract`). The three SHALL always be written, empty when nothing names the component. The charts and the SVG SHALL draw none of them.

#### Scenario: node detail
- GIVEN component `a` whose Contract section reads `Alpha does X.`, an `applied` change `20260101-c` whose delta modifies `a` and whose two added requirements have one current, an archived change `20250101-z` with a delta for `a`, a `done` change `20260102-d` with a delta for `a`, backlog items `q` of kind `question` titled `Ask` and `b` of kind `bug` titled `Fix` whose `components` name `a`, item `o` whose `components` name only `b`, and component `c` that no live change's delta and no backlog item's `components` name
- WHEN `clm view spec --format json` runs, and then `clm view spec` and `clm view spec --format svg`
- THEN `a`'s `contract` is `Alpha does X.`, its `changes` is exactly one entry, `20260101-c` with status `applied`, current 1, total 2 and complete false, and its `backlog` is exactly `b` (`bug`, `Fix`) then `q` (`question`, `Ask`); `c`'s `changes` and `backlog` are empty lists and its `contract` is its Contract section's text; and neither the chart nor the SVG holds `Alpha does X.`, `20260101-c`, `Ask` or `Fix`

#### Serves
- capability/watch-beside-chat
