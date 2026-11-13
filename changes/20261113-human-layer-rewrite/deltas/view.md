---
component: view
---

## MODIFIED Requirements

### Requirement: status-report <!-- base: 538240fba331e725 -->
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
- architecture/slim-components

#### Needs
- check.coverage/coverage-report
- check.coverage/status-report
- plugin.panel/component-inspector
- model/parse-backlog
- check.coverage/state-names
- plugin.hooks/session-status

### Requirement: view-command <!-- base: 0fb3094741049982 -->
`clm view <name>` SHALL print the `spec`, `flight`, `blockers` or `queue` view of the repository, from the working tree and the local refs only and without fetching, and exit 0, or print the spec graph's violations and exit 1. `--format` SHALL select the form: `text` (the default), a chart in box-drawing characters with no escape sequences; `ansi`, the same characters with ANSI colour escapes added; `svg`, a standalone SVG document; and `json`, the view's model. In `text` and `ansi`, no line SHALL be wider than `--width` columns (default 100), and every state the chart shows SHALL be written as a word or symbol, not only as a colour. When `HEAD` can be read, the `text` and `ansi` charts SHALL begin with the checkout line as `status-report` defines it, the `svg` document SHALL hold the same words, and the `json` model SHALL carry a top-level `checkout` with the line's `commit` and, when `refs/remotes/origin/main` exists, its count as `behind`, 0 included (`views-name-their-commit`). An unknown name or format, `png` among them, and `--format svg` for `queue`, SHALL exit 2 with usage.

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
- WHEN `clm view everything` runs, then `clm view spec --format png`, and then `clm view queue --format svg`
- THEN each exits 2

#### Scenario: checkout behind on the chart
- GIVEN a repository whose `HEAD` is three commits behind its remote-tracking ref `origin/main`, and an `origin` that can't be reached
- WHEN `clm view flight`, `clm view flight --format svg` and `clm view flight --format json` run, and then `clm view spec --format json` with `HEAD` at `origin/main`
- THEN each exits 0; the chart's first line starts `commit ` and ends `⚠ 3 behind origin/main`, the SVG contains the text `3 behind origin/main`, the first JSON's `checkout` has `behind` 3 and `commit` the hash of `HEAD` cut to its first 7 hex digits, and the last JSON's `checkout` has `behind` 0

#### Serves
- capability/see-project-state

#### Needs
- plugin.panel/views-panel
