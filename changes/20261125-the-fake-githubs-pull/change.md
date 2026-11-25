---
status: applied
resolves: [the-fake-githubs-pull-listing-ignores-its-base]
---

## Intent

The fake GitHub's REST pull-request listing honours `base=` as GitHub does, so a review's test can show that its `page` is held to a pull request onto the default branch. `web.worker.platform/fake-github` gains the rule and a scenario, and `web.worker.review/pr-review` a scenario that a pull request onto another base gives no `page`.

## Problem

`web.worker.review/pr-review` gives as `page` the open pull request on the default branch whose head is the branch, and the review's page query passes `base=<default branch>` for it (`crates/clm-web-review/src/review.rs`, `page`). The fake's REST `pulls?head=` route (`rest_pulls` in `crates/clm-web-platform/src/fake.rs`) filters on `head=` alone, so a pull request from the same branch onto another base is listed too. Dropping `base=` from the review's query would pass every test, so a passing check proves less than `pr-review` says. `fake-github` as written states the `head=` filter and nothing of `base=`, so the fix needs a written rule (`code-fix-needs-a-written-rule`), and the review's case needs a scenario its test cites.

## Approaches

- **Taken: the fake honours `base=`, and `pr-review` gains a scenario.** The fake answers as GitHub does where the crates' tests depend on it, which is what `fake-github` is for. A new sentence states the filter; a new scenario proves it; the existing sentence and scenario stay as they are, since a listing with no `base=` is unchanged. A new scenario on `pr-review`, rather than a widened *the page*, keeps that scenario's outcome under its name.
- **Not taken: a deferred item for the review's test alone.** It would need the human to judge it due, and nothing would make the review's build cite the case; a scenario makes it pending, ordered after the fake by `pr-review`'s existing `#### Needs` on `web.worker.platform/fake-github`.
- **Not taken: filtering by base in the review's own code after the listing.** The query already names the base; holding the rule twice is a workaround for the fake, not a fix of it (`redesign-before-workaround`).

## Outcome

- `web.worker.platform/fake-github`: with `base=<base>` the REST listing gives only the pull requests whose base `ref` is `<base>`, a page's base being `FakeRepo.default_branch`; new scenario *the base a listing names*.
- `web.worker.review/pr-review`: new scenario *a pull request onto another base*, `page` null. Its `#### Needs` already names `web.worker.platform/fake-github`, so the review's build waits for the fake's.
- No Contract or Design changes: `FakePull` already carries `base_ref`, and the fake's knobs are unchanged.

Edge cases the change knows it needs: none. Workarounds: none.
