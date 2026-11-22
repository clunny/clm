---
status: applied
---

## Intent

`ledger` offers `Body::approved(fingerprint, branch, change)`, a constructor of the `approved` record body, beside the variant. It is a prerequisite of wave 3's branch A (`20261121-wave-3-land-by-the-rule`), which gives the `approved` record a field naming the requests for changes it clears (`ledger/request-cleared`), so that an approval made before a request no longer clears it because its writer's clock ran ahead (`backlog/a-request-for-changes-made-after-an-approval-is-read-as-cleared-when-the-approver-s-clock-runs-ahead`).

## Problem

Rust can't add a field to an enum struct variant without breaking every struct literal of it. Fourteen literals of `Body::Approved` sit in other components' files: five in `land.approval` (`crates/clm-approval`), seven in `land`'s tests, one in `web.worker.platform`'s tests and one in `web.worker.review`. A build changes only its own component's files (`coupled-changes-sequence`), so `ledger` can add the field only once those callers build the body through a constructor, and a constructor must exist on `main` before they can move to it. Nothing in branch A could order that: `web.*` is optional, so no `#### Needs` of `ledger`'s can wait on it, and the build loop never schedules a deferred item.

## Outcome

- `ledger/approved-constructor` is added and the Contract names the constructor. Nothing else changes, and nothing is cut.
- After this change is built, the session moves each component's literals onto the constructor on small code-only branches (`small-change-no-claim`); branch A is then rebased onto `main` and its `ledger/request-cleared` adds the field.
- The build of this change also folds in a code-only loosening that states no behaviour: `crates/clm-ledger/src/verify.rs:301-308` (`Rated::gate`) stops matching `clm_gate::Kind` exhaustively, naming `Kind::Code` and `Kind::Mixed` and sending every other kind to `Rated::Spec { review }`, so `gate`'s build in branch A, which adds a kind, compiles in `clm-ledger`.
