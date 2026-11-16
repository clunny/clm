---
title: "The rebase call takes the name it was run by, since warnings-name-the-command bars the crate from spelling it"
kind: workaround
components: [sync]
requirement: sync/commands-in-crate@77d09cc0
---

`sync/commands-in-crate` has `clm-sync`'s call write everything `clm rebase` prints, with the prefix the command gives each line (`clm rebase: <warning>`), and the clap usage names the command. `sync/warnings-name-the-command` says `clm-sync` holds no string literal naming `clm rebase`, and its test reads every file under `crates/clm-sync/src` for one. To satisfy both, `rebase_command` takes a further argument, the name the command was run by (`rebase`), which `crates/clm` passes, and builds the prefix, the outcome line and the usage name from it at run time. The Contract names no such argument. A spec change should reconcile the two: either drop `warnings-name-the-command`'s bar on the literal now that the crate owns what the command prints, or state the name argument in the Contract.
