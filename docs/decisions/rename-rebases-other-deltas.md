---
title: "clm rename rebases the other live changes' deltas whose bases it moves, by applying its own edits to them"
date: 2026-11-02
status: accepted
---

A term swap, or a reference rewrite, changes the text of every element it touches, and so its hash. When another live change's delta MODIFIES or REMOVES one of those elements, that delta is stale (`apply/stale-base`), and `clm check` fails the rename branch over a change it doesn't own. Drafting `20261102-rename-states-and-keys-2` met it: a pair reworded `config`'s Contract, which the deferred contract change `20261102-rename-node-to-component-contract` modifies; `clm rename` exited 0 and `clm check` then failed. A rename branch can't rebase the other delta by hand, since its tree must be exactly what `clm rename` produces (`rename-by-replay`), and `clm rebase`'s `other-delta-merge` would commit a merge the replay doesn't reproduce. The session dropped the pair and filed its leftover as a backlog item.

So `clm rename` rebases those deltas itself, as part of its output (`rename/other-deltas`): for each element the rename moved from a current base to a stale one, it applies to the delta's text of that element the same edits it makes to spec text, and records the element's new hash as the base. The replay reproduces it, since it is made from the base.

Why the rename's own edits and not a three-way merge (`stale-delta-three-way-merge`): the edits are a function of the text, so applied to the delta's text they keep the delta's own edits and add the rename's, and a line both touched can't conflict. A merge would refuse whenever the other change reworded a line holding the term, need a `git` process in a crate the wasm build compiles, and give the same text where it succeeds, but for the term in lines the other change wrote, which the swap retires anyway. The cost is that the other change's own new lines get the new word too; the term swap needs review (`rename-by-replay`), and the rebased deltas are in the diff the human approves.

Rejected:
- **Refuse such a term, naming the change and the element**, as `rename/refusals` does for a renamed ID another change targets. It warns before the tree is written, but leaves the same dead end: the pair waits, by hand, until the other change is applied, and a deferred contract change can stay `proposed` for a long time. The ID refusal stays: a renamed component or requirement is gone from the graph after the rename, so a delta naming it can't be rebased, only rewritten.
- **Rebase with `stale-delta-merge` in `sync.regenerate`.** `clm-rename` would depend on `clm-regenerate`, which runs `git` in a repository and keeps the stages a rename has no use for; see above for the merge itself.
- **Swap the terms in every live delta.** It would edit changes the rename doesn't break, and each such change's intent as reviewed (`yagni`).
