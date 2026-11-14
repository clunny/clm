---
title: "Without a clm binary, hooks stay quiet but session start says the guard is off"
date: 2026-10-21
status: accepted
---

When the plugin's shim can't get a `clm` binary (no release for the platform, no `gh`, a failed download), its hooks can't run. Failing closed would block every Bash command and break the session, so hooks exit 0. But then the `spec-approved` guard is silently off, and neither Claude nor the human knows.

So `clm hook session-start` is the one hook that speaks: the shim prints a line saying the CLM state and the guard are unavailable, with the `cargo install` command. Session-start output reaches Claude's context, so Claude can pass it on to the human. The guard itself stays silent on every Bash call, because a warning per tool call would be noise.

## The rule in full

`ARCHITECTURE.md` holds no entry for this rule, which serves the capability `guard-sessions` (`human-layer-entries-stay-short`); the rule in full:

When the plugin can't get a `clm` binary, its hooks SHALL let calls through, but session start MUST tell the session that CLM's state and the guard are unavailable.

**Scenario: binary missing**
- GIVEN a plugin whose shim can't find or download `clm`
- WHEN a session starts
- THEN it is told the guard is off
