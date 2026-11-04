---
title: "The audit recomputes a signed land's kind and tier with the gate, on demand and in the checkpoint, not in every check of main"
date: 2026-10-27
status: superseded
superseded_by: yes-no-review
---

`clm ledger verify` judged a signed land by the kind and tier its `landed` record states, which the lander writes (`signed-ledger-records`). A lander who records a HARD change as `SOFT`, `AUTO` or `code` skips the approval audit, and one recorded below `SOFT` skips `clm review` too; the signature makes the lie attributable, but nothing reported it. Now `clm ledger verify --tiers` recomputes each signed land's kind and tier as `clm land` computed them, the gate on the merge commit against its first parent, and reports `ledger/understated-land` when the recomputed one ranks above the recorded one (`code` = `AUTO` < `SOFT` < `HARD` < `mixed`). A land recomputed `HARD` is audited for a counted approval as a recorded `HARD` land is. The gate gains `evaluate_at(repo, base, head, approved)`, the same verdict read from git objects for any `head`, so the audit needs no checkout per land.

**Only understatement is reported.** An over-stated tier asked for more review than the diff needed, which hides nothing, so a land recorded `HARD` whose diff the gate now rates `SOFT` is no finding.

**Where it runs.** `clm checkpoint` passes `--tiers`, and a human can run it any time; `clm check` on `main` doesn't. Recomputing loads `specs/` at two revisions for each signed spec land, so a check that ran it would pay that for the whole history on every run, growing with every land, and it may not remember what an earlier run verified: no check reads a cache (`git-is-truth`). The checkpoint ends every session (`checkpoint-before-done`), so an understated land is reported within a session of landing, and `checkpoint incomplete` makes a session look at it. The rest of the audit, which reads records and one tree listing per land, stays in `clm check`.

**Rules drift.** The tier is recomputed with the gate of the `clm` that runs the audit, not the one that landed. A later gate that rates some past diff higher would report an honest land as understated; the land's record and signature are unchanged, and the report names both tiers, so a human can tell drift from a lie. A way to acknowledge such a report is left open.

Rejected:
- **Recompute in every `clm check` on `main`.** Its cost grows with history, without bound, on the most frequent check.
- **A cache of verified lands that `clm check` reads.** It would make a check depend on state outside git (`git-is-truth`); deleting it would change nothing but the time, but a check may not read it even so.
- **A `verified` ledger record that later runs trust.** It moves the trust to whoever wrote the record, which is the problem this solves.
- **A temporary worktree per land, running `clm gate` in it.** It checks out the whole tree for each land, where the gate reads only a few files and `specs/` at two revisions.
