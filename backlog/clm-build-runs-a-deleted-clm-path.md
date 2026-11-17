---
title: "clm build runs `clm` from a path that ends in ` (deleted)` once the dev install replaces it"
kind: bug
components: [orchestrator.loop]
---

Found by review while building `land/land-binary-copy@a11966b0` on `impl/land`. `clm build` hands `clm_loop::Options.clm` the path `std::env::current_exe()` gives (`crates/clm/src/main.rs`), and `crates/clm-loop/src/cmd.rs` spawns every `clm` subcommand from it. On Linux, once the dev install renames a new `clm` over the running one because `main` moved, that path ends in ` (deleted)`. Every subcommand the loop spawns after that fails with `No such file or directory`, for as long as the `clm build` process runs.

Expected: the loop runs the `clm` now at the path without the suffix, as `clm land` copies it (`land/land-binary-copy`). No requirement of `orchestrator.loop` states this. `crates/clm-land/src/exe.rs` holds the resolution privately; a shared helper would need a component both may depend on. Rule: `dogfood-from-main`.
