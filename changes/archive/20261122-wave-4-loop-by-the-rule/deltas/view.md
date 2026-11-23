---
component: view
base: { frontmatter: "d70d2d0fdd58b112", contract: "1e27142555bc9b14" }
---

## MODIFIED Frontmatter
No crate of `view`'s names `clm-context` or `clm-git`; reading the checkout is `view.repo`'s.

```yaml
depends_on: [model, check.coverage, orchestrator]
```

## MODIFIED Contract
Views of a CLM repository for people to read (`views-rendered-by-clm`). `clm view <name>` prints one of four views:
- **`spec`**: the architecture. Every component nested under its parent, with its status, its requirements counted by coverage state and drawn as a bar segmented by state with `n/m` current of total, owners and `depends_on` edges; its chart and SVG also show each pending component's queue state, and mark a component that is over its size budget (`oversized-is-a-state`).
- **`flight`**: the work in flight. Every component with pending work and its queue state (`ready`, `claimed`, `blocked`), for a claimed one the hours since its claim's last push and until it lapses, and its pending requirements; then every live change with its requirements current of those its deltas add or modify, and whether it is complete.
- **`blockers`**: what holds work up. Every component that can't start, grouped under the blocker at the end of its chain of waits, ranked by how many components that blocker holds up, with what waits on the human first.
- **`queue`**: the build loop's plan (`orchestrator.loop`): what runs, what the loop launches next in its order with each entry's priority and why it is there, what waits on the human, and what waits on other work. It has no `svg` form.

`--format text` (the default) draws a chart in box-drawing characters for a terminal or the chat; `--format ansi` draws the same chart coloured with ANSI escapes; `--format svg` prints a standalone SVG document; `--format json` prints the view's model. `--width <columns>` fits the chart to a width. `clm status` prints a grouped summary of the repository: totals first, then components, changes and claims.

Both read the working tree and the local refs only and never fetch, and exit 0, or print the spec graph's violations and exit 1; what the commands read, how they take their arguments and how they exit are `view.repo`'s to state and prove (`view.repo/reads-the-repository`, `view.repo/arguments-and-exit`), and this component states what is drawn for what it is handed. Since `clm next` reads `origin/main` instead, both name the commit they describe and warn when it is behind `origin/main` as of the last fetch (`views-name-their-commit`): `clm status`'s summary and each chart begin with the checkout line `status-report` defines, and the JSON carries the same as `checkout`. Reading the repository for the two commands, the working tree, the checkout and the refs, is `view.repo`'s, and so is `clm view <name> --at <rev>`, which describes `<rev>`'s commit instead of the working tree (`view.repo/view-at-rev`); the CLM pane always passes `--at origin/main` (`plugin.panel`).

The models and renderers are library calls without git or a checkout, so the `web` Worker computes the same views (`view-library`):
- `spec_view(graph, markers, oversized)`: the `spec` model, `oversized` the optional size report (`spec-view`); `flight_view(graph, markers, refs, now, expiry_hours)` and `blockers_view(graph, markers, refs, now, expiry_hours)`: the `flight` and `blockers` models, `refs` the claim refs (`ClaimRef`) with their tip times and whether each is merged (`flight-view`, `blockers-view`); `queue_states(&flight)`: the queue states the `spec` chart and SVG show. `QueueView` is the `queue` model, which `orchestrator.loop` builds (`queue-view`).
- `render(&view, format, width, head)`: a `View` (`Spec` with its queue states, `Flight`, `Blockers` or `Queue`) as a `Format` (`Text`, `Ansi`, `Svg` or `Json`), the charts cut to `width` columns, beginning with `head`, a `Checkout` or the `At` of `--at`, when one is given (`view-command`). `DEFAULT_WIDTH` is `clm view`'s default width, and `Name` the views `view.repo` reads.
- `status_text(graph, markers, refs, backlog_refs, now, oversized, checkout)`: the whole of `clm status` (`status-report`).

## MODIFIED Requirements

