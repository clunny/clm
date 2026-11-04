---
title: "Merge gate without branch protection"
date: 2026-10-21
status: superseded
superseded_by: land-locally
---

The private repo is on a free plan, so branch protection, rulesets and native auto-merge are unavailable. Instead, `clm gate` runs in CI and decides deterministically: spec PRs need the `spec-approved` label, mixed PRs are rejected, and branches must be up to date. A workflow merges passing PRs into `main`. Direct pushes to `main` stay technically possible and are forbidden by convention. Revisit if the repo moves to a paid plan or an org.
