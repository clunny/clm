---
component: check
---

## MODIFIED Requirements

### Requirement: pending-narrows-on-a-graph <!-- base: 1c2fb00a2e31f332 -->
`clm-check` SHALL export `pending_for(graph, markers, component)`, which returns the pending list `check.coverage`'s `pending(graph, markers)` gives, narrowed to `component`'s entries when one is given, or, when `component` names no component of `graph`, one a live change removed included, a violation of the rule `check.coverage/pending-report` at that component's `SPEC.md` path whose message is ``unknown component `<component>` ``, the same violation `pending_in` returns for it; `pending_in(root, component)` SHALL load the graph and markers from the working tree and return what `pending_for` returns for them, holding no narrowing of its own, so a caller that loads a graph and markers elsewhere, such as from a commit's tree (`view.repo/pending-names-its-commit`), narrows them the same way.

#### Scenario: same as the working tree
- GIVEN a repository with components `a` and `b`, each with an uncited requirement
- WHEN `pending_in(root, Some("a"))`, `pending_for` on the graph and markers loaded from that root with `Some("a")`, and both with `Some("nope")` run
- THEN the first two return the same list, holding `a`'s requirement and not `b`'s, and the last two return the same violation naming `nope`

#### Scenario: a removed component is unknown
- GIVEN no component `g`, an `applied` change whose delta for `g` has `remove: true` and lists `g/x` under `REMOVED Requirements`, and a test citing `g/x` at `t.rs:1`, a file no component's `paths` match
- WHEN `pending_in(root, Some("g"))`, `pending_in(root, Some("nope"))` and `pending_in(root, Some("clm"))` run
- THEN the first two return a violation, the second's of the rule `check.coverage/pending-report`, and the third lists `g/x obsolete t.rs:1`

#### Serves
- capability/see-project-state

