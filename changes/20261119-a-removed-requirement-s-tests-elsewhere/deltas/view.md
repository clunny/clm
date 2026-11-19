---
component: view
base: {}
---

## MODIFIED Requirements

### Requirement: blockers-view <!-- base: 3950c6903a572505 -->
The `blockers` view SHALL hold every component `queue_from_refs` reports as `blocked`, grouped under each root blocker its chain of waits reaches: a component that is `ready` or `claimed`, or a backlog item. A component blocked by a requirement it needs (`requirement-needs`) waits on that requirement's component when the requirement is unblocked, and on the requirement's own blockers when it is blocked, and the chain continues from there. A component blocked by an obsolete requirement of its own that is also pending on other components for the markers in their files (`check.coverage/pending-report`, which gives that line its own ID to wait on) waits on each of those components, taken from every line `pending` returns for the ID and not from one of them. For each root it SHALL hold its weight, the number of distinct components it holds up directly or through other components, and the components between it and each one. Roots that wait on the human (a `question` or `human-task` backlog item) SHALL come first, then the rest, each by weight, highest first, then by name.

#### Scenario: chain weight
- GIVEN component `c` whose only pending requirement needs `b/y`, component `b` whose only pending requirement `b/y` needs `a/x`, and `a/x` uncited on claimed component `a`
- WHEN `clm view blockers --format json` runs
- THEN the only root is `a`, with weight 2, holding up `b` and `c`

#### Scenario: an obsolete requirement pending on two components
- GIVEN a root with `wiring: []`, components `a` and `b`, a live change that removed `b/gone`, markers citing it in a file of `b` and in a file of `a`, and nothing else pending
- WHEN `clm view blockers --format json` runs
- THEN the only root is `a`, with weight 1, holding up `b`

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
- plugin.panel/component-inspector
- model/parse-backlog
