---
component: view
base: { contract: "1984f1c023dc8320", design: "2f82cd05e68b3e22" }
---

## MODIFIED Contract
Views of a CLM repository for people to read (`views-rendered-by-clm`). `clm view <name>` prints one of four views:
- **`spec`**: the architecture. Every component nested under its parent, with its status, its requirements counted by coverage state and drawn as a bar segmented by state with `n/m` current of total, owners and `depends_on` edges; its chart and SVG also show each pending component's queue state, and mark a component that is over its size budget (`oversized-is-a-state`).
- **`flight`**: the work in flight. Every component with pending work and its queue state (`ready`, `claimed`, `blocked`), who holds its claim and for how long, and its pending requirements; then every live change with its requirements current of those its deltas add or modify, and whether it is complete.
- **`blockers`**: what holds work up. Every component that can't start, grouped under the blocker at the end of its chain of waits, ranked by how many components that blocker holds up, with what waits on the human first.
- **`queue`**: the build loop's plan (`orchestrator.loop`): what runs, what the loop launches next in its order with each entry's priority and why it is there, what waits on the human, and what waits on other work. It has no `svg` form.

`--format text` (the default) draws a chart in box-drawing characters for a terminal or the chat; `--format ansi` draws the same chart coloured with ANSI escapes; `--format svg` prints a standalone SVG document; `--format json` prints the view's model. `--width <columns>` fits the chart to a width. `clm status` prints a grouped summary of the repository: totals first, then components, changes and claims.

Both read the working tree and the local refs only, never fetch, and exit 0, or print the spec graph's violations and exit 1. Since `clm next` reads `origin/main` instead, both name the commit they describe and warn when it is behind `origin/main` as of the last fetch (`views-name-their-commit`): `clm status`'s summary and each chart begin with the checkout line `status-report` defines, and the JSON carries the same as `checkout`. The models and renderers are library calls without git or a checkout, taking what `queue_from_refs` takes, so the `web` Worker computes the same views, and the crate builds for `wasm32-unknown-unknown` (`view-library`). Reading the repository for the two commands, the working tree, the checkout and the refs, is `view.repo`'s, and so is `clm view <name> --at <rev>`, which describes `<rev>`'s commit instead of the working tree (`view.repo/view-at-rev`); the CLM pane always passes `--at origin/main` (`plugin.panel`).

## MODIFIED Design
Crate `clm-view`, with a model per view and a renderer per format. The commands are `view.repo`'s crate, `clm-view-repo`, which reads the repository and hands this crate's models and renderers what it read: `crates/clm` dispatches `clm view` and `clm status` to it, and the `clm-hooks` session-start hook prints its status. `clm_check` gives the `components` and `changes` rows of `clm status` as cells, and `clm_orchestrator` the `claims` rows, each proven by tests in its own crate; `clm-view` adds the summary and lays the sections out.

**Models.** `spec_view`, `flight_view` and `blockers_view` take the graph and markers as `clm_check::report` does, and the component states and pending requirements from `clm_orchestrator::queue_from_refs`; `status` is built from all three. The `spec` model is a function of the graph and markers alone, so the `web` app stores it in a snapshot; the queue state its chart and SVG show is the `flight` model's, computed from the same refs and handed to the renderer beside the model (a component the `flight` model doesn't list is `converged`), so `clm view spec` and `clm view flight` never disagree about a component. JSON is the serde form of the model, with snake_case keys and a top-level `"view"` naming the view; fields may be added, never renamed or removed without a spec change, since the `web` app and the plugin's panel read them.

**Checkout** (`views-name-their-commit`). `view.repo` reads the checkout through `clm-git` (`git-plumbing-layer`): `HEAD`'s hash is cut to 7 hex digits rather than taken from `--short`, which can lengthen. The line is cut to `--width` like the chart's own lines, its `⚠` part amber in `ansi` and the SVG (`colour-meanings`).

**Oversized components** (`oversized-is-a-state`). `spec_view` and `status` take an optional size report, `check`'s `size(...)` result, computed as `clm size` computes it; when the settings can't be loaded the command passes none and prints `size not checked: <error>` on stderr. The web app's `replica` passes none until it can compute one (`replica/snapshot-size`).