### Requirement: status-report <!-- base: cddca12131e399c3 -->
`clm status` SHALL print, for the graph, markers, claim refs, checkout and size report it is handed, which `view.repo` reads from the repository (`view.repo/reads-the-repository`), a summary and then its sections: `components` and `changes` (`check.coverage/status-report`), each `components` row gaining the cell `oversized` when the component is oversized (`oversized-is-a-state`), then `claims` (`orchestrator/status-claims`). It SHALL hold no review section: nothing is reviewed after it lands (`yes-no-review`). The summary is the line `CLM status` followed by these lines, each indented two spaces:
- the checkout line, when it is handed a checkout: `commit <hash>`, the checkout's commit, followed by two spaces and `⚠ <n> behind origin/main` when the checkout's count `n` of commits behind `origin/main` is given and non-zero (`views-name-their-commit`);
- `components <n>: <a> active, <s> unbuilt; <current>/<total> requirements current`, counting the capabilities that don't hold `#### Untested` with the requirements (`model/human-layers`), followed by `, architecture <e>/<t> enforced` when `ARCHITECTURE.md` holds entries, `<t>` the entries holding an `#### Enforced by` block and `<e>` those of them `check.coverage/coverage-report` gives as current, then `, <u> untested` when any entry of either layer holds `#### Untested`, an entry holding both blocks counting in `<t>` and in `<u>`, then `, <k> oversized` when any component is oversized (`human-layer-proof`);
- `changes <n> live`, followed by `: ` and `<count> <status>` for each status that has changes outside the archive, in the order `implementing`, `applied`, `approved`, `proposed`, separated by `, `, then `, <count> complete` when any live change is complete;
- `pending <n> components`, followed by `: ` and `<count> <state>` for each of `claimed`, `ready` and `blocked` that has components, separated by `, `, counting each component in the state `orchestrator/queue-from-refs` gives it with the default expiry;
- `waiting on you <n>`, followed by `: ` and what waits on the human: every `question` and `human-task` backlog item as `backlog/<slug>`, whether or not it blocks a requirement, and the `proposed` and `approved` changes outside the archive as `changes/<id>`, sorted, separated by `, `, when there are any;
- `backlog <n>`, followed by `: ` and `<count> <kind>` for each kind that has items, in the order bug, deferred, idea, question, human-task, then any other, separated by `, `;
- `claims <n> live`, followed by `, <count> lapsed` and `, <count> merged` when there are any.

A blank line follows the summary and each section. A section is its name on a line, then its rows, each indented two spaces, or four under a group's name, which is itself indented two. A section with no rows has the single row `none`. The cells of a section's rows are separated by at least two spaces, each column starting at the same position on every row of the section. The command's exit status is `view.repo/arguments-and-exit`'s.

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

#### Scenario: nothing in flight
- GIVEN no changes outside the archive and no claim ref
- WHEN `clm status` runs, and then the status is built for a graph that holds no component
- THEN the `changes` section has the single row `none`, and so has the `claims` section; the second's `components` section has the single row `none`

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
- model/parse-backlog
- check.coverage/state-names

