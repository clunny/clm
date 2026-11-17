---
title: "Install clm for mutate_jobs and mutate_minutes"
kind: human-task
components: [config]
blocks: [config/verify-here]
origin: 20261116-a-mutation-pass-that-stops
---

`config/verify-here` sets two new `[verify]` fields in this repository's `clm.toml`: `mutate_jobs = 6` and `mutate_minutes = 60`. Every `clm` refuses keys it doesn't know (`settings-before-use`). Once those lines reach `main`, any clone whose installed `clm` predates the parser for them (`ba6623d4`, on `main`) fails every `clm check`, `clm rebase`, `clm land` and `clm verify` with "unknown key `mutate_jobs`".

What is needed: install `clm` from a `main` that holds the parser, using `scripts/dev-install.sh` or a merging pull with its hook (`dogfood-from-main`), in every clone you use. Then delete this item on a backlog-only branch, which needs no review. The next `config` build then writes the two values.
