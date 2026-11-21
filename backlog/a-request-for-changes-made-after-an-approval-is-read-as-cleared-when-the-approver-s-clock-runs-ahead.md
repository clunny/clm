---
title: "A request for changes made after an approval is read as cleared when the approver's clock runs ahead"
kind: deferred
components: [ledger, land, land.approval, web.worker.review]
origin: 20261120-wave-2-rest-by-the-rule
---

**The problem.** Whether a request for changes still holds a land is decided by comparing record times written by different machines' clocks. When one person approves a diff and later requests changes to that same diff from another machine, and the first machine's clock is ahead of the second's by more than the time between the two acts, the request is read as already cleared by the approval that came before it. The land goes ahead over a request that is still standing.

**Where it lies.** `ledger` (`Fold::cleared`, `ledger/request-cleared`; the fold orders records by `at` as each writer wrote it, `ledger/fold`), `land.approval` (`land.approval/request-refusal`, `crates/clm-approval/src/hold.rs`, which holds a land only for a request `cleared` is false for), `land` (`land/changes-requested-holds`), and the web app's review page, which shows a request as held or not by the same rule (`crates/clm-web-review/src/feedback.rs`, `held: current && !fold.cleared(r)`).

**The exact sequence** (confirmed with `fold_records` and `Fold::cleared` on `main` at `a98783b5`):
1. Bob approves `spec/x` at fingerprint `f` from his clone, whose clock is two minutes ahead of the web app's: `{"id":"a1","kind":"approved","writer":"bob/1a2b3c4d","at":"2026-11-21T10:02:00.000Z","fingerprint":"f","branch":"spec/x","change":null}`.
2. One minute later in real time, Bob changes his mind and asks for changes from the web app, whose clock is correct: `{"id":"r1","kind":"changes_requested","writer":"bob/web","at":"2026-11-21T10:01:00.000Z","branch":"spec/x","fingerprint":"f","text":"no"}`.
3. `Fold::cleared(r1)` is **true**: the fold holds an `approved` record for `f` by the requester's person (`bob`) whose `at` (`10:02`) is greater as a string than the request's (`10:01`).
4. `clm land spec/x` finds no request holding the branch and lands it if the approval policy is otherwise met; the web app shows the request as not holding.

The same happens with any two of Bob's clones whose clocks differ, or with one clone whose clock was stepped back between the two acts.

**What a person sees.** Bob's "request changes" in the web app appears to do nothing: the branch lands, and the review page doesn't show his request as holding it. Nothing warns that the times disagree with the order the acts happened in.

**What it doesn't affect.** A request by one person is never cleared by another person's approval, whatever the clocks say: `cleared` requires the approval's writer's person to be the requester's (with an approver two minutes ahead and a different requester, `cleared` is false). So the case needs one person acting from two machines, or one clock stepped back.

**Which requirements state the behaviour today.** `ledger/request-cleared` (an approval clears a request when its `at` is greater as a string, by the same person), `ledger/fold` and the Contract's `at` paragraph (records ordered by `at` as written, "so two records' times order as strings"), `land/changes-requested-holds` and `land.approval/request-refusal` (a request holds while `Fold::cleared` is false for it). Each states the comparison as it works; none says what happens when writers' clocks disagree.

**Found by** wave 2 of phase 5, which states the fold's order as it is (`ledger/fold`, scenario "times out of order or equal") and changes nothing here: a fix is a design of how a request and an approval are ordered without trusting two clocks, and it spans `land.approval`, whose pass is wave 3.

**Deferred** (2026-11-21, the human): taken up in wave 3 of phase 5, with `land` and `land.approval` (`backlog/a-passing-check-proves-less-than-the-spec-claims.md`). Until then, `clm triage` lists it as `deferred` and the build loop drafts nothing for it.
