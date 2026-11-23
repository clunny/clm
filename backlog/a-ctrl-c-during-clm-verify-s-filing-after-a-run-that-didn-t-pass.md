---
title: "A Ctrl-C during clm verify's filing after a run that didn't pass still kills clm and leaves the unproven branch"
kind: bug
components: [clm, verify]
---

`clm/verify-command` installs the `SIGINT`/`SIGTERM` handlers that set the call's stop request only once the outcome line starts with `verify passed ` (`crates/clm/src/main.rs`, `verify`'s `announce`). But `verify/files-unproven` files at the end of every `clm verify --due` in `local` mode with `[verify] mutate` set, whether or not a run was due, so the filing also runs after `verify not-due` (the common case at a session's end) and after a red run. There no handler is installed: a Ctrl-C during the filing's land takes the default action and ends `clm verify` itself, so the cleanup `files-unproven` promises (the branch `unproven/<sha>` deleted here and on `origin`, the temporary worktree removed, the `unproven: not filed:` note) never runs, and the branch is left on `origin`.

Expected: the stop window covers the filing whatever the outcome line, for example by installing the handlers for any `--due` call that can reach the filing, so `files-unproven`'s stop and its cleanup hold for the not-due path too. `clm/verify-command` states the window as starting at the `verify passed ` line, so this needs a spec change. Found by the code review of `verify`'s build of `20261122-ctrl-c-during-clm`.
