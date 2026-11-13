---
title: "context's bundle-contents requirement text is cut off mid-sentence by a scenario"
kind: bug
components: [context]
---

In `specs/context/SPEC.md`, the new `test dependency's contract` scenario sits inside the first sentence of `bundle-contents`, where the `#### Serves` mention was, so the statement reads cut off. Also the Design's list is cut off at "before its contract, `#" and the Contract says only "each component it depends on directly", with no mention of `test_depends_on`. This needs a spec change to fix the text: no code fix can restore it.
