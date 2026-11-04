---
title: "CLM ships one detector of its own, for public items nothing outside their library uses, matched by name"
date: 2026-10-27
status: accepted
---

`dead-code-detectors` leaves dead code to each ecosystem's tools, and they leave one gap: rustc's `dead_code` lint assumes a `pub` item may have users outside its crate, so a public function that no other crate in an application workspace calls is never reported. `clm unreferenced` (node `check.unreferenced`) covers it, and a project names it in `clm.toml` like any other detector, so `clm check` runs it and nothing else changes (`check/detectors`).

- **A child node of `check`, with its own crate** (`nested-splits`, `slim-nodes`). It has dependencies `check` doesn't (tree-sitter's C grammars, which don't build for the `wasm32` target `clm-check` must), it is run by `clm check` only through `clm.toml`, never called, and `check` is already over its code budget.
- **Matched by name, not resolved.** An item is used when its name appears as a whole word, outside plain comments, in a tracked Rust file outside its own library; doc comments count, so a doctest is a use. Resolving `use` paths, re-exports, glob imports and method receivers would need the compiler; a name match needs none of it, runs from the tracked files alone, and errs one way only: two items sharing a name hide each other, so it can miss dead code but never reports a used item. Strings count, so names that derives and macros spell as strings (`#[serde(default = "f")]`) are used.
- **"Outside its library" includes its own package's tests, binaries and examples.** A public item that only its own library calls should be `pub(crate)`, and then rustc's lint covers it; one its integration tests call is the library's tested API (`proven-by-tests`).
- **Exceptions are `--allow` arguments in the detector's `command`**, so they live in `clm.toml` as the backlog item asked, show in the diff, and need no new settings key (`config` is untouched). An entry that matches nothing is a finding, so the list can't outlive what it excuses. Entry points called from outside Rust (`no_mangle`, `export_name`, `wasm_bindgen`) are skipped without an entry.
- **Rust first.** This repository is Rust; Python and TypeScript come later as another grammar and query file in the same crate (`tree-sitter-analysis`), recorded as a backlog item.
- **No `cargo`.** The tracked `Cargo.toml` files are the workspace, so the result needs no build and is the same on every machine.

Rejected:
- **Inside `clm-check`, run by `clm check` itself.** It would bring tree-sitter into a crate that must build for wasm, grow an oversized node, and make every project pay for a Rust-only rule; as a detector, a project opts in.
- **rustc's `unreachable_pub` lint.** It flags `pub` items not reachable from the crate root, not items no other crate uses, and it doesn't look across a workspace.
- **An allow-list key in `clm.toml`** (`[unreferenced] allow`). It would be a `config` contract change for a list only this detector reads.
- **Adopting it in this repository in the same change.** The first run will list many items across every crate, whose clean-up is other nodes' code; that is a backlog item.