### Requirement: view-command <!-- base: 9fec4d395186def9 -->
`clm view <name>` SHALL print the `spec`, `flight`, `blockers` or `queue` view of the graph, markers, claim refs and checkout it is handed, which `view.repo` reads from the repository (`view.repo/reads-the-repository`). `--format` SHALL select the form: `text` (the default), a chart in box-drawing characters with no escape sequences; `ansi`, the same characters with ANSI colour escapes added; `svg`, a standalone SVG document; and `json`, the view's model. In `text` and `ansi`, no line SHALL be wider than the columns `--width` gives it, a width of 0 cutting no line, and every state the chart shows SHALL be written as a word or symbol, not only as a colour. In `ansi`, each state word SHALL be in the colour `colour-meanings` gives it, each opened by its escape and closed by `\x1b[0m`: `active`, `converged`, `current` and `complete` green (`\x1b[32m`); `unbuilt` and `ready` sky (`\x1b[36m`); `claimed` and `behind` amber (`\x1b[33m`); `blocked`, `obsolete` and `deprecated` rose (`\x1b[31m`); and each heading indigo (`\x1b[34m`); the SVG SHALL draw them in the same colours. The SVG SHALL escape the repository's text, so it parses as XML whatever that text holds. When it is handed a checkout, the `text` and `ansi` charts SHALL begin with the checkout line as `status-report` defines it, its `⚠` part amber in `ansi`, the `svg` document SHALL hold the same words, and the `json` model SHALL carry a top-level `checkout` with the line's `commit` and, when the checkout's count is given, that count as `behind`, 0 included (`views-name-their-commit`). When it is handed a commit in place of a checkout, as `clm view <name> --at <rev>` hands it (`view.repo/view-at-rev`), the `text` and `ansi` charts SHALL begin instead with `<rev> <commit>`, then ` · fetched <age> ago` when the time of a fetch is given, `<age>` the time from it to the current time it is handed, in whole minutes as `<n>m` under an hour and in whole hours as `<n>h` from then on, and `0m` when the fetch is later than that time, then ` · checkout <branch>`, or ` · checkout detached` when no branch is given, and no `⚠`; the `svg` document SHALL hold the same words, and the `json` model SHALL carry a top-level `at` with `rev`, `commit`, and `fetched` and `branch` when given, and no `checkout`. The command's default width, its usage errors and its exit status are `view.repo/arguments-and-exit`'s.

#### Scenario: plain chart
- GIVEN a repository with components `a` and `b`, `b` depending on `a`
- WHEN `clm view spec` runs
- THEN it exits 0 and its output contains `a` and `b` and no escape character

#### Scenario: coloured chart
- GIVEN the same repository
- WHEN `clm view spec --format ansi` runs
- THEN its output with the ANSI escapes removed is the output of `clm view spec`

#### Scenario: svg
- GIVEN the same repository, then one with a component owned by `<o&o>`, and then one whose title holds U+0001
- WHEN `clm view spec --format svg` runs
- THEN its output parses as XML with root element `svg` and contains the text `a` and `b`; the second and third parse as XML too, and the second holds the text `<o&o>`

#### Scenario: no width
- GIVEN a component whose ID is 60 characters long
- WHEN the `spec` view is rendered as `text` and as `ansi` with a width of 0
- THEN the component's line holds its whole ID and no line holds `…`

#### Scenario: colours
- GIVEN component `a` (`active`) with an uncited requirement and a live claim ref, component `b` whose only pending requirement is in the `blocks` of `backlog/q`, component `c` with an uncited requirement that nothing blocks, and a checkout three commits behind `origin/main`
- WHEN the `spec` view is rendered with the checkout as `ansi` and as `svg`, and the `flight` view as `ansi`
- THEN in `ansi`, `claimed` and `⚠ 3 behind origin/main` each lie between `\x1b[33m` and `\x1b[0m`, `blocked` between `\x1b[31m` and `\x1b[0m`, `ready` between `\x1b[36m` and `\x1b[0m`, `active` between `\x1b[32m` and `\x1b[0m`, and the flight chart's heading `components` between `\x1b[34m` and `\x1b[0m`; and the SVG draws `claimed` and the `⚠` part in amber and `blocked` in rose

#### Scenario: checkout behind on the chart
- GIVEN a checkout three commits behind `origin/main`
- WHEN the `flight` view is rendered with it as `text`, `svg` and `json`, and then the `spec` view as `json` with a checkout whose count is 0
- THEN the chart's first line starts `commit ` and ends `⚠ 3 behind origin/main`, the SVG contains the text `3 behind origin/main`, the first JSON's `checkout` has `behind` 3 and the checkout's `commit`, and the last JSON's `checkout` has `behind` 0

