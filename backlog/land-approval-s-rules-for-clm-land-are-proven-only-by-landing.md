---
title: "land.approval's rules for clm land are proven only by landing a branch, which its crate can't do"
kind: deferred
components: [land.approval, land]
blocks: [gate/own-files]
---

Four requirements of `land.approval` state what `clm land` does: `approval-policy`, `approval-from-base`, `changes-requested-holds` and `signed-land`. Every scenario of theirs reads "WHEN `clm land` runs" and ends "it lands" or "it exits 1 ... and `main` is unchanged". The 25 tests that prove them sit in `land`'s files, `crates/clm-land/tests/approval.rs` (10), `hold.rs` (7) and `signed.rs` (8): each calls `clm_land::land` on a fixture repository and looks at `main` on `origin`. `clm-approval` can't depend on `clm-land`, which calls it, so no test in `crates/clm-approval/tests/` can land a branch, and `land.approval`'s Design says these tests live in `land`'s crate.

`some-tests-prove-a-component-from-another-component-s-files` asked for them to move into `crates/clm-approval/tests/`. They can't move as they stand: there, a test could only ask `clm-approval` whether an approval counts, which is a different assertion from the one each makes today. So they were left where they are, unchanged, and still cite `land.approval` from `land`'s files, which is what `gate/own-files` will refuse a build of `land.approval` to edit.

**What settles it** is a spec change. The human's answer of 2026-11-15 stands for the class (`commands-live-in-their-crates`): a scenario that only another component's command can show leaves the component that can't show it, word for word. Here the command is `land`'s, and its crate already holds the tests. So the proposal is that the four requirements become `land`'s, word for word, the 25 tests are re-cited where they stand, and `land.approval` keeps what its own crate decides and its own tests prove: `approve`, the policy read from a base, whether a fingerprint's approvals count, the refusal a request makes, and signing. Where a requirement states both, it is split at the clause. No test moves and none changes.

Not due before the other two designs that block `gate/own-files` are proposed (step 5 of phase 4 in the plan); it can be proposed with them. The change that carries it out names this item in `from`.
