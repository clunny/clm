---
title: "The triage skill says clm draft --finish deletes every branch of a draft"
kind: deferred
components: [plugin]
origin: 20261123-wave-5-plugin-by-the-rule
---

`plugin/skills/triage/SKILL.md` step 7 ("removes the draft's worktree and deletes its branches") and step 9 ("which removes its worktree and deletes its branches now that its change is on `main`") say `clm draft --finish` deletes the draft's branches, where `orchestrator.triage/draft-finish` keeps a local `spec/<change>` with commits on neither `origin/main` nor `origin/spec/<change>`, printing `kept spec/<change>: commits not on origin`. Add that exception to both, and tell the session to pass a `kept` line to the human. Prompt text, fixed by component with no test; resolve with the trailer.
