---
title: "The fake's `pulls?head=` listing `FakeRepo.pulls` breaks `web.worker.review`'s `no_ledger` test"
kind: question
components: [web.worker.platform, web.worker.review]
blocks: [web.worker.platform/fake-github]
choices:
  - "Review moves no_ledger first"
  - "Drop fake-github's REST half"
---

`web.worker.platform/fake-github` (`1a1c6ebd`) says the fake's `GET /repos/{o}/{r}/pulls?head=<owner>:<branch>` SHALL give each open pull request of `FakeRepo.pulls` whose head is that branch, beside `FakeRepo.pages`. `crates/clm-web-review/tests/review.rs::no_ledger` (`web.worker.review/pr-review`) builds its repo with an open `FakePull` #5 from `spec/c2` and no `pages`, and asserts the review has no page ("no page without a PR"). Once the fake lists `FakeRepo.pulls`, the review's own page lookup (`src/review.rs`, `page`) finds #5 and that test fails; the test is `web.worker.review`'s file, which the platform's build may not edit (`path-ownership`).

The platform's first build of wave 6 built every other part of the requirement (the paged `Pulls` query, `t<j>` directory entries with integer modes, `blob_fails` and `fail_paths`, and the richer `pages` entries), but kept `pulls?head=` answering from `pages` alone and cites nothing at `fake-github`, so `main` stays green. The review's notes for wave 6 already plan tests that need the REST half (the fork's page, `FakePull { fork: true, .. }`).

A spec change should order the two: a requirement of `web.worker.review` that moves `no_ledger` off an open `FakePull` (or onto a `FakePull` whose head is not asked for), named in `fake-github`'s `Needs`, so the platform's next build adds the REST half after it; or drop that half from `fake-github` and have the review's tests set `pages` for a page.