#### Scenario: at a commit
- GIVEN the commit `abc1234` named `origin/main`, fetched 200 seconds ago, with the branch `impl/a` checked out
- WHEN the `flight` view is rendered with it as `text`, `ansi`, `svg` and `json`, and then as `text` with a fetch 3 hours and 59 minutes ago and no branch, with no fetch time and no branch, and with a fetch 10 seconds after the current time and no branch
- THEN the first chart's first line is `origin/main abc1234 · fetched 3m ago · checkout impl/a`, the `ansi` chart's first line holds no `⚠`, the SVG holds the same words, and the JSON has no `checkout` and an `at` with `rev` `origin/main`, `commit` `abc1234`, that fetch time as `fetched` and `branch` `impl/a`; the later charts' first lines are `origin/main abc1234 · fetched 3h ago · checkout detached`, `origin/main abc1234 · checkout detached` and `origin/main abc1234 · fetched 0m ago · checkout detached`

#### Serves
- capability/see-project-state

### Requirement: spec-view <!-- base: 2904c6a242028d74 -->
The `spec` view SHALL hold every component exactly once, nested under its parent by ID, with its title, its status written `active`, `unbuilt` or `deprecated`, owners, `depends_on`, and its requirements counted by coverage state: `current`, `behind`, `unbuilt` and `total` for the requirements the component holds, the counts of the component `model/capability-tests-component` names as the one the capabilities are pending on including the capabilities that don't hold `#### Untested` (`check.coverage/pending-report`), and no component's counting the architecture entries, and `obsolete` for those a live change removed from it that a marker still cites, together with each requirement removed with its component that a marker in a file the component's `paths` own still cites, or, for a marker in a file no component's `paths` own, the root (`node-removal`). Its chart SHALL show each component below its parent, indented one level further, and its SVG inside its parent's box, each with those facts, its requirements as a bar segmented by state with `<current>/<total>` and `✕<obsolete>` when there are any, and, when the component isn't `converged`, the queue state the `flight` model gives it, coloured as the `flight` chart colours it. The chart's bar SHALL be ten cells shared in proportion among the component's current (`█`), behind (`▓`) and unbuilt (`░`) requirements, each non-zero count getting at least one cell and the rest going by largest remainder, ties in the order current, behind, unbuilt, or ten `·` when it has none. When a size report is given, each component SHALL carry `oversized`, true when the report says so, and the chart SHALL show `⚠ oversized` on such a component's line and the SVG `⚠` in its label with an amber outline unless a queue state colours the outline; without a report, `oversized` SHALL be absent and nothing marked (`oversized-is-a-state`).

#### Scenario: entries on the root
- GIVEN root `clm` with no requirements, `capability/b` cited by nothing, and `architecture/a` enforced by a rule of another component
- WHEN the `spec` view is built
- THEN the root's counts are 1 `unbuilt` of 1 `total`, and no component `capability` or `architecture` appears

#### Scenario: nested nodes
- GIVEN components `clm` (the root), `check` and `check.drift`, where `check.drift` depends on `model`, owned by `@a`, with 2 requirements, one current and one cited only at an older version
- WHEN `clm view spec --format json` runs
- THEN `check.drift` has parent `check`, `check` has parent `clm`, and `check.drift` has status `unbuilt`, owners `["@a"]`, `depends_on` `["model"]`, current 1, behind 1, unbuilt 0, obsolete 0 and total 2, and no other count

#### Scenario: tree chart
- GIVEN the same components
- WHEN `clm view spec` runs
- THEN the line for `check.drift` follows the line for `check` and is indented further, and holds a bar of `█` cells then `▓` cells, `1/2`, `@a` and `model`

#### Scenario: a node behind its spec is not full
- GIVEN component `a` with 4 requirements, 3 current and 1 cited only at an older version
- WHEN `clm view spec` runs
- THEN the line for `a` holds `3/4` and a bar with at least one `▓` cell and no `░` cell

#### Scenario: bar cells
- GIVEN component `a` with 1 current, 1 behind and 1 unbuilt requirement, `b` with 98 current, 1 behind and 1 unbuilt, `c` with 2 unbuilt, and `d` with none
- WHEN `clm view spec` runs
- THEN their lines hold `████▓▓▓░░░ 1/3`, `████████▓░ 98/100`, `░░░░░░░░░░ 0/2` and `·········· 0/0`

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

