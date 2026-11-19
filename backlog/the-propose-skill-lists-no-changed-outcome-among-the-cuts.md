---
title: "The propose skill lists no changed outcome among the cuts"
kind: deferred
components: [plugin]
---

`20261118-a-scenario-whose-outcome` has `gate/cuts` list a scenario whose outcome (its lines from `- THEN` on) changes under its name, which makes the branch need review. `plugin/skills/propose/SKILL.md`, line 7, lists what makes a spec branch need review as "a requirement or a scenario that is gone, a requirement that states fewer sentences holding `SHALL`, or a test a change lists to delete", and leaves the new cut out, so a proposing session doesn't learn that rewording a scenario's outcome makes its branch need review. Add "a scenario whose outcome changes under its name" to that list, worded as `docs/guide/how-it-works.md` words it. Prompt text, fixed by component with no test (`prompt-text-is-source`), once that change is applied on `main`.
