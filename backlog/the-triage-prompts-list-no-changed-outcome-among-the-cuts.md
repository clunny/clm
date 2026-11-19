---
title: "The triage prompts list no changed outcome among the cuts"
kind: deferred
components: [orchestrator.triage]
---

`gate/cuts` now lists a scenario whose outcome (its lines from `- THEN` on) changes under its name, and that makes a spec branch need review. The unattended draft prompt in `crates/clm-triage/src/draft.rs` (around line 643) and the fold prompt in `crates/clm-triage/src/fold.rs` (around line 285) list what a branch cuts as renamed or removed scenarios, lost `SHALL` sentences and listed tests, and leave out the new cut. So a draft that rewords a `- THEN` line under the same name expects to land on checks, and doesn't name the change under `## What the requirements no longer state`. Add "a scenario whose outcome changes under its name" to both lists, worded as `docs/guide/how-it-works.md` words it. This is prompt text, fixed by component with no test (`prompt-text-is-source`). It is the triage counterpart of `the-propose-skill-lists-no-changed-outcome-among-the-cuts`. The gate's build found it in review.
