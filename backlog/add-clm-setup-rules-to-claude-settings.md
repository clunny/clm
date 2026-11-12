---
title: "Add the clm setup rules to .claude/settings.json's permissions.allow"
kind: human-task
components: [clm]
blocks: [clm/session-commands]
---

`clm/session-commands` now requires `.claude/settings.json`'s `permissions.allow` to hold `Bash(clm setup)` and `Bash(clm setup *)`. An unattended build's edit to that file was refused by Claude Code's auto mode (it widens permissions), so a human must add both entries after `Bash(.github/scripts/ci-check.sh *)`. The build then adds both to `TOOLCHAIN` in `crates/clm/tests/toolchain.rs` and re-cites the two `session-commands` markers at `562aaa70`.
