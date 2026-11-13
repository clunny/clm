---
title: "The builder-check requirement's text lives in a file the propose skill's component owns"
kind: question
components: [plugin, plugin.propose]
blocks: [plugin/builder-check]
---

`plugin/builder-check` now says `skills/propose/SKILL.md` tells Claude that no build edits another component's code or tests (`path-ownership`) and that coupled changes are written in steps (`coupled-changes-sequence`), replacing the `forced-fixes-cover-additions` paragraph. Its test in `crates/clm/tests/plugin.rs` reads that text from the skill, and the skill file is `plugin.propose`'s (`plugin/skills/propose/**`), which `plugin`'s build may not edit (`path-ownership`). `plugin.propose` holds no requirement that carries the builder-check wording: its pending `propose-nests-splits` states only the split paragraph.

What spec change settles it? One way: a `plugin.propose` requirement stating the builder-check paragraph's new wording (the sentence naming `coupled-changes-sequence` and its three steps, and no `forced-fixes-cover-additions`), with `plugin/builder-check` needing it, so `plugin`'s build then only re-cites its test at the new version (the markers in `crates/clm/tests/plugin.rs` cite `2cf8bddc`). Another: move the paragraph's requirement to `plugin.propose` altogether.
