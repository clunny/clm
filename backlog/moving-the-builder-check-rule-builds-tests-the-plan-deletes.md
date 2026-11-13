---
title: "Moving the builder-check rule to plugin.propose builds string-matching tests the plan deletes"
kind: deferred
components: [plugin.propose]
origin: 20261113-builder-check-text-lives
blocks: [plugin.propose/builder-check]
---

`plugin.propose/builder-check` states what `plugin/skills/propose/SKILL.md` tells Claude, and its tests check that the skill's text holds certain strings. The plan in `backlog/a-passing-check-proves-less-than-the-spec-claims` deletes requirements of that kind with their tests in its phase 2, so building this one first is work thrown away.

Held on 2026-11-13, with the human, until phase 2 decides it. The obsolete markers of `plugin/builder-check` wait behind it.

Answered on 2026-11-13, by the human: delete it whole. All five of its scenarios are "the skill lacks a string, so a test fails", and none states behaviour a program observes. The requirement goes with its string-scan tests, in `crates/clm/tests/plugin.rs` and `crates/clm/tests/plugin_propose.rs`. The skill keeps its builder-check step as a plain source file, and its stale `forced-fixes-cover-additions` paragraph is updated as an ordinary edit. Keeping only the step-order part was rejected: it is still a scan of prompt text. Whether propose really runs the check is left to the evals of the propose skill in the plan's phase 6.

The deletion is done in phase 2's change, with the other prompt-text requirements and under the same approval, not on its own. Until then this item keeps the requirement blocked, so no build writes the tests; it is `deferred`, since nothing waits on the human any more. The phase 2 change that removes the requirement deletes this item.

What the paragraph guards against, where a program could decide it, is `backlog/spec-shapes-the-builder-check-warns-about-are-refused-by-nothing`.
