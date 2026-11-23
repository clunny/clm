---
status: done
resolves: [draft-of-gone-items-dropped-forbids-the-kept-say-that-return]
---

## Intent

Two requirements of `orchestrator.loop` disagree on the `kept ` lines of the finish the loop runs for a draft whose items are gone. `return-rules` says the loop passes each line starting `kept ` to a `say` "whenever any `clm draft --finish` the loop runs prints it", and wave 4's build notes ask `finish_gone_drafts` to say them. `draft-of-gone-items-dropped` says that finish passes each `left <path>: ...` line to a `say` "and print no other action for it", which, read literally, forbids the `say` of its `kept` lines. The code (`crates/clm-loop/src/step.rs`, `finish_gone_drafts`) follows `return-rules`, and `tests/draft_gone_items.rs::a_printed_draft_whose_only_item_a_build_resolved` already asserts the `kept` line is said.

This change rewords `draft-of-gone-items-dropped` so it names the `kept` lines beside the `left <path>` lines, said as `return-rules` says for every `clm draft --finish` the loop runs. "Print no other action for it" stays: no `launch`, `stop` or `resume` is printed for the dropped draft.

## Approaches

- **Reword `draft-of-gone-items-dropped` to name the `kept` lines (chosen).** `return-rules` is the general rule for what the loop says of a finish's lines, and the code and its test already follow it; the narrower requirement only failed to mention the `kept` lines. A worktree a finish keeps is something the human may need to clear, so saying it is right whichever finish ran.
- **Narrow `return-rules` to exclude this finish.** It would hide a kept worktree from the human for no gain and reverse code and a test that hold now. Not taken.

## Outcome

- `orchestrator.loop/draft-of-gone-items-dropped`: the finish's `left <path>: ...` lines and its lines starting `kept ` each go to a `say`, as `return-rules` does for every `clm draft --finish` the loop runs; no other action is printed for the run. No scenario changes: a scenario whose outcome changed would be a cut the human reviews, and the rule is a rewording that brings this requirement in line with `return-rules`, which the code and its test already follow.
- No code changes: `finish_gone_drafts` already says the `kept` lines (`crate::returned::kept`). The build re-cites the requirement's tests at its new version, `tests/draft_gone_items.rs::a_printed_draft_whose_only_item_a_build_resolved` among them, which already asserts that a `kept` line the finish prints is said.

Edge cases and workarounds the change knows it needs: none.
