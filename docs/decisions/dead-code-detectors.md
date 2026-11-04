---
title: "Each project names its dead-code detectors; clm check runs them"
date: 2026-10-22
status: accepted
---

`no-dead-code` needs a program to enforce it (`deterministic-enforcement`), and CLM builds projects in any language. So CLM doesn't analyse code itself. A repository's `clm.toml` lists detectors, each a command such as `cargo machete`, `knip`, `vulture` or `deptry`, and `clm check` runs them and fails on a non-zero exit (`check/detectors`). Each ecosystem's own tools know its entry points, macros, reflection and framework conventions, which a generic analysis gets wrong (`reuse-first`).

- **Removal is chained.** Deleting a requirement leaves its tests citing a missing ID, which `dangling-markers` reports. Once they go, the detectors see the code they exercised as unused.
- **Repository-only.** Only `clm.toml` sets detectors, never the user's file or a variable, so a developer can't switch a repository's checks off locally (`config/repository-only`).
- **A missing tool fails.** A detector that can't start is a violation with its install hint, rather than a silent pass.
- **Command only, not `clm_check::run`.** Sync, checkpoint and the `web` Worker check trees with no toolchain, so they never run detectors.
- **Suppressions use each tool's own mechanism** (`#[allow(dead_code)]`, `knip`'s ignore list, a `vulture` whitelist). They show in the diff, and the invariant forbids keeping code that serves no requirement.
- **This repository** names `cargo machete` for unused dependencies. Private dead Rust code is already an error through `clippy -D warnings` in `ci-check.sh`.

Rejected:
- **CLM's own tree-sitter reachability analysis** (`tree-sitter-analysis`) as the main detector. It's uniform across languages but shallow, with false positives wherever code is reached dynamically. It stays a backlog idea for the gap no tool covers, public items unused across a workspace (`unreferenced-public-items`).
- **Leaving cleanup to code review.** Review advises (`advisory-code-review`); it can't gate.