**Component detail** (`component-detail`, `panel-inspector-contract`). The inspector's fields ride on the `spec` model so the panel draws them from it rather than reading `specs/`, `changes/` or `backlog/` itself (`views-rendered-by-clm`). All three are functions of the graph and markers, so the model stays one the `web` app can store in a snapshot, at the cost of one more coverage pass in `spec_view`. The `web` Worker's shape check (`web.worker`'s `current_shape` in `crates/clm-web/src/snapshot.rs`) lists the fields, so it sees a snapshot stored before them as stale and rebuilds it; nothing deserialises the model, so they carry no serde default. The spec branches on `origin` that touch the component aren't here: `clm view` reads the working tree and the local refs, not each branch's tree.

**Blockers** (`blockers-view`). A component's waits are the edges `queue_from_refs`'s `blocked { by }` gives it; `depends_on` gives none (`needs-alone-order`). A need's edge goes on to the needed requirement's own blockers when that requirement is blocked, so the root is never a `ready` component whose session would leave the needed requirement out. A component waiting on several things appears under each root its waits reach. The `flight` model's components carry no `paths` and no `waiting_on`, and the view shows no `waiting` state.

**Charts.** Long names are cut with `…`. The spec chart is an indented tree drawn with `├─ └─ │`, each component's line holding, in order, its ID, status, queue state, `⚠ oversized`, bar and `n/m`, owners, and `→` with its dependencies. The bar is ten cells shared in proportion among the component's current (`█`), behind (`▓`) and unbuilt (`░`) requirements, each non-zero count getting at least one cell and the rest going by largest remainder, ties in the order current, behind, unbuilt. `spec_view` takes the obsolete counts from `clm_check::pending` rather than from the requirement's ID, so the spec chart, the `flight` view and `clm next` agree on which component holds a removed requirement (`node-removal`). The flight chart draws each component's pending requirements as a bar of behind, unbuilt and obsolete, and each change's requirements as a bar with `complete` where it applies. The pinned line shapes (`flight-component-lines`, `spec-component-lines`, `blockers-root-lines`) exist because the `clm-panel` plugin finds a component's line by its prefix to add its own rows beneath and presses lines by position, the nth such line being the model's nth component or root (`plugin.panel`'s `component-inspector` and `blocker-chains`); they change only with this spec. Colours, in `ansi` and SVG, are `colour-meanings`, shared with the `web` app. The SVG lays components out in layers, dependents above their dependencies, with children inside their parent's box; it embeds its own styles and fonts fall back to the system's. Each box holds its label, its bar as coloured segments and `n/m`; a `converged` component keeps the outline of its status.

**Size budget** (`slim-components`, `node-size-budgets`). The code budget is 55000 tokens, over the default 40000, because the three views and `clm status` share one chart and SVG shell (`chart.rs`, `svg.rs` and the head line every form begins with), which a split by view would leave in the parent while each child held a third of its callers.

**Queue** (`queue-view`). The `queue` model is a plain type this crate defines, with no git and no dependency on `orchestrator.loop`, which computes it (`plan`); `view.repo` hands it to the renderers, so imports run one way, `view` below `orchestrator.loop` below `view.repo`, and the crate still builds for `wasm32-unknown-unknown`. Its chart has no layout of its own beyond sections of lines, so it has no `svg` form: the web app shows no loop, which is one clone's.

## ADDED Requirements
### Requirement: queue-view
`clm-view` SHALL offer the `queue` model, a plain type with the sections `running`, `next`, `waiting_on_you` and `waiting_on_work`, each a list of entries carrying a name, a kind, and the fields its section shows: for a running entry its model, round, how long it has run and whether it is held by a session taken over; for a next entry its priority and the reason for its place; for an entry waiting on the human what it is, since when, and what the human does about it; for an entry waiting on work what it waits on. Its `text` and `ansi` charts SHALL begin with the checkout line, then a line `loop held, renewed <n>m ago`, `loop not held` or `stop requested: drain|now`, then each section under its heading (`running`, `next up`, `waiting on you`, `waiting on work`) with `  none` under an empty one, one entry per line starting with two spaces and its name, in the model's order; `json` SHALL be the model with `"view": "queue"`.

#### Scenario: chart sections
- GIVEN a queue model with running `build:a` on `sonnet` for 12 minutes, next `build:b` at `P2` waiting for a place, a fold batch `x` waiting on the human, and nothing waiting on work
- WHEN it is rendered as `text`
- THEN the lines after the checkout line are the loop line, `running`, `  build:a` with `sonnet` and `12m`, `next up`, `  build:b` with `P2` and `waits for a place`, `waiting on you`, `  fold:x` with `say "approve x"`, `waiting on work` and `  none`

#### Scenario: wasm
- WHEN `cargo build --target wasm32-unknown-unknown --no-default-features -p clm-view` runs
- THEN it builds

#### Serves
- capability/watch-beside-chat
- capability/build-from-spec

## MODIFIED Requirements
### Requirement: view-command <!-- base: 8e470e55284de994 -->
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
- architecture/views-name-their-commit

#### Needs
- plugin.panel/views-panel
