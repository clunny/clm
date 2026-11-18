---
title: "A run the loop deletes takes its task IDs with it, so its late report counts against the next run"
kind: bug
components: [orchestrator.loop]
---

Step: building `orchestrator.loop/launched-refuses-an-earlier-task` and `orchestrator.loop/returned-ignores-an-earlier-task`. The code review and the spec auditor both found this.

What happens: the loop keeps a run's earlier task IDs only when it clears the run's task ID or replaces a listed run of the same name. That is what the requirement states. Three paths delete a run outright and keep nothing:
- `clm build start` drops every `returned` run (`crates/clm-loop/src/step.rs`, `f.runs.retain(|r| r.state != RunState::Returned)`).
- `clm build release` forgets the run (`crates/clm-loop/src/release.rs`).
- A fold or archive orphan is dropped for the plan to compute again (`Prepared::Drop` in `step.rs`).

When the plan later starts a run of the same name, that run has no earlier IDs. So `clm build launched <name> <old-task>` is accepted, and a late `clm build returned <name> --task <old-task>` from the old subagent is recorded against the new run. Example: `build:a` lands with task `t-1`, a new session runs `clm build start`, and `build:a` is launched again.

Expected: these paths keep the deleted run's task IDs, either in a list on the loop file keyed by run name, or by keeping a stub run. Then the two rules hold whatever path the earlier run left by. This needs a spec change, because no requirement states what a deletion keeps.
