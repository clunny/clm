---
title: "No test proves the settings clm rebase loads"
kind: bug
components: [clm]
---

Once `sync/commands-in-crate` moved sync's tests into `clm-sync`, the tests that ran the binary on a `clm.toml` it can't load (`[component_size] spec_tokens = 0`, an unknown key) and on `[health] reconsider = 2` call `clm-sync`'s `rebase_command` with a settings closure written in the test. Nothing now proves `crates/clm/src/main.rs`'s `sync_settings`: that it loads `config`'s `health` from the CLM root it is given, and that it formats a load error as `<source>: check.health/health-report: <error>` (`sync/judged-by-check`'s Design). A root test of `clm rebase` on such a `clm.toml` would prove it.
