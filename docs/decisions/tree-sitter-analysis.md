---
title: "Multi-language analysis via tree-sitter"
date: 2026-10-21
status: accepted
---

Targets are Rust, Python and TypeScript first, with more later. Import and dependency extraction uses tree-sitter grammars from the Rust CLI, so adding a language means adding a grammar and a query file, not a new tool. Per-language tools (import-linter, dependency-cruiser) remain an option if tree-sitter turns out too shallow.

The tree-sitter runtime and grammars are C, which the `cc` crate compiles at build time and links into `clm`, so `clm` isn't pure Rust (2026-10-23). Each release build needs a C compiler for its target, which the runners already have, and v0.6.0 built all four targets with them. The binaries stay self-contained: no one who installs `clm` needs a compiler. Rejected: putting `clm survey` behind a cargo feature or in a separate binary to keep `clm` pure Rust, which costs more than the toolchain it saves; and running the tree-sitter CLI instead, which every user would have to install, and which compiles grammars on their machine.

(Amended 2026-11-04, `20261104-macos-cross-build`: the release now builds every target on Linux with Zig as the C compiler (`static-linux-binaries`, `cross-built-macos-binaries`); no target uses a runner's own compiler.)
