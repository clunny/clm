---
title: "Questions and human steps are backlog items that block tasks"
date: 2026-10-22
status: accepted
---

Implementation sessions run unattended, so they can't ask. When the work needs a fact only a human knows (a domain, an account name) or a step only a human can take (creating an account, a DNS record, a secret), the session doesn't guess. It adds a backlog item: kind `question` for a fact, or the new kind `human` for a step. When the rest of its task can't go on without it, it sets the task's `blocked` to `backlog/<slug>` and opens its PR with the work so far. `clm next` skips the task until the item is deleted, and the web UI lists such items as "Waiting on you".

The human answers by deleting the item in a small spec PR that records the answer in the spec or a decision. On the human's word, Claude may make that PR. The gate rates closing a `question` or `human` item `SOFT`, so a human sees each answer.

Most questions should be caught before implementation, so `propose` runs a **builder check**: a fresh subagent reads the change as its builder would and reports what it would have to guess. `propose` settles what it can and lists the rest under `## Open questions` in the PR body, so they're answered during review, at no extra approval. The check advises and never gates (`deterministic-enforcement`).

This came up designing `web`: nothing said which domain to use, and creating the GitHub App and the Cloudflare secrets needs the operator.

Alternatives considered:
- A separate pre-implementation Q&A phase. It would cost another review round, and questions also surface mid-task.
- Letting sessions block on a GitHub issue. That's state outside git, against `git-is-truth`.
- Letting sessions guess and flag the guess in the PR. The guess may be built on before anyone reads the flag.
