---
title: "`land` is split by carving approval out as `land.approval`, crate `clm-approval`, which `clm-land` calls"
date: 2026-10-31
status: accepted
---

`land` was over both budgets (spec 19,702 of 12,000 tokens, code 50,621 of 40,000 on 2026-10-31, `backlog/split-land`). The human chose, in the triage of 2026-10-31, to create `land.approval`, share the derived-specs check with `sync`, make `ledger`'s `slug` public, and trim `land`'s Design prose. Raising the budget was rejected: the node holds two parts that change for different reasons, which is what `slim-nodes` splits on.

- **Approval**: which commit a branch is, its fingerprint, `clm approve`, whether an approval counts under the policy at the base, whether a request for changes holds the branch, and signing. It changes when the review model does (`humans-review-intent`, `approval-policy`, `land-policy-from-base`, `signed-ledger-records`, `web-review-feedback-as-ledger-records`).
- **Landing**: the gate, the merge, the checks and where they run, the push and its retries, the ledger records, offline and `--check-only`. It changes when the machinery of landing does (`land-locally`, `build-places`).

So the approval half becomes the child `land.approval` with `approve`, `signed-approve`, `approval-policy`, `approval-from-base`, `signed-land` and `changes-requested-holds`, moved with their text unchanged, and `land` keeps its ID (`nested-splits`).

**A crate of its own.** The child can't live in `clm-land` as modules: `use crate::` edges between files of two nodes need `depends_on` both ways (`undeclared-imports`), which is a cycle the model rejects. So it is crate `clm-approval`, which `clm-land` depends on and which takes the root, the environment, the settings and the base as arguments rather than `clm-land`'s `Repo`. `clm-land` re-exports `approve`, so the `clm` crate's subcommand and the root's `depends_on` don't change, as `clm-check` re-exports `clm-coverage` (`check-coverage-split`). The branch's head and fingerprint move with it, since an approval binds to exactly the head and fingerprint `clm land` later computes, and two copies could disagree; `clm-land` takes them from `clm-approval`.

**Dedupes.** `clm-land`'s derived-specs check rebuilt `clm sync`'s regenerate (the scratch `specs/` from the base with the branch's `changes/` and `backlog/`, the change-by-change `reapply`, the own-violations filter), so a fix to one could miss the other; `clm-sync` now offers that tree once (`sync/regenerated-tree`) and `clm land` keeps only choosing the IDs and the byte comparison. `clm-land` slugged commit authors with a copy of `clm-ledger`'s crate-private `slug`; `ledger` makes it public (`ledger/person-slug`), as `20261030-one-rule-for-a` did for `person`.

**How the builds divide the move.** `land` depends on the child, so the child builds first (`nested-splits`): `land.approval`'s build creates `crates/clm-approval`, moves `approve.rs`, `hold.rs`, `policy.rs`, `sign.rs`, `Repo`'s `approval`, `head`, `branch_name`, `fingerprint`, `listing` and `changes` into it, deletes `clm-land`'s copies, adds the re-export, moves `clm approve`'s tests into `crates/clm-approval/tests` and re-cites the land-driven tests in `crates/clm-land/tests` (`approval.rs`, `hold.rs`, `signed.rs`) with the child's IDs. `ledger`'s and `sync`'s builds make their functions public before the callers need them. `land`'s build then switches derived-specs to `regenerated`, drops what that leaves unused (`derived.rs`'s scratch build, `clm-apply` if nothing else uses it), and cites the modified `derived-specs`.

Expected sizes after the builds, estimated from the files: `land` about 38,000 code tokens and 10,000 spec tokens; `land.approval` about 12,000 code tokens and 7,000 spec tokens. `crates/clm-land/tests/common/mod.rs`, about 4,000 tokens with no markers, stays `land`'s, being what every land-driven test uses.
