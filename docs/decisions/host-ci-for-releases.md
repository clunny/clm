---
title: "The release workflow stays on GitHub Actions for now"
date: 2026-10-26
status: accepted
---

`land-locally` takes every check and every merge off hosted CI. The one job left there is `release.yml`: a human dispatches it a few times a week at most, and it builds `clm` for four targets, two of them macOS, which this machine can't build. Its cost is a few metered minutes per release, and it is the only push to `main` that isn't a `clm land`; `clm ledger verify` knows a release merge by its `Release v` message.

Moving it off Actions means either building the macOS binaries elsewhere (a Mac, or a hosted macOS builder that isn't Actions) or dropping the macOS targets, which is a decision about who CLM serves. Revisit when either is wanted; nothing else depends on the workflow.

(Amended 2026-11-04, `20261104-macos-cross-build`: the macOS binaries are now cross-built on Linux (`cross-built-macos-binaries`), so every target can be built on this machine. The first paragraph's reason for Actions no longer holds. The release stays there for what it still gives: a clean machine built from the pinned toolchain for each release, and a home for the secrets it uses (`CLOUDFLARE_API_TOKEN` for `web-deploy`), which this machine would otherwise hold. Moving it is a change of its own, weighed against those.)

(Amended 2026-11-05, `20261105-site-blog`: the site is now a Worker of its own, built and deployed by the site workflow a human dispatches, `.github/workflows/site.yml` (`site-worker-of-its-own`, `ci/site-deploy`). It is a second dispatched workflow kept on Actions for the same reason as the release: the Cloudflare secrets it uses live there, not on this machine.)
