---
title: "CLM works with protected and unprotected main"
date: 2026-10-22
status: superseded
superseded_by: land-locally
---

Refines `gate-without-branch-protection`, which assumed this repository's situation: private on GitHub Free, where branch protection and rulesets aren't available, so CI's merge job pushes to `main` itself. Adopting teams often have protection, and this repository would use it if it could.

The repository's `clm.toml` sets `merge` (`config`):
- `push` (the default): CI merges a PR the gate passes by pushing a tested merge commit to `main` (`merge-tested-tree`).
- `auto-merge`: GitHub enforces the merge. `check` and `gate` are required status checks, and `clm ci merge` turns on GitHub's auto-merge only once the gate passes, so the gate's verdict, `spec-approved` included, still decides what lands. Protection's up-to-date rule or a merge queue stands in for testing the merged tree. It needs no required approving review: spec review is `spec-approved` (`humans-review-intent`), and code isn't reviewed by humans.

`clm init` sets `merge` from what `gh` shows about `main`, and prints the protection settings the repository still lacks. It's a repository-only setting, so nobody can change how their own PRs land from outside the repository.
