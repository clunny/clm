---
title: "Doc-only CI jobs run `main`'s `clm` from the Actions cache, without a toolchain"
date: 2026-10-23
status: superseded
superseded_by: land-locally
---

With `light-ci-for-doc-prs`, a doc-only PR's jobs run only `clm check` and the gate, but each job still spent about 20–25s of its ~30s installing the toolchain, restoring the Rust cache and compiling `clm` (measured on 2026-10-23; compiling itself was only 4–9s). `cache.yml` now builds `clm` on `main` and saves the binary in the Actions cache, and the `check`, `gate` and `merge` jobs restore it on a doc-only diff and skip the toolchain.

The cache key is the *code key*: a hash of the commit's tree without the doc-only paths. It changes when anything that isn't doc-only changes, so a build input nobody thought to list can't leave a stale binary in use, and a doc-only merge keeps the cached binary valid. A job runs the binary only while its key equals the code key of the `main` it checks against; otherwise it builds `clm` from source as before. So the cache only ever saves time: a miss, an eviction or a moved `main` never skips a check.

The light path runs `clm check --no-detectors`. The only detector, `cargo-machete`, reads code and manifests, which on a doc-only diff are `main`'s and passed it when they landed. A future detector that reads doc-only files must change the light path when it is added.

The light `gate` job runs `main`'s `clm`, not the PR head's: a doc-only head's code is its merge base's, and `main`'s rules are the ones the merged tree lives under.

Rejected:
- **A workflow artifact** fetched with `gh run download`: needs `actions: read`, a lookup of the right run and retention handling, for the same result.
- **A rolling prerelease of `main`**: publishes internal binaries and clutters the releases.
- **Keying on `main`'s commit**: every doc-only merge would invalidate the binary until `cache.yml` rebuilt it.
- **A prebuilt `cargo-machete` as well**: keeps a detector that can find nothing on a doc-only diff, and pins another binary.
