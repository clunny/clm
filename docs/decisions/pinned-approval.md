---
title: "Spec approval is pinned to a diff fingerprint"
date: 2026-10-21
status: superseded
superseded_by: per-writer-ledgers
---

GitHub labels aren't tied to commits, and without branch protection (`gate-without-branch-protection`) nothing dismisses an approval when the PR changes. So when a human adds `spec-approved`, CI records `clm fingerprint` (the `git patch-id --verbatim` of the PR's diff) in a bot comment. On every later push it recomputes the fingerprint: a pure rebase keeps the approval, anything else removes the label.

Alternatives considered: tying approval to the head SHA (every rebase would cost a re-approval) and storing the record in a git ref (needs write access to refs and is invisible to reviewers). A comment is visible on the PR and needs only `pull-requests: write`.

This protects against accidental changes after approval, not a malicious actor with the human's account: anyone who can add the label can also write the comment.
