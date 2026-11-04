---
title: "A web approval names the review it was given for"
date: 2026-10-27
status: accepted
---

An approval is pinned to the branch's fingerprint (`per-writer-ledgers`, `tree-diff-fingerprint`) so that `clm land` accepts only the diff a human approved (`humans-review-intent`). From the CLI, `clm approve` computes the fingerprint at the moment the human says yes at the guard's prompt. In the web app, the human reads a review that was drawn earlier and taps Approve later; if the Worker then fingerprints the branch as it is when the request arrives, a push made in between is approved unseen.

So the app sends what the review it showed was built for, and the Worker refuses with 409, writing nothing, when the branch no longer matches:

- On the branch route, the review's `fingerprint`. A fingerprint survives a rebase or a merge of `main` that touched none of the branch's files, so a person isn't asked to approve again for nothing, and it is exactly what the record stores.
- On the PR-numbered route, while the app still used it, the review's `head_sha`, since that review carried no fingerprint. It was stricter than needed (any push refused); the route is gone with `web.worker/pr-routes-removed`, and only the branch route remains.

A request that names neither is refused (422) rather than approving the branch as it is, so an app shell cached before this change can't approve unseen either; reloading the app fixes it.

Alternatives considered:
- *Send the head on both routes.* Simpler, but a merge of `main` that leaves the spec diff unchanged would need a fresh look the diff doesn't call for.
- *Make the field optional and keep the old behaviour without it.* That keeps the gap open for any caller that omits it, which is the bug.
- *Have the Worker remember which review it served to whom.* Needs per-user server state, which `git-is-truth` and `web-derived-cache` rule out, and buys nothing over the client echoing it.
