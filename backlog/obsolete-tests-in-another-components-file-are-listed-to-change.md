---
title: "The run prompt tells a build to delete or re-cite an obsolete requirement's tests in a file another component owns"
kind: question
components: [orchestrator.run]
choices:
  - "List them apart, as behind markers"
  - "Leave the prompt as it is"
---

`orchestrator.run/pending-prompt` lists an `obsolete` requirement's markers, and now its tests to delete and to keep, wherever their files sit. Once the root sets `wiring`, `clm land` refuses a build's edit to a file another component owns (`land/build-holds-own-files`), so a test of `b/gone` in `c`'s file that the prompt tells `b`'s build to delete or re-cite can't be changed by that build. A `behind` marker in such a file is already listed apart, with the instruction to leave it and add a `question` item; an `obsolete` one has no such rule. Should the requirement list obsolete markers and tests in another component's file apart the same way?

Found by `/code-review` on the `impl/orchestrator.run` build that brought `pending-prompt` to `c50d1883`.
