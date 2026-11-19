---
component: view
base: { design: "e46af7236b5e9724" }
---

## MODIFIED Design
Crate `clm-view`, with a model per view and a renderer per format. The commands are `view.repo`'s crate, `clm-view-repo`, which reads the repository and hands this crate's models and renderers what it read: `crates/clm` dispatches `clm view` and `clm status` to it, and the `clm-hooks` session-start hook prints its status. `clm_coverage` gives the `components` and `changes` rows of `clm status` as cells, and `clm_orchestrator` the `claims` rows, each proven by tests in its own crate; `clm-view` adds the summary and lays the sections out.

**Models.** `spec_view`, `flight_view` and `blockers_view` take the graph and markers as `clm_coverage::report` does, and the component states and pending requirements from `clm_orchestrator::queue_from_refs`; `status` is built from all three. The `spec` model is a function of the graph and markers alone, so the `web` app stores it in a snapshot; the queue state its chart and SVG show is the `flight` model's, computed from the same refs and handed to the renderer beside the model (a component the `flight` model doesn't list is `converged`), so `clm view spec` and `clm view flight` never disagree about a component. JSON is the serde form of the model, with snake_case keys and a top-level `"view"` naming the view; fields may be added, never renamed or removed without a spec change, since the `web` app and the plugin's panel read them.

**Checkout** (`views-name-their-commit`). `view.repo` reads the checkout through `clm-git` (`git-plumbing-layer`): `HEAD`'s hash is cut to 7 hex digits rather than taken from `--short`, which can lengthen. The line is cut to `--width` like the chart's own lines, its `⚠` part amber in `ansi` and the SVG (`colour-meanings`).

**Oversized components** (`oversized-is-a-state`). `spec_view` and `status` take an optional size report, the IDs of the components `clm size` marks, which the command computes and hands in; when the command passes none (`view.repo/reads-the-repository`), nothing is marked. The web app's `replica` passes none until it can compute one (`replica/snapshot-size`).

**Component detail** (`component-detail`). The inspector's fields ride on the `spec` model so the panel draws them from it rather than reading `specs/`, `changes/` or `backlog/` itself (`views-rendered-by-clm`). All three are functions of the graph and markers, so the model stays one the `web` app can store in a snapshot, at the cost of one more coverage pass in `spec_view`. The `web` Worker's shape check (`web.worker`'s `current_shape` in `crates/clm-web/src/snapshot.rs`) lists the fields, so it sees a snapshot stored before them as stale and rebuilds it; nothing deserialises the model, so they carry no serde default. The spec branches on `origin` that touch the component aren't here: `clm view` reads the working tree and the local refs, not each branch's tree.

**Blockers** (`blockers-view`). A component's waits are the edges `queue_from_refs`'s `blocked { by }` gives it, each requirement edge followed to the lines `check.coverage`'s `waits` gives for the component's own lines, since an ID alone names no line (`a-wait-names-a-line`); `depends_on` gives none (`needs-alone-order`). A need's edge goes on to the needed requirement's own blockers when that requirement is blocked, so the root is never a `ready` component whose session would leave the needed requirement out. A component waiting on several things appears under each root its waits reach. The `flight` model's components carry no `paths` and no `waiting_on`, and the view shows no `waiting` state.

**Charts.** Long names are cut with `…`. The spec chart is an indented tree drawn with `├─ └─ │`, each component's line holding, in order, its ID, status, queue state, `⚠ oversized`, bar and `n/m`, owners, and `→` with its dependencies. The bar is ten cells shared in proportion among the component's current (`█`), behind (`▓`) and unbuilt (`░`) requirements, each non-zero count getting at least one cell and the rest going by largest remainder, ties in the order current, behind, unbuilt. `spec_view` takes the obsolete counts from `clm_coverage::pending` rather than from the requirement's ID, so the spec chart, the `flight` view and `clm next` agree on which component holds a removed requirement (`node-removal`). The flight chart draws each component's pending requirements as a bar of behind, unbuilt and obsolete, and each change's requirements as a bar with `complete` where it applies. The pinned line shapes (`flight-component-lines`, `spec-component-lines`, `blockers-root-lines`) exist because the `clm-panel` plugin finds a component's line by its prefix to add its own rows beneath and presses lines by position, the nth such line being the model's nth component or root (`plugin.panel`'s `component-inspector` and `blocker-chains`); they change only with this spec. Colours, in `ansi` and SVG, are `colour-meanings`, shared with the `web` app. The SVG lays components out in layers, dependents above their dependencies, with children inside their parent's box; it embeds its own styles and fonts fall back to the system's. Each box holds its label, its bar as coloured segments and `n/m`; a `converged` component keeps the outline of its status.

**Size budget** (`slim-components`, `node-size-budgets`). The code budget is 55000 tokens, over the default 40000, because the three views and `clm status` share one chart and SVG shell (`chart.rs`, `svg.rs` and the head line every form begins with), which a split by view would leave in the parent while each child held a third of its callers.

**Queue** (`queue-view`). The `queue` model is a plain type this crate defines, with no git and no dependency on `orchestrator.loop`, which computes it (`plan`); `view.repo` hands it to the renderers, so imports run one way, `view` below `orchestrator.loop` below `view.repo`, and the crate still builds for `wasm32-unknown-unknown`. Its chart has no layout of its own beyond sections of lines, so it has no `svg` form: the web app shows no loop, which is one clone's.

## MODIFIED Requirements

### Requirement: blockers-view <!-- base: fc8b00ac174dbab5 -->
The `blockers` view SHALL hold every component `queue_from_refs` reports as `blocked`, grouped under each root blocker its chain of waits reaches: a component that is `ready` or `claimed`, or a backlog item. A component blocked by a requirement it needs (`requirement-needs`) waits on that requirement's component when the requirement is unblocked, and on the requirement's own blockers when it is blocked, and the chain continues from there. A component blocked by an obsolete requirement of its own that is also pending on other components for the markers in their files (`check.coverage/pending-report`, which gives that line its own ID to wait on) waits on each of those components, taken from the lines `check.coverage`'s `waits` gives for its line (`check.coverage/wait-lines`); and every wait is followed to the line it names by `waits`, never by a requirement's ID alone, which an obsolete requirement's several lines share. For each root it SHALL hold its weight, the number of distinct components it holds up directly or through other components, and the components between it and each one. Roots that wait on the human (a `question` or `human-task` backlog item) SHALL come first, then the rest, each by weight, highest first, then by name.

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

#### Needs
- check.coverage/pending-report
- check.coverage/wait-lines
- plugin.panel/component-inspector
- model/parse-backlog
