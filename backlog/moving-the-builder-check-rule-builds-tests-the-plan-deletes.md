---
title: "Moving the builder-check rule to plugin.propose builds string-matching tests the plan deletes"
kind: question
components: [plugin.propose]
origin: 20261113-builder-check-text-lives
blocks: [plugin.propose/builder-check]
---

`plugin.propose/builder-check` states what `plugin/skills/propose/SKILL.md` tells Claude, and its tests check that the skill's text holds certain strings. The plan in `backlog/a-passing-check-proves-less-than-the-spec-claims` deletes requirements of that kind with their tests in its phase 2, so building this one first is work thrown away.

Held on 2026-11-13, with the human, until phase 2 decides it. The obsolete markers of `plugin/builder-check` wait behind it.

Question for phase 2: is `plugin.propose/builder-check` deleted with the other prompt-text requirements, or is some part of it behaviour worth keeping as a requirement? The change that settles it deletes this item.
