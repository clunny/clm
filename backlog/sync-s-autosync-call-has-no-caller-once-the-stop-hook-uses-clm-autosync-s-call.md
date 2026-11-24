---
title: "sync's autosync(root) has no caller outside its tests once the Stop hook uses clm autosync's call"
kind: deferred
components: [sync]
origin: 20261123-wave-5-hooks-by-the-rule
---

`clm_sync::autosync(root)` exists for `clm-hooks`' `Stop` hook, and it reads `GIT_SSH_COMMAND` from the process's own environment rather than the lookup the hook call is given. Wave 5 (`20261123-wave-5-hooks-by-the-rule`) moves the hook onto `autosync_command`, passing the hook call's environment lookup and streams that discard what it writes (`plugin.hooks/autosync-hook`). Once that build of `plugin.hooks` has landed, only `clm-sync`'s own tests call `autosync`, so it is dead (`no-dead-code`).

Take this up after `plugin.hooks/autosync-hook` is current on `main`. On a spec branch, remove from `sync`'s Contract the bullet "`autosync(root)`, which does what `clm autosync` does … plugin.hooks's `Stop` hook calls it (`autosync`)". From `sync/autosync`, remove the words "and this process's own for `autosync`" and the sentence "`autosync(root)` SHALL do what `clm autosync` does for the CLM root it is given, and return the failure `clm autosync` would print". Losing that SHALL sentence needs the human's yes (`removal-needs-a-yes`); the rest of the requirement stays. The build then removes `autosync` and moves its tests in `crates/clm-sync/tests/sync.rs` onto `autosync_command`. They are kept, re-cited, never deleted.

**Answered by the human on 2026-11-24** (wave 6's opening session): yes, cut it. The SHALL sentence of `sync/autosync` on `autosync(root)` goes with the call, on a small spec branch, and `sync`'s build removes the call and moves its tests onto `autosync_command`.
