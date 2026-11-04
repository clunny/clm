---
status: proposed
---

# Triage revises a fold the human declines with a different design

## Intent

When the human declines a fold's intent in `/clm:triage` and says what the change should do instead, triage today only leaves the branch, and `clm draft` refuses the item because the declined branch still proposes it, so the human's answer waits for a separate `/clm:propose` session. This change has triage revise the fold in the session, as `propose` revises a declined change: in the fold's worktree, edit the declined change, re-apply it with `clm rebase`, push and ask again. It comes from issue #67 (clunny/clm), accepted by the maintainer.

## Problem

`plugin`'s Design and `skills/triage/SKILL.md` step 9 say only "On a no, leave the branch." `skills/propose/SKILL.md` step 12 already says what to do on a no (discuss, edit, re-apply, push, ask again), but triage never designs in its own context, so it has no such path. `clm draft`'s refusal (`proposed in <id>`) is right: the item is proposed, on the fold's branch, and a second draft would race the first.

## Approaches

- **Revise in the fold's worktree, as propose's step 12 does (chosen).** The human is in the conversation and has just said what they want, so the revision is a conversation, which is what `propose` is for. Triage already knows how to enter the fold's worktree to rebase and land it (`triage-enters-branch-worktree`); revising reuses that.
- **Let `clm draft` redraft an item whose change the human declined.** Rejected: it would need the CLI to know a decline happened (it has no record of one), would leave the declined branch and the new draft both proposing the item, and would run the redesign in a subagent that can't hear the human.
- **Tell the human to run `/clm:propose` afterwards.** Rejected: that is today's behaviour, which the issue reports; the human's answer waits for a session they have to start.

## Outcome

- A no with a different design ends, in the same session, with a revised fold put to the human again; a yes lands it as a first yes does.
- A plain no leaves the branch, as today.

## Constraints

- `approve-in-session`: the revised fold is approved only by a yes in the conversation, through the guard.
- The revision changes the fold's diff, so an earlier approval's fingerprint no longer holds; that is expected.
