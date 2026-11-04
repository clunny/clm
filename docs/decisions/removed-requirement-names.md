---
title: "`clm check` reports spec text that names a requirement a live change removed"
date: 2026-10-27
status: accepted
---

`20261025-forge-adapters` removed `plugin.hooks/label-guard`, but another requirement of the node still named it, and nothing noticed until the node was built (`backlog/check-flags-references-to-requirements-a-change-removes`). A requirement that names a removed one is built against a spec that contradicts itself.

Now `check/removed-requirement-names` reports, in `specs/`, a code span that is a removed requirement's full ID anywhere, or its bare slug in its own node's `SPEC.md`. *Removed* means a change outside the archive lists it under `REMOVED Requirements` and no node holds it: that is the window in which the old name is known and stale text is likely, the change's builds still running. Once the change is archived the rule forgets the name.

False reports are kept out by what counts: only whole code spans, never prose or fenced examples; the bare slug only in its own node; and never a slug that a requirement of any node, a decision or an invariant also uses, since then the span may mean that. A requirement moved to another node by a split keeps its slug, so the parent's text naming it is not reported, while text naming its old full ID is.

Rejected: matching every backticked `<node>/<slug>` that names no requirement. On 2026-10-27 that matched thirteen spans in `specs/` and none was stale: violation rule names (`ledger/unrecorded-land`), file paths (`web/src`) and examples (`check/x`) share the shape.
