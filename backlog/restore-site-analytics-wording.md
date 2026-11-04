---
title: "Put \"friction\" back in the site-analytics change's intent"
kind: deferred
components: [clm]
origin: 20261104-found-by-search-and-agents
---

`20261104-found-by-search-and-agents` changed one sentence of `changes/20261104-site-analytics/change.md`'s `## Intent` from "where most friction after install lies" to "where most of the trouble after install lies", because the retired-terms test then banned "friction" as a bare word and the sentence failed the suite on `main`. The same change frees the plain word (`clm/old-state-and-key-names-retired`), and "friction" is the word the human prefers there.

**Waits on:** `clm/old-state-and-key-names-retired` current at `381faa28`, so the retired-terms test no longer flags the bare word.

**Then:** on a spec branch, restore the original wording in that sentence of `changes/20261104-site-analytics/change.md`, run `cargo test -p clm --test retired_terms` to confirm it passes, and delete this item in the same branch. If the change has been archived by then, leave it as it is, since `changes/archive/` is never edited, and just delete this item.
