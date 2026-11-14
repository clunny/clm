---
title: "The release and site workflows stay on GitHub Actions for now"
date: 2026-10-26
status: accepted
---

`land-locally` takes every check and every merge off hosted CI. Two workflows are left there, each dispatched by a human:
- `release.yml`, a few times a week at most, which builds `clm` for four targets. Its cost is a few metered minutes per release, and it is the only push to `main` that isn't a `clm land`; `clm ledger verify` knows a release merge by its `Release v` message.
- `.github/workflows/site.yml`, which builds and deploys the public site, a Worker of its own (`site-worker-of-its-own`, `ci/site-deploy`).

Actions isn't needed to build anything: the macOS binaries are cross-built on Linux (`cross-built-macos-binaries`), so every target can be built on this machine. The workflows stay there for what Actions still gives: a clean machine built from the pinned toolchain for each release, and a home for the secrets they use (`CLOUDFLARE_API_TOKEN` for `web-deploy` and for the site's deploy), which this machine would otherwise hold.

Moving either off Actions is a change of its own, weighed against those. Nothing else depends on the workflows.
