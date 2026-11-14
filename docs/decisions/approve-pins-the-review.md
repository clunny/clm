---
title: "A web approval names the review it was given for"
date: 2026-10-27
status: accepted
---

An approval is pinned to the branch's fingerprint: the `approved` record carries the fingerprint of the diff the human said yes to (`per-writer-ledgers`, `tree-diff-fingerprint`), and `clm land` accepts the branch only while its fingerprint still equals it, so only the diff a human approved lands (`humans-review-intent`). A rebase, or a merge of `main` that touches none of the branch's files, leaves the fingerprint the same and the approval standing; a change to the diff's content leaves the approval matching nothing, and the branch needs approving again. Pinning the approval to the head commit was rejected: every rebase would cost a fresh approval. The pin protects against a change made after the approval by accident, not against someone acting with the human's account (`approval-gap-accepted`).

From the CLI, `clm approve` computes the fingerprint at the moment the human says yes at the guard's prompt. In the web app, the human reads a review that was drawn earlier and taps Approve later; if the Worker then fingerprints the branch as it is when the request arrives, a push made in between is approved unseen.

So the app sends the `fingerprint` of the review it showed, and the Worker refuses with 409, writing nothing, when the branch no longer matches. A fingerprint survives a rebase or a merge of `main` that touched none of the branch's files, so a person isn't asked to approve again for nothing, and it is exactly what the record stores. Approvals go through the branch route alone; the PR-numbered route is gone (`web.worker/pr-routes-removed`).

A request that names no fingerprint is refused (422) rather than approving the branch as it is, so an app shell cached from before the field existed can't approve unseen either; reloading the app fixes it.

Alternatives considered:
- *Send the review's head commit instead.* Simpler, but any push would be refused, and a merge of `main` that leaves the spec diff unchanged would need a fresh look the diff doesn't call for.
- *Make the field optional and approve the branch as it is without it.* That keeps the gap open for any caller that omits it, which is the bug.
- *Have the Worker remember which review it served to whom.* Needs per-user server state, which `git-is-truth` and `web-derived-cache` rule out, and buys nothing over the client echoing it.

## The rule in full

`ARCHITECTURE.md`'s entry `approval-policy` states this in short (`human-layer-entries-stay-short`); the rule in full:

An approval SHALL name the branch fingerprint it was given for, and counts only while the branch still matches it; an approval of a branch that has moved on is refused and writes nothing.

**Scenario: branch moved after review**
- GIVEN a reviewer viewing a branch that gains a commit changing its spec
- WHEN they approve the page they read
- THEN the approval is refused and nothing is recorded
