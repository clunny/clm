---
title: "The guard asks about any cargo run whose tokens hold an approval word"
date: 2026-10-30
status: accepted
---

Supersedes `guard-sees-cargo-run`, which counted a `cargo run` as running `clm` only when it selected the package or binary `clm` with one of a listed set of spellings (`-p clm`, `--package=clm`, `--bin clm`...), and then worked out `clm`'s arguments by skipping the values of a listed set of `run` options. The build's code review found spellings that run `clm` and that the lists leave out: `-p=clm`, package IDs such as `-p clm@0.7.0`, the package taken from the directory (`cd crates/clm && cargo run -- approve x`) or from `--manifest-path`, and any `run` option that takes a value and isn't listed, now or in a later cargo. Each gap is an approval recorded, or a defect filed, without the human confirming it. Patching the lists would chase cargo's grammar for ever; the lists were the wrong design.

So the guard no longer works out which package a `cargo run` runs, or which of its tokens reach the binary. A segment whose command is `cargo` and whose subcommand is `run` or `r` is asked about whenever `approve`, `defect`, or `capabilities` followed by `decide`, is one of its tokens after the subcommand. Asking too often is the safe side of a backstop: the cost of a false ask is one confirmation, and the cost of a missed one is an approval nobody gave. This removes the package and option lists from `approve-guard` (`YAGNI`, `Redesign before you work around`).

Accepted false asks: another binary's `approve` or `defect` subcommand run through `cargo run`, and a `cargo run -p clm` call one of whose values is the word itself, such as `--title approve`. Both are rare in this repository and cost one confirmation each.

Alternatives considered:
- Widen the lists (`-p=`, `clm@<version>`, `--manifest-path` naming `crates/clm`, the directory's package): still a list of cargo's spellings, and the directory and an unknown option's value can't be listed at all.
- Ask about every `cargo run`: it would ask on each `cargo run -q -p clm -- status`, which sessions here run all the time.
