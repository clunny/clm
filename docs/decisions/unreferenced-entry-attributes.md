---
title: "clm unreferenced knows the language's entry-point attributes and takes the rest as --entry arguments"
date: 2026-10-27
status: accepted
---

An item called from outside Rust carries an attribute that says so, and no Rust file names it, so `check.unreferenced` must skip it. It knew `no_mangle`, `export_name` and `wasm_bindgen`; it missed proc-macro entry points (`#[proc_macro]`, `#[proc_macro_derive]`, `#[proc_macro_attribute]`) and framework attributes on `pub` functions, such as pyo3's `#[pyfunction]`, napi-rs's `#[napi]` or workers-rs's `#[event(fetch)]`. (The review that found this named `crates/clm-web/src/worker.rs`, but its `#[event]` functions are private, so the detector never lists them and this repository needs no `--entry`.)

- **Built in: the attributes Rust itself defines**, plus `wasm_bindgen`, which the first build already skipped. The proc-macro attributes are part of the language, so every project wants them skipped.
- **Framework attributes are `--entry <attribute>` arguments in the detector's `command`**, beside `--allow` (`public-item-detector`): the frameworks are open-ended, and a short built-in list of common names (`event`, `napi`) would skip any unrelated attribute that happens to share one. The name is compared with the attribute's last path segment, so `#[worker::event(fetch)]` and `#[event(fetch)]` both match `--entry event`.
- **An `--entry` that marks no item is a finding**, as a stale `--allow` is, so the list can't outlive the code it excuses (`no-dead-code`). It marks an item when an exactly-`pub` item the detector would otherwise list carries it, whether or not something uses that item: the entry describes how the code is called, not whether it is dead. Entries apply before the allow-list, so an `--allow` that only repeats an entry is stale.

Rejected:
- **A settings key in `clm.toml`** (`[unreferenced] entry`): a `config` contract change for a list only this detector reads, as `public-item-detector` found for the allow-list.
- **Growing the built-in list with framework names.** Each name is a guess about which crates a project uses, and a wrong guess hides dead code silently.
- **Spelling them as `--allow` entries per item.** One attribute marks many items, and an item added later would need its own entry, so the entry would lag the code.
