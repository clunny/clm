---
component: web.views
base: {}
---

## MODIFIED Requirements

### Requirement: implementation-view <!-- base: 915d41a2fb902abc -->
The implementation view SHALL show the `view` component's `flight` model: the pending components in its order, each with the state `queue_from_refs` gives it, for a claimed component its last push and when the claim lapses, for a component whose branch has a page that page and whether the ledger holds a `checked` record for its head's code key, for a blocked component the backlog items marked as waiting on a human and the requirements it waits on (`requirement-needs`), each shown as its ID followed by ` on ` and the components the queue names in that blocker's `components` (`orchestrator/queue-from-refs`), in the order given, separated by `, ` and each linked to its component's entry, never to the component the first part of the ID names; a requirement blocker that carries no `components`, as in a view stored before the queue gave them, SHALL be shown as its ID alone, with no link, and its pending requirements by state, showing no `waiting` state and reading no `waiting_on` field (`needs-alone-order`), and taking a requirement's blockers from its `blocked_by` and `waits_on`, never from `needs_pending`; then the live changes in the model's groups and order, each with its requirements current of those it adds or modifies, and `complete` where it applies.

#### Scenario: claimed node
- GIVEN a pending component claimed by a branch last pushed an hour ago
- WHEN the view is shown
- THEN the component shows as claimed, lapsing in 3 hours

#### Scenario: change progress
- GIVEN an `applied` change whose deltas add two requirements, one current
- WHEN the view is shown
- THEN the change shows 1 of 2 requirements current and isn't marked complete

#### Scenario: blocked by a need
- GIVEN a pending component `a` whose only pending requirement needs `b/y`, uncited
- WHEN the view is shown
- THEN `a` shows as blocked by `b/y`

#### Scenario: no waiting field
- GIVEN a `flight` model whose pending component `a` is `ready` and has no `waiting_on` field
- WHEN the view is shown
- THEN `a` shows as ready, with no line naming what it waits on

#### Scenario: checked page
- GIVEN a claimed component `a` whose branch `impl/a` has page #7, and the implementation state's `pages` says its head was checked
- WHEN the view is shown
- THEN `a` links to page #7 and shows it as checked

#### Scenario: waits on
- GIVEN a pending component `a` whose only pending requirement `a/x` has `waits_on` `["b/y"]`, an empty `blocked_by` and no `needs_pending` field
- WHEN the view is shown
- THEN `a/x` shows as blocked by `b/y`, and no source file under `web/src` or `web/test` names `needs_pending`

#### Scenario: a blocker pending on another component than its ID names
- GIVEN a `flight` model in which component `b` is `blocked` by the requirement `b/gone` with `components` `[c, d]`, and component `e` by the requirement `a/x` with no `components`
- WHEN the implementation view is drawn
- THEN `b`'s entry reads `Blocked by b/gone on c, d`, `c` and `d` each linking to its own entry and nothing linking to `b`'s, and `e`'s entry reads `Blocked by a/x` with no link

#### Needs
- orchestrator/queue-from-refs
- replica/implementation-ledger
