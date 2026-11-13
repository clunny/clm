---
title: "The root spec doesn't declare orchestrator.loop as a dependency, so crates/clm can't dispatch clm build"
kind: bug
components: [clm]
blocks: [orchestrator.loop/step-actions]
---

`clm build` is dispatched by `crates/clm` to the crate `clm-loop`, which the root's Design says (`clm build` to `clm-loop`), and the build of a component that gains a command edits `crates/clm/src/main.rs` and `Cargo.toml` (the root's Design). But the root's `depends_on` in `specs/SPEC.md` doesn't list `orchestrator.loop`, so adding `clm-loop` to `crates/clm/Cargo.toml` makes `clm check` fail with `check/undeclared-imports: imports crates/clm-loop/Cargo.toml of component orchestrator.loop, and clm doesn't depend on orchestrator.loop`, and `clm land` refuses the branch. The `20261112-build-loop-in-clm` change edited the root's Design but not its `depends_on`.

The fix is a spec change: a delta on `clm` that adds `orchestrator.loop` to the root's `depends_on`. The build of `orchestrator.loop` withheld the dispatch (the `clm build` clap subcommand and `build_loop` function in `crates/clm/src/main.rs`, the `clm-loop` dependency in `crates/clm/Cargo.toml` and `[workspace.dependencies]`, the `C::Build { root, .. }` arm in `crates/clm/src/upgrade_gate.rs`) and the move of the capability test `build_from_spec.rs` onto `clm build start`, `step`, `launched` and `returned`, so `orchestrator.loop/step-actions`, whose text names that move, stays unbuilt until this lands. `clm_loop::Options` takes `clm`, `root`, `max_builds` and `agent_harness`; `crates/clm` passes `clm_agent_harness::detect(process env)`.
