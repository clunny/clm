---
status: done
resolves: [add-clm-setup-rules-to-claude-settings]
---

## Intent

Delete the backlog item `add-clm-setup-rules-to-claude-settings`, which is already done on `main`: commit `74a6fdb6` ("Allow clm setup in .claude/settings.json", merged as `a4f02b79`) added `Bash(clm setup)` and `Bash(clm setup *)` to `.claude/settings.json`'s `permissions.allow` after `Bash(.github/scripts/ci-check.sh *)`, at the human's request, added both to `TOOLCHAIN` in `crates/clm/tests/toolchain.rs`, and re-cited both `session-commands` markers at `562aaa70`, so `clm/session-commands` is current. This change only deletes the item from the backlog.

## The human's answers

> Can you add them for me?

The rules were added at that request by `74a6fdb6`, so nothing is left to design.
