---
title: "Take the operator's App ID and KV namespace ID out of the repository, so clones don't point at the operator's app"
kind: idea
components: [web, web.worker, ci]
---

## Intent

Someone who clones the public repository and deploys it should set up their own Worker, KV namespace and GitHub App, not find the original operator's IDs in the files. These IDs aren't secrets: GitHub's API serves the App ID to anyone, and a KV namespace ID is useless without a Cloudflare token. This is about clones and tidiness, not security.

## Where they are

- The KV namespace ID: `web/wrangler.toml`, `web/README.md`, `specs/web/SPEC.md` and `crates/clm-web/tests/deploy.rs`.
- The App ID: `web/README.md`, `specs/web/worker/SPEC.md` and `crates/clm-web/tests/deploy.rs`.

## Ideas so far

- Remove both IDs from the prose and the test.
- Have the release workflow fill the KV ID into `wrangler.toml` from a GitHub Actions variable, because a file outside git wouldn't reach the release's fresh checkout.
- Rewrite `web/README.md`'s "What the setup made", which reads like a private runbook, as general setup instructions a person deploying their own copy can follow.
