---
title: "Each task session reviews its own diff before opening its PR; review advises, never gates"
date: 2026-10-23
status: accepted
---

`humans-review-intent` keeps humans out of code review, and `deterministic-enforcement` keeps model judgment from gating. Model review is still worth having: it catches bugs and spec gaps that tests miss. So it runs **inside the implementing session, before the PR opens**, where its findings can still be fixed. It has two reviewers, each starting from fresh context so it isn't anchored on the implementer's reasoning:

- **Claude Code's built-in `/code-review`** for correctness bugs (`reuse-first`). A probe on 2026-10-23 confirmed it runs in a headless `claude -p` session: it found a planted divide-by-zero for $0.12 at level `low`.
- **The plugin's `spec-auditor` subagent** for the spec, which generic reviewers can't see. For each requirement and scenario of the task, it checks where the behaviour is implemented, which test cites it, and whether that test would actually fail if the behaviour broke. It adapts two of bmad-method's review layers, the Acceptance Auditor and the Verification Gap reviewer. It reports findings and never edits.

The session fixes the findings it agrees with, re-runs the checks, and lists every finding and what it did about it in the PR body. That makes the review visible to anyone reading the PR, without making it a gate. The merge still depends only on CI.

Rejected for now:
- reusing `bmad-code-review` directly, because it depends on bmad's own config and scripts (`_bmad/`, `uv`);
- review in CI (Managed Code Review or `claude-code-action`), because it can only comment after the PR opens, when no session is left to act on it, and it needs an API key in CI.

The `/code-review` level and the reviewer names are constants of node `orchestrator.run`, like the superpowers pin, so they can be tuned by a code PR. This names the node, not its crate, so moving the code between crates doesn't make it wrong.