### Requirement: flight-view <!-- base: d01ad49ea8c016a3 -->
The `flight` view SHALL hold every component that isn't `converged`, sorted by effective priority, highest first, then by ID, each with the state `orchestrator/queue-from-refs` gives it, for a claimed component the hours since its claim's last push and the hours until the claim lapses, each to the nearest hour, a half rounding up, the first 0 when the push is after the current time, and its pending requirements with their states; and every change outside the archive whose status isn't `done` or `abandoned`, grouped by status in the order `implementing`, `applied`, `approved`, `proposed`, each group sorted by priority, highest first, then by ID, each with its requirements current out of the requirements its deltas add or modify, the markers still citing requirements it removed, and whether it is complete, each exactly as `check.coverage/change-progress` gives it for that change: an `applied` or `implementing` change leaves out requirements the spec no longer holds, and the markers it lists cite only removed requirements the spec no longer holds. It SHALL also hold totals: pending components by state, live changes, and requirements still to make current. Its chart SHALL begin, after the head line when it is handed one (`view-command`), with a line holding `flight` and the summary `<n> components pending`, followed, when any is pending, by ` (` and `<count> <state>` for each of `claimed`, `ready` and `blocked` that has components, separated by `, `, and `)`, then `  ·  <n> changes live  ·  <n> requirements to do`. Each component's line SHALL hold, after its ID, its priority, its state, a bar of one cell per pending requirement (`▓` behind, `░` unbuilt, `✕` obsolete), ten at most and then `…`, followed by ` <n> pending`, and then `pushed <h>h ago, lapses in <h>h` when it is claimed, or `by ` and its blockers, each `backlog/<slug>` or a requirement's ID, separated by `, `, when it is blocked. Each change's line SHALL hold, after its ID, its priority, a bar of ten cells, `█` for its share of requirements current, rounded up, and `░` for the rest, or ten `·` when it has none, followed by ` <current>/<total>`, and then `complete` when it is complete, or `<n> obsolete`, the markers still citing requirements it removed, when there are any.

#### Scenario: claimed node
- GIVEN component `a` with two uncited requirements, claimed by a branch last pushed an hour ago
- WHEN `clm view flight --format json` runs, and then with the branch pushed 3620 seconds ago, and with it pushed 60 seconds after the current time
- THEN `a` is `claimed`, pushed 1 hour ago and lapsing in 3 hours, with its two requirements `unbuilt`; the second is pushed 1 hour ago and lapsing in 3 hours, and the third pushed 0 hours ago and lapsing in 4 hours

#### Scenario: the flight summary
- GIVEN component `a` with the uncited requirement `a/p`, claimed by a branch pushed an hour ago; component `b` whose only pending requirement needs `a/p`; component `c` whose only pending requirement is in the `blocks` of `backlog/q`; component `d` with 12 uncited requirements that nothing blocks; and an `applied` change `e` whose delta removes `n/x`, which no component holds and a test in a file `d`'s `paths` own still cites
- WHEN `clm view flight` runs
- THEN its first line holds `flight` and `4 components pending (1 claimed, 1 ready, 2 blocked)`; `a`'s line holds `claimed` and `pushed 1h ago, lapses in 3h`, `b`'s holds `blocked` and `by a/p`, `c`'s holds `by backlog/q`, `d`'s bar is ten cells and `…`, followed by ` 13 pending`, and `e`'s line holds `1 obsolete`

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
- THEN `d` lists the marker `t.rs:3` and is not complete

#### Needs
- check.coverage/change-progress
- check.coverage/state-names

