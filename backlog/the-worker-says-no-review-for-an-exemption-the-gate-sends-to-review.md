---
title: "The web Worker says an exemption-only spec branch needs no review where the gate says it does"
kind: question
components: [gate, web.worker.review]
---

## Intent

`gate/needs-review` now makes a spec branch need review when it adds or widens an exemption, and `evaluate_at` decides that. The Worker (`crates/clm-web-review/src/review.rs`) still decides by `gate/needs-review-of`'s `needs_review_of`, whose "true exactly when" list has no exemption term, so it answers "no review" where the gate answers "review". `needs-review-of` says the Worker may differ only by saying review when the gate doesn't, never the reverse, so the two requirement texts disagree.

## Open questions

- Should `needs-review-of` take the exemptions the branch adds or widens (as a new argument), so the Worker can compute them from the files it reads, or should the exemption clause be dropped from the Worker's promise and the Worker refuse to approve such a branch?
