---
title: "Only cargo projects get a shared land build directory and its lock; other toolchains rebuild from nothing or share by accident"
kind: idea
components: [land, init, config]
---

`land` keeps one build directory for every land in a clone, `.clm/land`, so lands build incrementally instead of from nothing (`shared-land-target`), and guards it with a lock (`land-target-lock`). Both are tied to cargo: the land sets `CARGO_TARGET_DIR`, and the lock is taken only when the check will run with it. A project in another compiled language gets neither: each land builds wherever its toolchain puts output, which is either inside the land tree (rebuilt from nothing each land, if the tree is cleaned) or a cache the toolchain shares by default with no CLM lock around it.

## Direction: toolchain-neutral land places

Make the shared build place something a project declares rather than something CLM infers from cargo:
- a setting naming the environment variables or arguments that point the toolchain's output at `.clm/land` (cargo's `CARGO_TARGET_DIR` being one case), which `clm init` fills for the toolchains it recognises and leaves unset otherwise;
- the land lock taken whenever that setting points output at the shared place, not only for cargo.

## Toolchains to consider

- **Rust:** `CARGO_TARGET_DIR`, as today. Large output, so sharing matters most.
- **Go:** the build cache (`GOCACHE`) is already shared per user and safe to use at once, so it needs no CLM place or lock. CLM should do nothing here, and the design should allow "nothing".
- **C and C++ (CMake, Bazel, Meson):** an out-of-tree build directory, plus a compiler cache such as ccache or sccache. Bazel has its own output base and locking, so CLM should not wrap it.
- **Java and Kotlin (Gradle, Maven):** Gradle's build cache and daemon are shared per user; the project's `build/` directory sits in the tree.
- **JavaScript and TypeScript:** the dependency install (`node_modules`) is the large part, not a build. Sharing or linking it (pnpm's store) matters more than a build directory.
- **Python:** usually no build; a virtual environment per land tree, or one shared.

The principle: CLM gives a project a shared, locked place when its toolchain benefits from one, and stays out of the way when the toolchain already shares safely. It never assumes cargo, and it never makes a project with no build step pay for one.

Related: `a-land-check-that-builds-nothing-still-waits-for-one-that-does`, `builds-run-out-of-disk-mid-check`.
