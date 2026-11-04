---
title: "The guard counts cargo run of the clm package as running clm"
date: 2026-10-26
status: superseded
superseded_by: guard-asks-any-cargo-run
---

Refines `approve-in-session` and `defect-reports`. The guard found a `clm` call only when a segment's first token was `clm` or ended in `/clm`. This repository's `CLAUDE.md` tells sessions to run CLM as `cargo run -q -p clm -- <args>`, so `cargo run -q -p clm -- approve spec/x` recorded an approval without the human confirming it, and `cargo run ... -- defect` filed a public issue without asking. The guard treats `bash -c` as obfuscation it doesn't follow, but this is the documented way to run CLM here.

So a segment also runs `clm` when it runs `cargo run` (or `cargo r`, with any `+<toolchain>` and options before it) and selects the package or binary `clm` with `-p`, `--package` or `--bin` before the first `--`; `clm`'s arguments are the tokens after that `--`, or, with no `--`, `run`'s positional arguments (skipping the values of options known to take one), since `cargo run` passes those to the binary too. `cargo`'s own options before `run` (`--color never`, `+stable`) don't hide it. Every guard rule that looks for a `clm` call uses the one definition, so `clm approve`, `clm review <id>` and `clm defect` are all asked about.

Alternatives considered:
- Treat any `cargo run ... -- approve` as `clm approve`: a different binary's `approve` subcommand would be asked about, and a workspace with one binary run without `-p` is the rare case, not this repository's.
- Change `CLAUDE.md` to run an installed `clm` instead: sessions building CLM need the checkout's binary, and the guard should cover the documented command rather than the documentation working around the guard.
- Follow `cargo run --manifest-path .../clm/Cargo.toml`: not a documented form; like `bash -c`, it stays a gap in a backstop for accidents.
