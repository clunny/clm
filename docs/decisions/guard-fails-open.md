---
title: "The guard fails open when its binary is missing or crashes"
date: 2026-10-27
status: accepted
---

Refines `say-guard-is-off` and `hook-logic-in-clm`. Claude Code blocks a tool call only when a `PreToolUse` hook exits 2, so when `clm hook guard` can't run, the call goes through: a `$CLM_BIN` that a `cargo clean` or a relink removed makes `sh` exit 127, and a panic in the guard exits 101. CLM keeps it that way. The guard has no fallback that exits 2 when its binary can't run, and the shim takes no copy of the binary.

**Why.**
- The guard is a backstop for accidents, not a defence against an adversary (`hook-logic-in-clm`). Removing its binary isn't a way the accidents it exists for happen: a skill or a habit reaching for `git push origin main`, `clm approve` or a release command.
- Failing closed would block every `Bash` call, and the session would then have no way to rebuild or reinstall the binary, since `cargo build` and `cargo install` are `Bash` calls too. A `cargo clean` would end the session. That is the outcome `say-guard-is-off` already rejected for a missing binary.
- A wrapper that exits 2 only on some commands would need the guard's rules in shell, which `hook-logic-in-clm` moved into `clm` so they are tested.
- `$CLM_BIN` exists to run the developer's latest local build. Copying it when the session starts would pin a session to the build it started with and drop the rebuilds the developer meant to pick up.

**What is left open, and where.** Headless sessions are gone (`builds-in-subagents`): `clm run` no longer starts a session with its own `--settings` hook running `'<clm>' hook guard` from the launching binary's path. Builds and drafts are subagents of the human's session, guarded by the plugin's hook through the shim, so the only path that can fail open is the shim's. Without `$CLM_BIN`, the shim runs the cached release binary, which only the shim writes, and when there's none at session start it says the guard is off (`say-guard-is-off`). If the cached binary is deleted during a session and downloading it again fails, the guard exits 0 without a word, as that decision chose. The other gap is a developer's own `$CLM_BIN` removed or broken during a session, which they set and can see.

**Alternatives considered.**
- Copy the binary to `.clm/bin/` or the cache before launching. This was the fix for headless launches from `target/debug/clm`, which no longer exist. For `$CLM_BIN` it breaks what the variable is for.
- A `sh -c` wrapper in `hooks.json` that exits 2 when the binary can't run. It turns an accident in the developer's build into a session that can't run any command, including the one that would repair it.
- Catch a panic in `clm hook guard` and exit 2. A panic is a bug in the guard's parsing of one command; blocking that command without saying why sends the session hunting for a workaround. The panic's message goes to stderr, where Claude Code shows it, and the fix is a code change with a test.
