---
status: proposed
resolves: [building-the-renamed-test-rule-breaks-land-s-test-of-the-old-rule, reconsider-check.coverage-test-lists, the-fix-for-a-red-verify-lost-to-a-note-is-outside-the-plan-s-freeze]
---

## Intent

Records three answers the human gave on 2026-11-19 to questions the builds of phase 5's tooling raised (`backlog/a-passing-check-proves-less-than-the-spec-claims`), and deletes the items they settle, so the requirements those items block can be built. It changes no requirement.

## Problem

- `20261118-renaming-a-citing-test` couldn't be built: `land`'s test of a renamed test asserted the old rule, `check.coverage`'s build may not edit it, and `land` waits on `check.coverage`. The question that asked how to sequence it blocks `check.coverage/test-lists`.
- That stop alone raised a workaround flag and, with it, a reconsider item on `check.coverage/test-lists`.
- `20261118-the-build-loop-loses` waits on a question asking for an exception to the freeze, which blocks `orchestrator.loop/plan-verify`.

## Approaches

- **Taken, already done: a small branch edited `land`'s fixture** (`a7baa5fa`), the human's answer, so its renamed test cites nothing and the land refuses under the old rule and the new. `check.coverage/test-lists` and then `land/deleted-tests-approved` build as written.
- **Taken: `20261118-the-build-loop-loses` is built now**, the human's exception to the freeze: every wave is built through that loop, and a red verify is how a slow test's break reaches the human.
- **Not taken here: an owner step for an obsolete marker in another component's file.** The human agreed to it on 2026-11-19 and it was drafted on this day as a change of its own; its architecture audit found it removes nothing of the removing component's code, gives two builds one requirement with no order between them, and hands the owner's build a re-cite it can't judge, which `stale-markers-keep-behind` had rejected for a `behind` marker. The case can't arise in this repository, which holds no exemption. The two questions that ask it are folded into one that carries the findings, for the human to answer again.

## Outcome

- The three items are deleted, with `backlog/workaround-check.coverage-test-lists-1`.
- The plan item records the exception and the answers.
- `backlog/an-obsolete-marker-in-another-component-s-file-is-nobody-s-to-remove` holds the one open question; `backlog/an-obsolete-marker-in-another-components-file-keeps-its-change-from-completing`, which asked the same, is deleted into it.

## Constraints

- No requirement, scenario, Contract or Design changes, so nothing is built from this change and it is archived in its own branch (`archive-in-spec-pr`).
