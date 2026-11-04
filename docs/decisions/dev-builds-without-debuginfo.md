---
title: "Dev builds of this workspace carry no debug info"
date: 2026-10-28
status: accepted
---

The workspace's `[profile.dev]` sets `debug = 0` (`clm/dev-builds-without-debuginfo`), so every machine, session, build worktree and land check builds without debug info. The release profile already has none. The `wasm32-unknown-unknown` builds in `ci-check.sh` use the dev profile too, so they lose it as well; nothing reads it there either. `cargo llvm-cov`, which `coverage.sh` runs, instruments with `-C instrument-coverage` and needs no debug info.

**Why.** A cold `cargo build --all-targets` of the workspace (24 cores, sccache off, 2026-10-28) took 352 s of CPU with the default full debug info, 255 s with `line-tables-only` and 216 s with `debug = 0`: 39% less, and its target directory was 2.5 GB instead of over twice that. Most of the saving is linking: each of the workspace's 168 integration test binaries links its library's debug info. Nothing here uses debug info: no debugger configuration exists, no session has run `gdb` or `lldb`, and a failing test reports its assertion and its file and line without it, which is what a session reads to fix it. Hosted CI already built this way: `ci.yml` and `release.yml` set `CARGO_PROFILE_DEV_DEBUG: 0`, and when checks moved to `clm land` on this machine (`land-locally`) the setting stayed in the release workflow alone, so every land, build and session since compiled with full debug info.

**Why in `Cargo.toml`.** It reaches every clone with no per-machine step, and cargo applies a workspace's profiles wherever it is built from. A per-machine `~/.cargo/config.toml` would have to be repeated on each machine, and an environment variable in `ci-check.sh` would leave sessions and builds, which run cargo directly, building with debug info.

Rejected:
- **`debug = "line-tables-only"`**, which keeps line numbers in backtraces: 18% more CPU than `debug = 0` for backtraces nobody reads; a panic's own message names its file and line.
- **Merging each crate's integration tests into one binary** (the backlog item this change resolves): it would cut the link count from 168 to about 26, but it is a requirement on every node whose crate has integration tests, four test files rely on a process of their own, and a test file left out of its crate's module list would stop running without failing anything. Debug info was most of what each link cost.
- **A shared build cache for the workspace's own crates.** sccache caches no linked binary and keys Rust compiles by absolute path (its `basedirs` covers C and C++ only, as of 0.19), and cargo's own cross-workspace cache is still a 2026 goal (rust-lang/goals#626).

A developer who wants a debugger session overrides it for one build with `CARGO_PROFILE_DEV_DEBUG=2`.
