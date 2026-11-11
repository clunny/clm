---
title: "Init skill writes [build] setup on the first spec branch, which the gate classifies as mixed"
kind: bug
components: [plugin.init]
---

The init skill (`plugin/skills/init/SKILL.md`, the `tests-run-in-their-own-tree` paragraph) tells the session to write the project's `[build] setup` into `clm.toml` "on the first spec branch". `clm init` puts `clm.toml` in the root component's `paths`, so the gate counts the edit as code and classifies the spec branch `mixed`. Following the skill as written makes the first spec branch unlandable until the `clm.toml` edit is moved to a code branch of its own.

Expected: the skill puts the setup line in the bootstrap commit, as it does `[verify]` and the check script, or lands it as a small code change before the spec branch.
