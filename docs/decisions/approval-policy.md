---
title: "Who may approve is the project's setting; the human approves with a command the guard makes them confirm"
date: 2026-10-26
status: accepted
---

Refines `humans-review-intent` in practice and supersedes `approve-in-session`, whose mechanism it keeps. When `propose` has a `HARD` change on a branch, it shows the human the change's `## Intent` and `## Significant` sections, its decisions and open questions, and asks whether they approve. On a yes, Claude runs `clm approve <branch>`, which appends an `approved` record for the branch's fingerprint to the human's ledger. A SOFT land's after-the-fact review is `clm review <change-id>`, the same way.

The conversation alone isn't the safeguard: a "yes" is Claude's reading of the conversation, which is model judgment (`deterministic-enforcement`). So the guard answers `clm approve` and `clm review <id>` with Claude Code's `ask` permission decision, and the human's confirmation of that prompt is the approval, in any permission mode. Unattended sessions run under `dontAsk`, which denies `ask`, so they can never approve.

Whether someone other than the author must approve is not the tool's rule but the project's: `approval = "any"` (the default) counts any writer's approval, the author's included; `approval = "other"` counts only an approval by a person who authored none of the branch's commits, and `clm land` refuses otherwise. A hackathon, a small team or a small project may not want the rigor, and one person working alone can't have it. There is no solo/team mode: a team differs from one person only in having a remote, which `host` says, and in the policy it chooses.

The record names who approved, by the writer's person. That is a claim, not a proof, as a label on a shared GitHub account was; a repository that lists its signers in `docs/allowed_signers` makes it a proof: there an approval counts only when signed by its writer's key (`signed-ledger-records`).

Rejected: enforcing other-person approval in a "team mode" (policy, not mechanism, and it excludes the projects that don't want it) and approving on a chat "yes" alone (model judgment would gate a land).
