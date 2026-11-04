---
title: "CI's approval pinning and merging are clm subcommands"
date: 2026-10-22
status: superseded
superseded_by: land-locally
---

`pinned-approval` and `merge-tested-tree` were implemented as `.github/scripts/pin-approval.sh` and `merge-pr.sh`, which only this repository's CI could run. Adopting repositories need the same steps (`init-cli-and-skill`), and two copies would drift. They move into `clm ci pin-approval` and `clm ci merge` (crate `clm-ci`, node `ci`), and both scripts are removed with their tests.

This repository's `ci.yml` runs them from its own build with `cargo run`, so every PR here exercises the code adopters run. A generated `clm.yml` runs the released binary it names. `clm ci merge` takes the check command as an argument, because the check differs (`.github/scripts/ci-check.sh` here, `clm-build.sh && clm check` elsewhere), and runs the gate as the running binary's own `clm gate`, as the script did.

Rejected: shipping the scripts as files `clm init` writes, which keeps the logic in shell, untestable without a fake repository per script, and copied into every adopter.