### Requirement: blockers-view <!-- base: 274e56fe36d0cc5b -->
The `blockers` view SHALL hold every component `orchestrator/queue-from-refs` reports as `blocked`, grouped under each root blocker its chain of waits reaches: a component that is `ready` or `claimed`, a backlog item, or a cycle of needs (below). A component blocked by a requirement it needs (`requirement-needs`) waits on that requirement's component when the requirement is unblocked, and on the requirement's own blockers when it is blocked, and the chain continues from there. A component blocked by an obsolete requirement of its own that is also pending on other components for the markers in their files (`check.coverage/pending-report`, which gives that line its own ID to wait on) waits on each of those components, taken from the lines `check.coverage/wait-lines` gives for its line; and every wait is followed to the line it names, never by a requirement's ID alone, which an obsolete requirement's several lines share. A chain of waits that comes back to a line it passed (a requirement on a component) is a cycle of needs, a violation the library still takes: the components whose lines form the cycle SHALL be one root, named by the first of their IDs in byte order, with the state `blocked`, holding up the cycle's other components and every component whose chain of waits reaches the cycle. For each root it SHALL hold its weight, the number of distinct components it holds up directly or through other components, and the components between it and each one. Roots that wait on the human (a `question` or `human-task` backlog item) SHALL come first, then the rest, each by weight, highest first, then by name.

#### Scenario: chain weight
- GIVEN component `c` whose only pending requirement needs `b/y`, component `b` whose only pending requirement `b/y` needs `a/x`, and `a/x` uncited on claimed component `a`
- WHEN `clm view blockers --format json` runs
- THEN the only root is `a`, with weight 2, holding up `b` and `c`

#### Scenario: an obsolete requirement pending on two components
- GIVEN a root with `wiring: []`, components `a` and `b`, an `applied` change that removed `b/gone`, markers citing it in a file of `b` and in a file of `a`, and nothing else pending
- WHEN `clm view blockers --format json` runs
- THEN the only root is `a`, with weight 1, holding up `b`

#### Scenario: an obsolete requirement pending on three components
- GIVEN a root with `wiring: []`, components `a`, `b` and `c`, an `applied` change that removed `b/gone`, markers citing it in a file of each, and nothing else pending
- WHEN `clm view blockers --format json` runs
- THEN the roots are `a` and `c`, each with weight 1, holding up `b`

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

#### Scenario: a cycle of needs
- GIVEN `a`'s only requirement `a/x` needing `b/x`, `b`'s only requirement `b/x` needing `a/x`, both uncited, and component `c` whose only pending requirement needs `a/x`
- WHEN the `blockers` model is built from it
- THEN the only root is `a`, with the state `blocked` and weight 2, holding up `b` and `c`

#### Needs
- check.coverage/pending-report
- check.coverage/wait-lines
- model/parse-backlog

### Requirement: view-library <!-- base: 645f94e9a8449bda -->
The `spec`, `flight` and `blockers` models SHALL be library calls of `clm-view` that touch no file, process or network: the `spec` model takes the spec graph, the markers and an optional size report; the `flight` and `blockers` models take the spec graph, the markers, the claim refs with their tip times and whether each is merged into the default branch, the current time and the expiry. A merged ref isn't a claim (`merged-claims-released`). The models SHALL carry no checkout: the renderer adds the one it is handed at the top level of the JSON (`view-command`), and that `clm view <name> --format json` prints the library's model for the repository is `view.repo/arguments-and-exit`'s. With its default features disabled, `clm-view` SHALL build for `wasm32-unknown-unknown`, models and renderers alike.

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

### Requirement: blockers-root-lines <!-- base: 8d9bf00989954134 -->
In `text` and `ansi`, the blockers chart SHALL hold a line whose text is `blockers`, followed by the single line `nothing is blocked` when there is no root, or else, for each root in the view's order, the root's line and then the lines of its tree. A root's line SHALL start at the first column with the root's name (a component ID or `backlog/<slug>`) followed by a space or by the end of the line, or a non-empty prefix of it followed by `…` when it doesn't fit; every line of its tree SHALL start with `├`, `└`, `│` or a space. A root's line SHALL hold, after its name, its state (a component's `ready`, `claimed` or `blocked`, or an item's kind), then `waits on you` when it waits on the human, then `holds up <weight>`, its cells separated by at least two spaces. No other line SHALL follow the heading. With ANSI escapes removed, the `ansi` form SHALL have the same lines.

