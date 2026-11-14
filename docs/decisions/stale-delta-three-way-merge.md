---
title: "clm sync merges a stale delta section three-way when the edits don't overlap"
date: 2026-10-27
status: accepted
---

A delta replaces whole sections: a Contract, a Design, a requirement block. When two spec branches modify the same section and one lands, the other's delta is stale (`hash-based-staleness`), and `clm sync` stopped and showed what `main` changed since the delta's base (`stale-delta-shows-main`), leaving the merge to the session. A triage run of 22 approved drafts that needed review and shared the `plugin` and `orchestrator.run` Designs and several requirements left 12 of them in that state after the first landed. Each session did the same three-way merge by hand, or with a script it wrote: the base's text (which `clm sync` already recovers from `origin/main`'s history for the report), the delta's text and `main`'s text. Most of those merges had nothing to decide: the edits were on different lines.

So when the regenerate step finds a delta stale, it first merges each stale MODIFIED section with `git merge-file`, the delta's text against `main`'s over the base's. A clean merge is written into the delta with the section's new base and committed alone as `Merge main into <change>`, printing `merged <element> of <node> into <delta path>`; the change is then applied again. Only a section whose edits overlap, one the delta removes, one gone from `main`, or one whose base text git doesn't have, stops the sync, with the report `stale-delta-diff` already prints. This is what `git rebase` does for code, and what `hash-based-staleness` asks for ("a delta must be rebased before use"): the rebase now happens mechanically where git can do it.

What stays the same:
- **The delta stays whole sections.** The merge is a step `clm sync` performs on the author's text, never a different delta format; the reviewer still reads a section as it will be.
- **`regenerate-dont-merge`.** `specs/` is still regenerated from `main`'s specs and the deltas, never merged. The delta is the authored source, and merging authored text is what a rebase of code already does.
- **Judgment and review.** A clean merge only means the edits are on different lines; two edits can still contradict each other. The merged text is committed on its own, so it shows in the branch's diff; `clm check` judges the result; and the commit changes the branch's fingerprint, so the approval of a branch that needs review lapses and a human approves the merged text, which no one has seen in that form (`tree-diff-fingerprint`, `approval-policy`). Keeping an approval across an automatic merge would let text a human never read land under their approval.

The other half of the cost, a fresh approval for each of many overlapping drafts that need review, is a triage question, not a sync one: drafts gathered into one spec branch for one approval (`yes-no-review`) never go stale against each other.

Alternatives considered:
- **Merge the whole `SPEC.md` instead of each section.** `clm sync` would need the branch's own applied file and `main`'s as the merge's sides, and the result would have to be turned back into a delta; per section it uses the texts it already has.
- **Merge into the branch's `specs/` and leave the delta stale.** `specs/` is regenerated from the deltas (`regenerate-applied-specs`), so the merged text would be lost on the next sync, and `clm check` fails a stale delta.
- **Write conflict markers into the delta on a real conflict.** A delta with markers parses as text and could pass `clm check` with the markers in it; the report already shows the session what `main` changed, and the delta stays clean until the session resolves it.
- **Keep the approval when the only new commit is a clean merge.** Rejected above: it trades the human's review of the combined text for fewer prompts.
