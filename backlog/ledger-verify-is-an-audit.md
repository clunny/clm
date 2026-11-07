---
title: "clm ledger verify audits main against the ledger, while clm verify runs slow tests: one word, two meanings"
kind: idea
components: [ledger, land.approval, plugin, ci]
origin: 20261107-slow-tests-after-land
---

## Intent

`clm verify` (`20261107-slow-tests-after-land`) runs a project's slow tests after the land, the usual meaning of verify (Maven's `verify` phase). `clm ledger verify` audits `main` against the ledger: every commit a recorded land, approvals and signatures valid. The glossary defines both, but the same word for two things fails `plain-terms` in spirit. Rename `clm ledger verify` to `clm ledger audit`, with the requirement IDs and every caller (the release workflow, skills, the guide), keeping the old name as an alias marked `clm:remove-by` until the callers move (`expand-contract`).

Raised by the human on 2026-11-07.
