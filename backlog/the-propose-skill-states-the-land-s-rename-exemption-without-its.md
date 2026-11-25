---
title: "The propose skill states the land's rename exemption without its uncited-test condition"
kind: bug
components: [plugin]
origin: 20261124-land-s-deleted-test
---

`plugin/skills/propose/SKILL.md:31` says `clm land` refuses a renamed test "unless it is renamed in its own file to a name no test carried and still cites every requirement it cited". `check.coverage/test-lists` also refuses that rename when an uncited test leaves the same file on the branch (deleted, moved, or renamed to a name the merge base carries), since a gone uncited test takes a new name first. `land/deleted-tests-approved`'s advice says so after 20261124-land-s-deleted-test, as does `docs/guide/how-it-works.md`. The skill's sentence should name the same condition. Prompt text, so a fix by component with no test (`prompt-text-is-source`). Found by the builder check of 20261124-land-s-deleted-test.
