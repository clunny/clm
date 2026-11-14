---
title: "The web Worker says an exemption-only spec branch needs no review where the gate says it does"
kind: bug
components: [gate, web.worker.review]
---

## Intent

`gate/needs-review` makes a spec branch need review when it adds or widens an exemption, and `evaluate_at` decides that. The Worker (`crates/clm-web-review/src/review.rs`) still decides by `gate/needs-review-of`'s `needs_review_of`, whose "true exactly when" list has no exemption term, so it answers "no review" where the gate answers "review". `needs-review-of` says the Worker may differ only by saying review when the gate doesn't, never the reverse, so the two requirement texts disagree.

Nothing unsafe lands: `clm land` goes by the gate and refuses the branch without an approval. What is wrong is the web app's answer, and that it can't offer the approval for such a branch.

## Settled

Answered on 2026-11-13, by the human: move the exemption comparison into the shared pure function, so the gate and the Worker give one answer. `needs_review_of` takes what it needs to tell an added or widened exemption (the text of each changed component `SPEC.md` at the merge base and at the head, or the exemptions read from them), `evaluate_at` passes it instead of comparing on its own, and the Worker, which already holds the base and head text of every changed file, passes the same. Having the Worker refuse to approve such a branch was rejected: it takes those approvals away from the phone.

It needs a spec change to `gate/needs-review-of`, `gate/needs-review` and `web.worker.review`, which waits for the freeze in `backlog/a-passing-check-proves-less-than-the-spec-claims` to lift.
