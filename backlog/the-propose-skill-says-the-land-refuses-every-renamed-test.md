---
title: "The propose skill says the land refuses every renamed test"
kind: deferred
components: [plugin]
---

`plugin/skills/propose/SKILL.md` says "`clm land` refuses a build that deletes, renames or un-cites a test no list on `main` names." Once `land/deleted-tests-approved` and `check.coverage/test-lists` are current at the versions `20261118-renaming-a-citing-test` gives them, a test renamed in its own file that still cites every requirement it cited is not refused; reword the sentence to say so. It is prompt text (`prompt-text-is-source`), so a spec branch can't carry the edit.
