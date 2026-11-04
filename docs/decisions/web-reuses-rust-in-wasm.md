---
title: "The web Worker runs clm's own Rust, compiled to WebAssembly"
date: 2026-10-22
status: accepted
---

The `web` Worker is Rust (`workers-rs`) built for `wasm32-unknown-unknown`. It uses in-memory entry points that sit next to the existing disk- and git-based ones:
- `clm_model::load_files` to parse repos;
- `clm_check::markers_in` and `clm_check::report` for coverage;
- `clm_orchestrator::queue_from_refs`, which uses `clm_context::bundle_from` to find stale plans, for the claim queue.

So the web app reports exactly what `clm` does, with no second parser to drift from it (`reuse-first`). Each crate builds for WebAssembly, `check` and `orchestrator` with their default features disabled, which leaves out the parts that need a filesystem, git or processes.

The web doesn't recompute the gate tier, which needs git throughout `clm-gate`. It shows the tier declared in `change.md`, and the PR's `gate` check guarantees that's at least what the diff needs.

Alternative considered: running the real `clm` binary in a Cloudflare Container, refreshed by webhook, with a `clm export --json` command. It would touch only the root node, but it needs the Workers Paid plan and has cold starts. The user chose WebAssembly.
