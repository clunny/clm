---
title: "Stopped to raise a spec change: pulls?head= breaks a review test"
kind: workaround
components: [web.worker.platform]
requirement: web.worker.platform/fake-github@1a1c6ebd
---

The fake's pulls?head= listing of FakeRepo.pulls, which fake-github states, makes crates/clm-web-review/tests/review.rs::no_ledger fail (an open FakePull #5 from spec/c2 now gives the review a page). That test is web.worker.review's, so the build kept pulls?head= answering from pages alone, built the rest of the requirement uncited, and filed backlog/fake-github-pulls-head-breaks-a-review-test.md for the spec change that orders the two.
