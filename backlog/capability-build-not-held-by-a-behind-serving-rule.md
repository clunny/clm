---
title: "A capability's build isn't held back by a serving rule that is behind and waiting on other work"
kind: bug
components: [check.coverage, capability, model]
issues: [249, 250]
---

The queue holds a pending capability back only while a rule that serves it has never been built. When a change rewrites a capability's scenario so that its test now exercises a rule that was built earlier but is now `behind`, and that rule can't be rebuilt yet because its own `#### Needs` wait on unbuilt work elsewhere, `clm pending` offers the capability's build anyway. That build either stops at the capability, or re-cites at the new version a test that still proves the old text. Today the only guard is a hand-written backlog item that blocks the capability, listed in the `resolves` of the change that brings the rule up to date.

Expected: the queue doesn't offer a capability's build while a rule serving it is `behind`, or at least while such a rule is itself blocked, so no hand-made hold is needed. Do this without making every edit to an old serving rule hold the capability back, since that is why the decision counts only never-built rules today. The alternative, letting a capability entry name its own needs, is one that `serving-rules-order-capabilities` rejected. Either way that decision, and the wording of `needs-alone-order`, need amending.
