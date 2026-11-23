---
title: "draft-of-gone-items-dropped forbids the kept say that return-rules asks of every draft finish"
kind: bug
components: [orchestrator.loop]
---

orchestrator.loop/return-rules (3c0219a4) says the loop passes each line starting `kept ` to a `say` "whenever any `clm draft --finish` the loop runs prints it", and the build notes for wave 4 ask finish_gone_drafts to say them. But orchestrator.loop/draft-of-gone-items-dropped (4dafc4af) says the finish of a draft whose items are gone passes each `left <path>: ...` line to a `say` "and print no other action for it", which read literally forbids a `say` for its `kept` lines. The code (crates/clm-loop/src/step.rs, finish_gone_drafts) follows return-rules. A spec change should reword draft-of-gone-items-dropped to say its `kept` lines are said as return-rules says, and its build re-cite tests/draft_gone_items.rs::a_printed_draft_whose_only_item_a_build_resolved.
