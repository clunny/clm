---
title: "macOS releases are cross-built on Linux with cargo-zigbuild"
date: 2026-11-04
status: accepted
---

The release built `aarch64-apple-darwin` and `x86_64-apple-darwin` on `macos-latest` runners, which are hard to get. Both now build on `ubuntu-latest` with `cargo zigbuild`, as the musl targets do (`static-linux-binaries`).

- `clm` links no Apple framework: its Rust dependencies are pure Rust, and the tree-sitter grammars are C that Zig compiles (`tree-sitter-analysis`). It links only `libSystem`, `libiconv` and `libcharset`, which Zig carries stubs for, so the build needs no macOS SDK.
- Zig's linker gives the `aarch64` binary the ad-hoc signature Apple Silicon needs to run it. A binary built this way was run on an Apple Silicon Mac on 2026-11-04 (change `20261104-macos-cross-build`).
- Each macOS build sets `MACOSX_DEPLOYMENT_TARGET` to `11.0`, the oldest macOS Rust supports on `aarch64` and the oldest Zig 0.17.0 targets. Without it Zig marks the binary as needing macOS 15.0, which would silently drop older Macs. The native `x86_64` build allowed 10.12, below what Zig targets; Claude Code, which CLM runs in, needs a newer macOS than 11, so the higher floor leaves out no Mac that can run CLM.
- A new Apple framework dependency would need the SDK, which Zig doesn't carry; the release's build would then fail to link, not ship a broken binary.

Rejected:
- A self-hosted runner: the spare Mac runs Ubuntu, so it would be a Linux runner, and a runner is a machine to keep online for each release.
- `osxcross` with a macOS SDK: Apple's licence ties the SDK to Apple hardware, and it is a second mechanism beside Zig.
- Keeping `macos-latest`: runners are hard to get, which stalls releases.
