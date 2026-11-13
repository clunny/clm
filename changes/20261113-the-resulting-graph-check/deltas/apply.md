---
component: apply
---

## MODIFIED Requirements

### Requirement: resulting-graph <!-- base: 8282f3f17e44be7a -->
Once every delta of the change is otherwise valid (`node_writes` reports nothing, whatever the change's status), validation SHALL build in memory the spec graph the change would leave: each component the change writes parsed from its new `SPEC.md` text by the model's component parser (`model/graph-check`), as loading parses it, with the keys live changes declare (`declared-frontmatter-keys`), at the path it is written to and with no assets, which no rule across components reads; each component it removes gone; each backlog item `clm apply` deletes with the change gone (`resulting-graph-call`); every other component and backlog item, and the changes and decisions, as they are. It SHALL run the model's graph check on the current graph, with every item, and on that graph with the backlog's `workaround` items left out (the graph `resulting` returns keeps them, `resulting-graph-call`), and the component parser on the current text of each existing component the change writes. A flag's `requirement` holds the version a build built (`model/parse-backlog`), which only the current graph gives: a flag on a requirement the change rewrites or removes stays valid while the change is pending, and once the change is applied, loading reports the flag on the branch that applied it, which deletes it or re-records it at the new version (`workaround-flags`). Each violation of the resulting graph, from the parser or the graph check, that the current graph doesn't have (the same path, rule and message) SHALL be a violation under `apply/resulting-graph`, with the model's path and the message ``change `<id>` would leave: <model rule>: <model message>``; nothing SHALL be written. Violations the current graph already has are not reported again, so `clm check`, which validates every `proposed` or `approved` change against a graph that may have violations of its own, reports each once. A violation whose message the change alters, such as a dependency cycle it lengthens, counts as new. `reapply`, which `clm rebase` uses to rebuild `specs/`, doesn't run this rule: on a newer `main`, the load after the rebuild reports what breaks.

#### Scenario: required node depends on a node made optional
- GIVEN component `a`, component `b` with `depends_on: [a]`, and change `x` whose delta for `a` sets `optional: true` in its `## MODIFIED Frontmatter`, with the correct base
- WHEN `clm apply x` runs
- THEN it exits 1 naming `apply/resulting-graph`, `model/optional-nodes`, `b` and `a`, and nothing is written

#### Scenario: dependant left behind
- GIVEN component `a`, component `b` with `depends_on: [a]`, and change `x` whose only delta is a valid removal of `a`
- WHEN `clm apply x` runs
- THEN it exits 1 naming `apply/resulting-graph`, `model/dependency-dag`, `b` and `a`, and `specs/a/SPEC.md` still exists

#### Scenario: backlog blocks a removed requirement
- GIVEN component `a` with requirements `x` and `y`, a backlog item whose `blocks` names `a/y`, and change `c` whose delta for `a` removes `y` with its base
- WHEN `clm apply c` runs
- THEN it exits 1 naming `apply/resulting-graph`, `model/parse-backlog` and `a/y`, and nothing is written

#### Scenario: answered question blocks a removed requirement
- GIVEN component `a` with requirements `x` and `y`, `backlog/q.md` with `kind: question` and `blocks: [a/y]`, and a proposed change `c` whose `resolves` is `[q]` and whose delta for `a` removes `y` with its base
- WHEN `clm check` runs, then `clm apply c`
- THEN `clm check` reports nothing under `apply/resulting-graph`, and `clm apply c` exits 0, removes `a/y` and deletes `backlog/q.md`

#### Scenario: resolved bug blocks a removed requirement
- GIVEN component `a` with requirements `x` and `y`, `backlog/b.md` with `kind: bug` and `blocks: [a/y]`, and change `c` whose `resolves` is `[b]` and whose delta for `a` removes `y` with its base
- WHEN `clm apply c` runs
- THEN it exits 1 naming `apply/resulting-graph`, `model/parse-backlog` and `a/y`, and `backlog/b.md` still exists

#### Scenario: violation already present
- GIVEN component `b` with `depends_on: [gone]`, naming no component, and a proposed change `x` whose valid delta modifies `a`'s Design
- WHEN `clm check` runs
- THEN it reports `b`'s `depends_on` once, under `model/dependency-dag`, and nothing under `apply/resulting-graph`

#### Scenario: workaround flag on a rewritten requirement
- GIVEN component `a` whose requirement `x` is at version `3fa2c9d1`, `backlog/workaround-a-x-1.md` with `kind: workaround` and `requirement: a/x@3fa2c9d1`, and a proposed change `c` whose delta for `a` modifies `x`'s text with its base
- WHEN `clm check` runs, then `clm apply c`, then `clm check` again
- THEN the first `clm check` reports nothing, `clm apply c` exits 0 and writes `x`'s new text, and the second `clm check` reports `backlog/workaround-a-x-1.md` under `model/parse-backlog` with `x`'s new version

#### Serves
- capability/catch-drift
- architecture/deterministic-enforcement

#### Needs
- model/graph-check

### Requirement: resulting-graph-call <!-- base: 06d5a9cc73e0d95f -->
`resulting(graph, change)` SHALL return the spec graph `resulting-graph` builds for the change: each component the change writes parsed from its new `SPEC.md` text at the path it is written to, each component it removes gone, each backlog item `clm apply` deletes with the change gone, and every other component and backlog item, and the changes, decisions and human layers, as in `graph`; or none when a delta of the change can't be applied (`node_writes` reports a violation), whatever the change's status. The items `clm apply` deletes with the change are the `question` items its `resolves` names (`answered-questions`) and the item its `from` names unless that is a `human-task` (`promoted-item`). `clm apply` and `resulting` SHALL take them from one function both call (which may be private to the crate), so the graph a change is checked against never holds an item its apply deletes. A written component whose new text the parser yields no component for is left out of the graph, as `resulting-graph` leaves it, which reports the parse. It SHALL write nothing. In Rust it is `pub fn resulting(graph: &Graph, change: &Change) -> Option<Graph>`, and `plan` builds the graph it checks through it.

#### Scenario: graph a removal leaves
- GIVEN component `a` with requirements `x` and `y`, and change `c` whose delta for `a` removes `y` with its base
- WHEN `resulting(graph, c)` is called
- THEN it returns a graph whose component `a` holds `a/x` and not `a/y`, with the source `clm apply c` would write to `specs/a/SPEC.md`, and that file is unchanged

#### Scenario: delta that can't apply
- GIVEN change `c` whose delta for `a` holds a stale base
- WHEN `resulting(graph, c)` is called
- THEN it returns none

#### Scenario: items apply deletes
- GIVEN `backlog/q.md` with `kind: question`, `backlog/b.md` with `kind: bug`, `backlog/h.md` with `kind: human-task`, `backlog/i.md` with `kind: idea`, and change `c` with `resolves: [q, b, h]`, `from: backlog/i` and a valid delta
- WHEN `resulting(graph, c)` is called
- THEN it returns a graph whose backlog holds `b` and `h` and neither `q` nor `i`, and all four files still exist

#### Scenario: promoted human task stays
- GIVEN `backlog/h.md` with `kind: human-task`, and change `c` with `from: backlog/h` and a valid delta
- WHEN `resulting(graph, c)` is called
- THEN it returns a graph whose backlog holds `h`

#### Serves
- capability/catch-drift
