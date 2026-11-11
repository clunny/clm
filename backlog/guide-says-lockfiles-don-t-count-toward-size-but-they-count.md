---
title: "Guide says lockfiles don't count toward size, but they count unless marked linguist-generated"
kind: bug
components: [init, plugin.init]
---

`docs/guide/shaping-the-graph.md` says "Lockfiles, binaries and anything `.gitattributes` marks `linguist-generated` or `linguist-vendored` don't count". `check/SPEC.md`'s **Size** excludes only binaries and marked files, as designed (`reuse-first`): a lockfile counts in full unless the project marks it. In a Python project a `uv.lock` made one component read as about twice its code budget, a false oversized state that could lead a session to split a component that needs no split. The session found the convention only by working around it.

Expected: the guide says lockfiles count unless marked, and `clm init` (or the init skill) proposes a `linguist-generated` line in `.gitattributes` for each lockfile the repository tracks, as this repository marks `Cargo.lock` and its `package-lock.json` files.
