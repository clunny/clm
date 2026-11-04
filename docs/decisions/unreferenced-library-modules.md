---
title: "clm unreferenced reads a library as the module tree of its src/lib.rs"
date: 2026-10-27
status: accepted
---

`check.unreferenced` first took a library's files to be every tracked `.rs` under `src/` other than `src/main.rs` and `src/bin/`. A module that `src/main.rs` declares (`mod cli;` → `src/cli.rs`) is binary code, but that rule read it as library code: its uses of the library didn't count, so a function only the binary calls was reported, and its own `pub` items were listed. Its library files are now the files rustc compiles into the library: `src/lib.rs` and, transitively, the files of the out-of-line modules a library file declares (`m.rs` or `m/mod.rs` in the declaring module's directory). Every other `.rs` file of the package is outside the library: the binary's modules, `src/bin/`, and a file no module declares.

- **It follows `mod` declarations, which the detector already reads** for `#[cfg(test)] mod m;`, so it needs no `cargo` and no resolution beyond the module-directory rule it has.
- **Errors fall on the side the detector already accepts.** A module declared with `#[path]`, or inside a block, isn't followed, so its file counts as outside the library: its mentions count as uses and its items aren't listed. That can miss dead code but never reports a used item (`public-item-detector`).
- **A file no module declares is outside the library.** rustc doesn't compile it, so its items are no one's API; listing them would report code that isn't built, which `cargo-machete` and review catch better.

Rejected:
- **Subtracting the modules `src/main.rs` declares** from the old rule. It needs the same module walk from a different root and still lists files rustc never compiles into the library.
- **Keeping the old rule and asking for `--allow` entries.** The false findings would land on every package with both a library and a binary with modules, which is the common shape of a CLI crate.
