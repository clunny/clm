---
title: "A modified delta is rated by the elements the PR changed in it"
date: 2026-10-27
status: superseded
superseded_by: yes-no-review
---

`change-tiers` rated a delta the PR adds or modifies by every section it holds at `HEAD`. A delta already on `main` passed its own gate when it merged; a later PR that only rebases it after a Design edit on `main`, moving its `design` base and its copy of the Design, was still rated `HARD` whenever the delta also held a `MODIFIED` requirement, though that requirement was untouched. `clm check` (`apply/stale-base`) forces such rebases, so `AUTO` and `SOFT` work on a node with a proposed `HARD` change became `HARD` too, and backlog items learnt to defer Design fixes to avoid it.

So a delta the PR modifies is rated by the elements whose text or base differs between the merge base and `HEAD`, each at the tier of its section: a Design is `AUTO`, an added requirement `SOFT`, a modified or removed requirement, the frontmatter or the Contract `HARD`. A base that moved with the text unchanged still counts, at the tier of the element it rebases onto: keeping a delta's text while moving its base past `main`'s edit discards that edit when the change applies, which is a change to that element. The element-wise rating is the one `spec-element-tiers` already gives `specs/`, applied one step earlier.

The whole-content rating stays wherever the parts can't be compared safely: an added delta, a modified one whose `node`, `create` or `remove` changed, one whose merge-base text doesn't parse, and one in which no element differs (a rationale-only edit), so the rule fails no less closed than before. A delta whose node is the root `clm` at either side stays `HARD` whatever changed, and so does every changed element of a `remove: true` delta, where the removal decides.

Alternatives considered:
- Rating a base-only move `AUTO`, since the delta's text is unchanged. A moved base overrides `main`'s new text for that element, so a requirement's base move would drop a reviewed requirement edit with no review.
- Rating a modified delta by the diff's lines. Lines don't map to tiers without parsing the delta, which the element compare already does.
- Exempting rebases only when `clm check` would otherwise report the delta stale. That needs the gate to recompute staleness, and still rates a hand rebase that touches nothing else `HARD`.
