---
title: "The human approves a spec PR in the session, by confirming the guard's prompt"
date: 2026-10-22
status: superseded
superseded_by: approval-policy
---

Refines `humans-review-intent` in practice; the invariant is unchanged. When `propose` has a `HARD` spec PR open, it shows the human the change's `## Intent` and `## Significant` sections, its decisions and open questions, and asks whether they approve. On a yes, Claude adds `spec-approved`. On a no, they discuss what to change. A `SOFT` PR's after-merge review is cleared the same way, by removing `spec-review`.

The conversation alone isn't the safeguard. A "yes" is Claude's reading of the conversation, which is model judgment (`deterministic-enforcement`). So the guard answers any command that adds `spec-approved` or removes `spec-review` with Claude Code's `ask` permission decision, as it does for `clm defect` issues (`defect-reports`). Claude Code then asks the human to confirm the command, in any permission mode; we tested `bypassPermissions` on 2026-10-22. Unattended sessions run under `dontAsk`, which denies `ask`, so they can never approve.

The human approves intent, not lines: the significance rubric (`significance-rubric`) makes sure the summary names everything that matters, and the diff is still there to read.

Rejected:
- **A `clm approve <pr> <fingerprint>` command the human types with `!`.** It works, but the human asked to approve a summary in conversation rather than a diff, and with no external command to type.
- **Approving on a chat "yes" alone.** That would let model judgment gate a merge.

Because Claude shares the human's GitHub account, CI still can't tell who added a label. That stays the job of `app-attested-approval`.
