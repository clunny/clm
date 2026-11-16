---
title: "No test runs the `[affected] all` settings `crates/clm` passes to `clm affected`"
kind: deferred
components: [clm]
---

Since `check.affected/commands-in-crate`, `clm-affected`'s `command` takes `config`'s `[affected] all` globs from a loader its caller passes, and its tests (`crates/clm-affected/tests/command.rs`) pass a stand-in that reads `clm.toml` with `toml`. The loader `crates/clm/src/main.rs` passes for `clm affected` (`clm_config::load`, then `settings_violation` naming `check.affected/affected-all`) is run by no test with an `[affected]` table, valid or not: the two tests left in `crates/clm/tests/affected.rs` run the binary with an empty `clm.toml`.

So breaking that loader (reading another key, or returning no globs on an error) fails nothing. Due with `clm/routes-only`, whose tests prove that a routed command prints and exits as its call does: one of them should run `clm affected` with `[affected] all` set and with it invalid.
