---
status: applied
resolves: []
---

## Intent

On the backlog, the change sets the `components` of the bug `defects-a-build-loop-session-reports-never-reach-main` from `orchestrator.loop` to `plugin`, where its fix lies, and says in the item what was checked. It has no delta: the fix is code-only, since written requirements already state the behaviour it restores, and it doesn't lie in the build loop, so `clm claim --fix` can't take it from here (`code-fix-resolves-bug`). The next `clm triage` groups it under `plugin`.

The loop behaves as its requirements say. It drafts the bugs `clm triage` groups from `origin/main` (`orchestrator.loop/plan-order`), and queues only the `dirty` and `unpushed` lines `clm wrap-up` prints (`orchestrator.loop/step-housekeeping`), which by design name no untracked file in a main checkout on `main` (`orchestrator.checkpoint`, scenario *human's files on main*). So its end summary saying the session can end follows `orchestrator.loop/end-summary`. What is wrong is the build skill's sentence that "the loop drafts reported defects with the backlog's other bugs": that happens only once a report is on `main`, and the skill names no step that puts it there. The route exists and its rules are written: a branch of the session's own from `origin/main` that only adds the item gates `code` (`gate/backlog-additions`, `small-change-no-claim`) and lands with `clm land <branch>`. The fix names that route in `plugin/skills/build/SKILL.md` in place of the false sentence, as `d62a4f0a` brought the same skill's `run` bullet into line with `orchestrator.loop/step-actions`.

Not taken: having `clm wrap-up` or the loop report a main checkout's untracked backlog items as unsafe. That needs a new requirement and is code work outside the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, whose freeze holds; the skill's route is enough to bring reports to `main`. This supersedes the acceptance in `20261113-a-defect-reported-from`, which held that a skill fix needed a requirement whose test reads the sentence.
