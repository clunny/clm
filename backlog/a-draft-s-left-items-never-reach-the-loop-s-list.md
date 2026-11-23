---
title: "A draft's left items never reach the loop's list"
kind: bug
components: [orchestrator.loop, orchestrator.triage]
origin: 20261122-wave-4-loop-by-the-rule
---

`orchestrator.loop/return-rules` says the loop records each `left <slug>: <reason>` line a draft's finish prints among the items drafts of the loop left, and the loop does read such lines from `clm draft --finish`'s output (`crates/clm-loop/src/returned.rs`). But `clm draft --finish` prints no such line: `crates/clm-triage/src/finish.rs` prints `freed`, `released`, `removed`, `kept` and branch deletions. `left <slug>:` is printed by `clm draft` when it claims (`crates/clm-triage/src/command.rs`) and by the draft's subagent in its final message, of which the session reports only the last line. So in practice a draft's left items reach the loop's list only through an escalation, and `orchestrator.loop/plan-order`, which relies on the list, never sees the others.

Found by wave 4 of `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, which kept the clause as the code does it: the fix needs a change to what the session reports or to what `clm draft --finish` prints, which crosses into requirements the plan holds (`plan-order`) and into `orchestrator.triage`, on the wave's other branch. Expected: the left items a draft reports reach the loop's list by a route the spec states, or `return-rules` stops claiming that a draft's finish prints them.
