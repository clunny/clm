---
title: "rust-toolchain.toml pins the Rust toolchain and installs the wasm target"
date: 2026-10-22
status: accepted
---

`rust-toolchain.toml` pins `channel` to an exact version (1.99.1 when this was decided), with `components = ["rustfmt", "clippy"]` and `targets = ["wasm32-unknown-unknown"]`. CI installs the toolchain from that file instead of `dtolnay/rust-toolchain@stable`, and the release workflow adds its cross targets on top.

**Why a file, not a manual step.** `20261022-web-ui`'s tasks build crates for `wasm32-unknown-unknown`, and unattended sessions can't run `rustup`. rustup reads the file whenever `cargo` runs and installs a missing toolchain, component or target first. We tested it on 2026-10-22: with the target listed, `cargo --version` installed `wasm32-unknown-unknown` into an existing toolchain. So every machine, `clm run` worktree and CI job gets the target with no setup and no new permission.

**Why exact.** With `channel = "stable"`, the same test also moved that machine's `stable` from 1.93.0 to 1.99.1, because rustup syncs the channel when it installs a target. A floating channel also let a session pass clippy locally and fail CI on a newer lint (`backlog/pin-rust-toolchain`). An exact pin means nothing moves without a PR, and moving it is a code PR that fixes new lints in the same diff.

**Alternatives.** A one-time `rustup target add` by the human works on one machine, isn't recorded and doesn't help CI. Allowing sessions to run `rustup` lets them change the machine's toolchains unattended.
