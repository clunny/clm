---
component: plugin
---

## MODIFIED Requirements

### Requirement: shim-targets <!-- base: ac8086e1104c4bb4 -->
`plugin/bin/clm` SHALL download `clm-x86_64-unknown-linux-musl.tar.gz` on `Linux x86_64`, `clm-aarch64-unknown-linux-musl.tar.gz` on `Linux aarch64`, `clm-x86_64-apple-darwin.tar.gz` on `Darwin x86_64` and `clm-aarch64-apple-darwin.tar.gz` on `Darwin arm64`, and map no other platform. On any other platform, Windows's Git Bash and MSYS2 shells included (`no-windows-support`), it SHALL download nothing and fail as `binary-shim` describes, saying there's no prebuilt `clm` for that platform. Each target it maps SHALL be written as one `target=<target>` assignment, which `ci/release`'s prepare step reads. Its text SHALL name no Windows target and no `clm.exe`.

#### Scenario: Linux arm64
- GIVEN an empty cache, a `uname` that reports `Linux` and `aarch64`, and a `gh` that serves release assets
- WHEN `bin/clm status` runs
- THEN `gh` is asked for `clm-aarch64-unknown-linux-musl.tar.gz`

#### Scenario: unsupported platform
- GIVEN an empty cache and a `uname` that reports `FreeBSD` and `amd64`
- WHEN `bin/clm status` runs
- THEN `gh` is not called, and it exits 1 saying there's no prebuilt `clm` for that platform

#### Scenario: Windows unsupported
- GIVEN an empty cache, and a `uname` that reports `MINGW64_NT-10.0` and `x86_64`, and in a second run `UCRT64_NT-10.0-22631` and `x86_64`
- WHEN `bin/clm status` runs
- THEN `gh` is not called either time, and each run exits 1 saying there's no prebuilt `clm` for that platform

#### Scenario: no Windows target in the shim
- GIVEN the text of `plugin/bin/clm`
- WHEN the tests run
- THEN its `target=` assignments are exactly the four targets above, and the test fails when the text names `windows` or `clm.exe`

#### Serves
- capability/install-and-release

#### Needs
- ci/release

## REMOVED Requirements

### Requirement: shim-path-conversion <!-- base: 98bed5b82837e0b0 -->
