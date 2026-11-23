---
title: "Whether a propose/ claim is done is decided in two crates"
kind: idea
components: [orchestrator.checkpoint, orchestrator]
---

**The problem.** Whether a `propose/<slug>` claim is done is decided twice. `clm wrap-up` deletes `origin/propose/<slug>` when `origin/main` has no `backlog/<slug>.md` or a change outside the archive there names `<slug>` in `resolves` (`orchestrator.checkpoint/checkpoint`, `crates/clm-checkpoint/src/lib.rs`), and `orchestrator`'s reading of the claim refs ends a claim when a live change on `main` names the item (`crates/clm-orchestrator/src/taken.rs`, `taken_from_refs`). The two agree today only because each was written to (checkpoint's Design: "checkpoint and `clm claim` agree"); a change to one leaves `clm wrap-up` deleting claims `clm proposals` still shows, or keeping ones it shows as ended.

**Where it was found.** Wave 4 of `backlog/a-passing-check-proves-less-than-the-spec-claims.md` (rule: a rule two crates each hold a copy of becomes one call its provider offers). Not merged there: `orchestrator` is on branch A and `orchestrator.checkpoint` on branch B, and moving it would order B's build after A's.

**What would fix it.** `orchestrator` offers one call that says whether a claim on an item is done for a given tree (its `requirement` stated in `orchestrator`), and checkpoint calls it, its own copy deleted.
