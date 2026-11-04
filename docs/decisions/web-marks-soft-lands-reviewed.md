---
title: "The web app marks a SOFT land reviewed with a `reviewed` record, its second and last write"
date: 2026-10-27
status: superseded
superseded_by: yes-no-review
---

`soft-review-after-merge` has a human review a SOFT land after it merges. Since `per-writer-ledgers` that review is a `reviewed` record naming the land's fingerprint, and `clm review <change-id>` appends it from a session, behind the guard's prompt (`approval-policy`). The web app's PR document is the reader's best view of a spec diff, and a land awaiting review is exactly such a diff, so the app lists the SOFT lands with no `reviewed` record and offers **Mark reviewed**, which the Worker answers by appending a `reviewed` record to the signed-in person's web ledger, `clm/ledger/<login>/web`, the way it appends `approved`.

Why this is safe:
- It is the human's own act, signed in with their GitHub account and tapping the button, as web approval already is; no agent can reach it, so `humans-review-intent` and `approval-policy` hold.
- A review gates nothing: `clm land` never reads `reviewed` records. A wrong one only drops a land from `clm review`'s list.
- The App already has `contents: write` for the ledger append, so no permission changes and no installation has to accept anything.
- The request takes no body. What it records is the `landed` record's fingerprint, fixed by the landed commit, so unlike an approval of a branch, whose head can move between showing the review and the tap, there is nothing to pin.

The Worker builds the land's review by comparing the landed commit's first parent with the commit, so it shows what the land brought to `main`, whatever happened to the branch since.

This is the app's second write, and the Contracts of `web` and `web.worker` name both. Further writes, such as proposals or comments (`backlog/web-proposals.md`), each need their own decision.

Rejected:
- **Keeping review in sessions only.** It works, but a reviewer on a phone can read the diff in the app and then has to open a session to say so.
- **The label it replaces.** `soft-review-after-merge` first used a `spec-review` label removed by hand; labels are gone (`per-writer-ledgers`).
