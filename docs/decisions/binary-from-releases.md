---
title: "The plugin gets clm from GitHub Releases, through a shim"
date: 2026-10-23
status: accepted
---

Answers `cli-distribution`. Teams shouldn't need a Rust toolchain to use CLM, so a release workflow attaches prebuilt `clm` binaries to each GitHub Release, and the plugin's `bin/clm` shim downloads the one matching the plugin's version on first use, verifies its checksum and caches it under `~/.cache/clm/<version>/`. Claude Code puts a plugin's `bin/` on the Bash tool's PATH, so skills and Claude call `clm` by name.

- `clunny/clm` is public, so its releases are. The shim downloads the release's asset and its checksum with `curl` from `https://github.com/clunny/clm/releases/download/v<version>/`, so installing CLM needs no `gh`, no sign-in, and no mirror or bucket. `curl` ships with macOS and nearly every Linux; where it is missing, the shim fails as for any failed download, printing the `cargo install` command.
- The plugin's version and the `clm` workspace version are the same number, enforced by a test, so a plugin always runs the binary built from the same commit.
- Releases are cut by a human dispatching the release workflow, which bumps the version, builds and publishes (`release-by-dispatch`).
- Where there's no prebuilt binary for the platform, the shim says how to `cargo install` the tagged version. Setting `CLM_BIN` points the shim at any binary, which is how CLM's own repo uses its working build.
- There is no Windows binary (`no-windows-support`).

Rejected: `cargo install` as the main path (needs Rust, slow first run); committing binaries to the repo (bloats git history); downloading with `gh release download` (it needs `gh` installed and signed in, which a public release doesn't call for).
