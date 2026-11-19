---
component: orchestrator
base: {}
---

## MODIFIED Requirements

### Requirement: priority-orders-components <!-- base: b239015b748feaa9 -->
A pending requirement's priority SHALL be the highest priority among the changes outside the archive whose deltas add, modify or remove it, or `P5` when there is none. A component's *effective priority* SHALL be the highest of the priorities of its unblocked pending requirements and of the priorities seeded on it: a pending requirement blocked by a wait seeds its priority on the component of each line it waits on, as `check.coverage`'s `waits` gives them (`check.coverage/wait-lines`), never on the component the first part of a waited ID names, and a line waited on that is itself blocked by a wait passes it on to the lines it waits on, through components of any state, so a `claimed` or `blocked` component in between passes a dependant's priority on. A wait SHALL be each entry of the requirement's `waits_on` (`check.coverage/pending-report`): a requirement its `Needs` names, for an `obsolete` requirement one a move took it to and, on its own component's line, its own ID, standing for its lines on the components that own the other files citing it, and for a capability a rule serving it that holds it back (an `unbuilt` one, or a `behind` one itself blocked), so the rules serving a capability, and the work a changed one waits on, inherit its priority. `depends_on` SHALL carry no priority (`needs-alone-order`). `queue_from_refs` SHALL report each component's effective priority, and `clm next` SHALL sort by it.

#### Scenario: urgent work pulls the node it needs forward
- GIVEN component `a` whose only pending requirement `a/x` comes from a change with `priority: P1` and needs uncited `b/y`, `b/y` from a change with no priority, and a pending component `c` on another part of the graph from a change with `priority: P3`
- WHEN `clm next` runs
- THEN it prints `b` then `c`, and not `a`

#### Scenario: passes through a claimed node
- GIVEN component `c` whose `P1` pending requirement `c/z` needs `b/y`, component `b` claimed, whose pending requirement `b/y` needs `a/x`, `a/x` uncited from a change with no priority, and a pending component `d` elsewhere at `P3`
- WHEN `clm next` runs
- THEN it prints `a` then `d`

#### Scenario: dependency lends no priority
- GIVEN component `a` depending on `b`, both pending, `a`'s pending requirement from a change with `priority: P1` and `b`'s from a change with no priority, and a pending component `c` elsewhere at `P3`
- WHEN `clm next` runs
- THEN it prints `a`, `c` then `b`

#### Scenario: blocked requirement doesn't lend its priority
- GIVEN component `b` whose `P1` pending requirement is in the `blocks` of an existing backlog item, and whose other pending requirement is `P5`, and component `c` pending at `P3`
- WHEN `clm next` runs
- THEN it prints `c` then `b`

#### Scenario: capability pulls its serving rules forward
- GIVEN component `capability` whose `capability/c` is uncited and comes from a change with `priority: P1`, component `w` with `serves: [capability/c]` holding `w/x` uncited from a change with no priority, and a pending component `d` elsewhere at `P3`
- WHEN `clm next` runs
- THEN it prints `w` then `d`, and not `capability`

#### Scenario: capability pulls a changed serving rule's needs forward
- GIVEN component `capability` whose `capability/c` is cited at an older version and comes from a change with `priority: P1`, component `w` with `serves: [capability/c]` holding `w/x` cited at an older version with `#### Needs` naming `b/y`, `b/y` uncited, both from changes with no priority, and a pending component `d` elsewhere at `P3`
- WHEN `clm next` runs
- THEN it prints `b` then `d`, and neither `capability` nor `w`

#### Scenario: an obsolete requirement pulls the owner of its other markers forward
- GIVEN a root with `wiring: []`, components `b` and `c` with `paths` `["crates/b/**"]` and `["crates/c/**"]`, a change with `priority: P1` that removed `b/gone`, markers citing `b/gone` in a file of each, `backlog/q.md` of kind `question` with `blocks: [b/gone]`, `c/y` uncited on `c` from a change with no priority, and a pending component `d` elsewhere at `P3`
- WHEN `clm next` runs
- THEN it prints `c` then `d`, and not `b`: `b/gone`'s line on `b` seeds `P1` on `c`, where its other line is, and not on `b`

#### Needs
- check.coverage/pending-report
- check.coverage/wait-lines
- view/blockers-view
- view/status-report
