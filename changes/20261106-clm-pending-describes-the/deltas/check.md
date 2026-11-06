---
component: check
---

## ADDED Requirements

### Requirement: pending-narrows-on-a-graph
`clm-check` SHALL export `pending_for(graph, markers, component)`, which returns the pending list `check.coverage`'s `pending(graph, markers)` gives, narrowed to `component`'s entries when one is given, or, when `component` names no component of `graph`, the same violation `pending_in` returns for it; `pending_in(root, component)` SHALL load the graph and markers from the working tree and return what `pending_for` returns for them, holding no narrowing of its own, so a caller that loads a graph and markers elsewhere, such as from a commit's tree (`view.repo/pending-names-its-commit`), narrows them the same way.

#### Scenario: same as the working tree
- GIVEN a repository with components `a` and `b`, each with an uncited requirement
- WHEN `pending_in(root, Some("a"))`, `pending_for` on the graph and markers loaded from that root with `Some("a")`, and both with `Some("nope")` run
- THEN the first two return the same list, holding `a`'s requirement and not `b`'s, and the last two return the same violation naming `nope`

#### Serves
- capability/see-project-state
