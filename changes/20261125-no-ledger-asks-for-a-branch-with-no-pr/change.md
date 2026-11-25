---
status: applied
resolves: [fake-github-pulls-head-breaks-a-review-test, workaround-web.worker.platform-fake-github-1]
---

## Intent

Answers `backlog/fake-github-pulls-head-breaks-a-review-test` with the human's choice (2026-11-25), "Review moves no_ledger first", for wave 6 of `backlog/a-passing-check-proves-less-than-the-spec-claims.md`. The small code branch `fix/no-ledger-asks-for-a-branch-with-no-pr` (landed as `08777abf`) already made the move: `crates/clm-web-review/tests/review.rs::no_ledger` now clears `spec_pr()`'s pull requests, so the branch it asks about has none. With that test fixed, `web.worker.platform/fake-github` can be built whole, REST `pulls?head=` half included, so this change retires the question and the platform build's workaround flag. It changes no requirement.

## Problem

`fake-github` says the fake's REST `pulls?head=` lists the open pull requests of `FakeRepo.pulls`. `no_ledger` set up an open `FakePull` #5 from `spec/c2` and asserted the review has no page, so the platform's build, which may not edit the review's tests (`path-ownership`), left `fake-github` unbuilt with a workaround flag and the question.

## Approaches

- **Move the test first, by code (taken).** `web.worker.review/pr-review` already says `page` is the open pull request whose head is the branch, "null when there is none", and its scenario "the page" gives #5 as `spec/c2`'s page. So `no_ledger` contradicted its own requirement, and fixing it states no new behaviour: a test edit that no build can be told, made on a small code branch before this one (`small-change-no-claim`). It was checked with the fake changed by hand to list `FakeRepo.pulls`, under which the web crates' 292 tests pass.
- **A requirement of the review named in `fake-github`'s `Needs`** (the question's first choice as written). `pr-review` already needs `fake-github`, so `fake-github` needing `pr-review` would be a cycle. The order would need a new requirement of the review that states nothing `pr-review` doesn't already state, which only adds spec text.
- **Drop the fake's REST half** (the question's second choice). The review's planned fork-page tests need it, so the human chose against it.

## Outcome

The question and the workaround flag are gone, and `web.worker.platform/fake-github` stays unbuilt with nothing blocking it. The next build of `web.worker.platform` builds the requirement whole, and then `web.worker.review`, `web.worker.replica`, `web.worker` and `web.views`, which wait on it, are built.
