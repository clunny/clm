---
title: "The fake GitHub's REST pull-request listing ignores its base, so no test sees a review's page held to the default branch"
kind: bug
components: [web.worker.platform]
---

`web.worker.review/pr-review` gives as `page` the open pull request "on the default branch" whose head is the branch, and `review.rs`'s page query passes `base=<default branch>` for it. The fake's REST `pulls?head=` route (`rest_pulls` in `crates/clm-web-platform/src/fake.rs`) filters on `head=` alone and ignores `base=`, so a pull request from the same branch onto another base is listed too. No test in `clm-web-review` can then show the review leaving such a pull request out: dropping `base=` from the query would pass every test. Expected: the fake honours `base=` as GitHub does (a pull request is listed only when its `base_ref` is the named base), so `web.worker.review`'s build can add the case to `the_page`. Found by the spec auditor of `web.worker.review`'s build.
