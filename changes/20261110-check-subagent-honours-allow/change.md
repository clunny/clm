---
status: applied
resolves: [check-subagent-honours-allow-rule]
---

## Intent

Record that the check `backlog/check-subagent-honours-allow-rule` waited on has passed, and close the item. The human ran it in notespeed once `20261105-unattended-steps-denied-in` was built: an auto-mode `/clm:build` pushed its rebased claim branch with `clm branch push` and landed, and a triage draft whose gate printed `spec` landed itself, with no classifier denial. The decision `unattended-steps-run-clm` now states that result instead of saying the claim rests only on Claude Code's docs, and the item is deleted. Backlog: deletes `check-subagent-honours-allow-rule`.

## The human's answers

> Ran: both landed — Build pushed and landed and a spec draft landed itself, with no denial; delete the item

## Problem

`unattended-steps-run-clm` said that a seeded allow rule covering a subagent's `clm` Bash call rested on Claude Code's docs and could be tested only once built, and handed the test to a `human-task` item. The test has now been run and passed; the decision should say so, so no later session doubts the seeded rules or re-runs the check.

## Approaches

- **Amend the decision with the result and delete the item (chosen).** The fact the item waited on goes into the decision it came from, where readers of the design look.
- **Delete the item only.** Rejected: the result would live only in this change's quote, and the decision would still read as unverified.

## Outcome

- `docs/decisions/unattended-steps-run-clm.md`: its last section records the 2026-11-10 confirmation.
- `backlog/check-subagent-honours-allow-rule.md` deleted.

No requirement changes; nothing to build.

## Constraints

None.

## Open questions

None.
