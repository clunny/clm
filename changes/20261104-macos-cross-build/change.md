---
status: applied
---

# Cross-build the macOS release binaries on Linux

## Intent

The release builds `aarch64-apple-darwin` and `x86_64-apple-darwin` on `macos-latest` runners, which are hard to get. This change builds them on `ubuntu-latest` with `cargo zigbuild`, as the two musl targets already are, so every release job runs on a Linux runner and Zig is the one C compiler and linker. Each macOS build sets `MACOSX_DEPLOYMENT_TARGET` to the oldest macOS Rust supports for its target (`11.0` and `10.12`), so no Mac that runs today's binaries loses them. The change is recorded as the decision `cross-built-macos-binaries`, and `host-ci-for-releases` is amended: its reason for keeping the release on Actions (macOS can't be built here) no longer holds, but moving the release off Actions is left for a later change.

## The human's answers

> I like the cross-compile idea.

The human chose it over a self-hosted runner: their only spare Mac runs plain Ubuntu, so it would register as a Linux runner and could take no macOS job.

## Evidence

Gathered on 2026-11-04 on this Linux machine, which has no macOS SDK, with Zig 0.17.0 and `cargo-zigbuild` 0.24.0, from `origin/main`:

- `cargo zigbuild --release --locked -p clm --target aarch64-apple-darwin` built a Mach-O arm64 executable linking only `/usr/lib/libSystem.B.dylib`, `libiconv.2.dylib` and `libcharset.1.dylib`, with an embedded `LC_CODE_SIGNATURE`. The build warned that `xcrun` and the SDK were missing, and succeeded.
- The human ran it on an Apple Silicon Mac: `codesign -dv` printed `flags=0x20002(adhoc,linker-signed)` and `Signature=adhoc`, `--version` printed `clm 0.15.0 (c791145d8876)`, and `survey --help` ran, so the tree-sitter C code linked.
- Without `MACOSX_DEPLOYMENT_TARGET` the binary was marked minimum macOS 15.0. With `11.0` the `aarch64` build was marked `minos 11.0`; with `10.12` the `x86_64` build was marked `LC_VERSION_MIN_MACOSX 10.12`.

## Approaches

- **Cross-build with `cargo zigbuild` on `ubuntu-latest`** (chosen). One mechanism for all four targets, already pinned and tested for musl; no runner to keep; free on a public repository.
- **A self-hosted runner.** The human's spare Mac runs Ubuntu, so it would be a Linux runner; and a self-hosted runner on a public repository that takes fork pull requests runs the host's code on the human's machine.
- **Keep `macos-latest`.** The runners are hard to get, which stalls releases.
- **`osxcross` with a macOS SDK.** Needs Apple's SDK, which Apple's licence restricts to Apple hardware, and a second mechanism beside Zig.

## Outcome

`release.yml`'s build matrix runs every target on `ubuntu-latest`; Zig and `cargo-zigbuild` are installed for every target; the plain `cargo build` step and the `shasum` fallback in the pack step, which only macOS needed, are removed (`no-dead-code`). The `release` requirement and the `ci` Design say so.

## Constraints

- Signing and notarizing are out of scope, as now: the release has never signed. The linker's ad-hoc signature is what Apple Silicon needs to run the binary; a downloaded binary still gets the quarantine attribute a browser sets, and the shim downloads with `curl`, which sets none.
- `land-locally`: nothing changes in how the release reaches `main`.
