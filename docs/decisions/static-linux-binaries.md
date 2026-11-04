---
title: "Linux releases are static musl binaries, built with cargo-zigbuild"
date: 2026-10-27
status: accepted
---

The release built `*-unknown-linux-gnu` on `ubuntu-latest`, so its binaries needed that runner's glibc and failed on Ubuntu 22.04 and Debian 12, which then fell back to `cargo install`. The release now builds `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` instead: static binaries that run on any Linux of their architecture.

- `clm`'s Rust dependencies are pure Rust; the one C dependency is `clm survey`'s tree-sitter grammars, which the `cc` crate compiles at build time. Each musl target therefore needs a C compiler and linker for musl.
- Both targets build on `ubuntu-latest` with `cargo zigbuild`, Zig serving as C compiler and linker. One mechanism covers both architectures, with nothing from `apt`; Zig comes from PyPI and `cargo-zigbuild` from crates.io, each pinned to an exact version.
- The `-gnu` targets are dropped, not kept beside musl (`no-dead-code`). A plugin version's shim downloads only its own version's release, so no shim that names a `-gnu` asset reads a release built after the switch.
- The switch is ordered by `shim-assets-before-release`: a release whose shim names an asset the build matrix doesn't build fails before it builds anything.

Rejected:
- `musl-tools` for x86_64 and a separate cross toolchain for aarch64: Ubuntu packages no aarch64 musl cross compiler, so it would be two mechanisms.
- A native arm64 runner (`ubuntu-24.04-arm`): its availability to private repositories depends on the account's plan, and it would cover only aarch64.
- `cross` (Docker images): heavier, and a second container layer on a hosted runner for what Zig does in-process.
- Keeping `-gnu` and building it on an older runner: still ties the binary to a glibc floor and to a runner image GitHub retires.
