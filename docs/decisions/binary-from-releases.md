---
title: "The plugin gets clm from GitHub Releases, through a shim"
date: 2026-10-23
status: accepted
---

Answers `cli-distribution`. Teams shouldn't need a Rust toolchain to use CLM, so a release workflow attaches prebuilt `clm` binaries to each GitHub Release, and the plugin's `bin/clm` shim downloads the one matching the plugin's version on first use, verifies its checksum and caches it under `~/.cache/clm/<version>/`. Claude Code puts a plugin's `bin/` on the Bash tool's PATH, so skills and Claude call `clm` by name.

- Downloading uses `gh release download`, because the repo is private and every CLM user already has `gh` authenticated for PRs (`reuse-first`).
- The plugin's version and the `clm` workspace version are the same number, enforced by a test, so a plugin always runs the binary built from the same commit.
- Releases are cut by a human dispatching the release workflow, which bumps the version, builds and publishes (`release-by-dispatch`, which replaced publishing a GitHub Release by hand).
- Where there's no prebuilt binary for the platform, the shim says how to `cargo install` the tagged version. Setting `CLM_BIN` points the shim at any binary, which is how CLM's own repo uses its working build.
- Releases stay private, so only accounts whose `gh` can read `clunny/clm` get a binary (accepted 2026-10-30, `20261030-public-releases`). CLM opens to the public when the owner judges a version worth getting people to test; making releases public (a public mirror, or a public bucket with checksums so the shim needs no `gh`) is a change of its own then, to `plugin`'s shim and `ci`'s release workflow. The Windows check `windows-paths-in-shim-and-panel` describes runs after CLM goes public (decided 2026-11-03, `backlog/windows-check-after-going-public`), so the first outside testers on Windows may meet failures, which they report with `clm defect`.

Rejected: `cargo install` as the main path (needs Rust, slow first run); committing binaries to the repo (bloats git history).
