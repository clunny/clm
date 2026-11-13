---
title: "The builder-check requirement's text lives in a file the propose skill's component owns"
kind: question
components: [plugin, plugin.propose]
blocks: [plugin/builder-check]
---

`plugin/builder-check` now says `skills/propose/SKILL.md` tells Claude that no build edits another component's code or tests (`path-ownership`) and that coupled changes are written in steps (`coupled-changes-sequence`), replacing the `forced-fixes-cover-additions` paragraph, which the skill still holds. The test that proves the requirement (`crates/clm/tests/plugin.rs`, markers citing `2cf8bddc`) reads that text from the skill, and the skill file is `plugin.propose`'s (`plugin/skills/propose/**`), which `plugin`'s build may not edit (`path-ownership`). No `plugin.propose` requirement carries the builder-check wording: its pending `propose-nests-splits` states only the split paragraph. Which component's requirement should state the paragraph?
