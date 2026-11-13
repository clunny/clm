---
title: "The propose skill and some specs still teach a widened type's forced fix, which a build may no longer make in another component"
kind: bug
components: [plugin, config, host]
---

## Problem

`clm draft`'s prompt now sequences a widened type through the user component's default arm marked `clm:remove-by` (`coupled-changes-sequence`, `path-ownership`). `plugin/skills/propose/SKILL.md` (pinned by `crates/clm/tests/plugin.rs`) still says the widening's build adds the arm as a forced fix (`forced-fixes-cover-additions`), and `specs/config/SPEC.md` and `specs/host/SPEC.md` requirements (`host-setting`, the `azure-*` ones) still cite it. A session reading both gets contradicting rules.

Bring the skill, its tests and those requirements to the sequencing rule.
