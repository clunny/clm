---
title: "clm defect runs gh with the process's environment, not the environment its call is given"
kind: deferred
components: [plugin.hooks, host]
origin: 20261123-wave-5-hooks-by-the-rule
---

`clm-hooks`' call of `clm defect` takes the environment as a lookup and reads the settings through it (`plugin.hooks/commands-in-crate`). The `gh` it runs through `host`'s `github` adapter to file an issue (`plugin.hooks/defect-issue`) inherits the process's own environment instead: `PATH`, `GH_TOKEN`, `GH_HOST`. A caller that passes another lookup can't steer `gh`, so `crates/clm-hooks/tests/defect.rs` sets `PATH` on its own process under a lock for each call. In production the root passes `process_env`, so nothing differs. This is plumbing, not behaviour a user sees.

Take this up when `host` offers an issue call that takes an environment, as `sync`'s `autosync_command` hands `GIT_SSH_COMMAND` on. `host` adds that call first (expand), then `plugin.hooks` passes its lookup through it. With that, a scenario of `defect-issue` states that the `PATH` of the lookup finds `gh`, and the tests drop their process-wide `PATH`. That is a spec change on both components. No requirement sentence goes.