#### Scenario: root lines
- GIVEN component `c` whose only pending requirement needs `b/y`, component `b` whose only pending requirement `b/y` needs `a/x`, `a/x` uncited on claimed component `a`, and component `d` whose only pending requirement is in the `blocks` of `backlog/q` of kind `question`
- WHEN `clm view blockers --format ansi` runs
- THEN with escapes removed, the lines after `blockers` are `backlog/q`'s line, which holds `question`, `waits on you` and `holds up 1`, one line starting `└` naming `d`, `a`'s line, which holds `claimed` and `holds up 2` and no `waits on you`, and the lines `├─ b` and `└─ c`, in that order: `c`'s need `b/y` is itself blocked, so `c` reaches `a` directly and both hang off `a`

#### Scenario: nothing blocked
- GIVEN no component blocked
- WHEN `clm view blockers` runs
- THEN the line after `blockers` is `nothing is blocked`, and no line follows it

### Requirement: flight-component-lines <!-- base: ace293679d8943bf -->
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

### Requirement: queue-view <!-- base: ba511898c222e0c0 -->
`clm-view` SHALL offer the `queue` model, a plain type with the sections `running`, `next`, `waiting_on_you` and `waiting_on_work`, each a list of entries carrying a name, a kind, and the fields its section shows: for a running entry its model, round, how long it has run and whether it is held by a session taken over; for a next entry its priority and the reason for its place; for an entry waiting on the human what it is, since when, and what the human does about it; for an entry waiting on work what it waits on. Its `text` and `ansi` charts SHALL begin with the checkout line, then a line `loop held, renewed <n>m ago`, `loop not held` or `stop requested: drain|now`, then each section under its heading (`running`, `next up`, `waiting on you`, `waiting on work`) with `  none` under an empty one, one entry per line starting with two spaces and its name, in the model's order, then its cells, separated by at least two spaces: a running entry's model, `round <n>`, how long it has run and, when a session taken over holds it, `taken over`; a next entry's priority and reason; for an entry waiting on the human what it is, `since <age>` and what the human does about it; for an entry waiting on work what it waits on; each of these times `<n>m` in whole minutes under an hour and `<n>h` in whole hours from then on. In `ansi`, `taken over`, `loop not held` and `stop requested: drain|now` SHALL be amber, and each entry line under `waiting on you` SHALL be amber from its name to its end, the padding between its cells included, so what waits on the human catches the eye (`colour-meanings`), while the heading stays a heading and `  none` stays plain; `json` SHALL be the model with `"view": "queue"`.

#### Scenario: chart sections
- GIVEN a queue model with running `build:a` on `sonnet` in round 1 for 12 minutes and `build:c` on `opus` in round 2 for 61 minutes, held by a session taken over, next `build:b` at `P2` waiting for a place, a fold batch `x` waiting on the human for 90 minutes, and nothing waiting on work
- WHEN it is rendered as `text`
- THEN the lines after the checkout line are the loop line, `running`, `  build:a` with `sonnet`, `round 1` and `12m`, `  build:c` with `opus`, `round 2`, `1h` and `taken over`, `next up`, `  build:b` with `P2` and `waits for a place`, `waiting on you`, `  fold:x` with `since 1h` and `say "approve x"`, `waiting on work` and `  none`

#### Scenario: waiting on you highlighted
- GIVEN the same queue model, and then one with `loop not held`
- WHEN it is rendered as `ansi`
- THEN every character of the line holding `fold:x`, from `fold:x` to its end, lies between `\x1b[33m` and `\x1b[0m`, the lines holding `build:a` and `build:b` hold no `\x1b[33m` before their names, `taken over` and the second's `loop not held` each lie between `\x1b[33m` and `\x1b[0m`, and with the escapes removed the lines are those the `text` chart gives

#### Serves
- capability/watch-beside-chat
- capability/build-from-spec

## REMOVED Requirements

### Requirement: says-component <!-- base: f1f9079e23fd1d8b -->
