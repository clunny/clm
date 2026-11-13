---
status: applied
resolves: [the-root-spec-doesn-t-declare-orchestrator-loop-as-a-dependency]
---

## Intent

The root's `depends_on` gains `orchestrator.loop`, so `crates/clm` may import `clm-loop` and dispatch `clm build` to it, as the root's Design already says. The `20261112-build-loop-in-clm` change edited that Design but not the edge, so `clm check` fails the dispatch with `check/undeclared-imports` and `orchestrator.loop/step-actions` can't be built. The edge is the whole fix, so this branch also deletes the item, which would otherwise block `orchestrator.loop/step-actions` until the change is archived.

## Problem

The build of `orchestrator.loop` withheld the dispatch (the `clm build` subcommand and its `build_loop` function in `crates/clm/src/main.rs`, the `clm-loop` dependency in `crates/clm/Cargo.toml` and `[workspace.dependencies]`, and the `C::Build` arm in `crates/clm/src/upgrade_gate.rs`), because adding the dependency made `clm check` report `check/undeclared-imports`, and `clm land` refused the branch.

## Approaches

- **Add the edge (chosen).** `depends_on` declares what a component's code may import; the root dispatches every command and already lists every other crate it dispatches to.
- **Dispatch without importing** (run `clm-loop` as its own binary). Against the root's Design, which dispatches every command in-process, and more code for nothing (`yagni`).

## Outcome

The root's frontmatter lists `orchestrator.loop`. No requirement changes, so nothing becomes pending: the dispatch is work `orchestrator.loop/step-actions` already names, and its build adds it (the root's Design: the build of a component that gains a command edits `crates/clm/src/main.rs` and `Cargo.toml`). The edge forms no cycle: `orchestrator.loop` depends on nothing that depends on the root.

Edge cases and workarounds: none.
