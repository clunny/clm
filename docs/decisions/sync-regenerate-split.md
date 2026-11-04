---
title: "`sync`'s regenerate is carved out as `sync.regenerate`, crate `clm-regenerate`, so `clm land` can share it without reaching `clm-health`"
date: 2026-10-31
status: accepted
---

`land-approval-split` had `clm land`'s derived-specs check call `clm sync`'s regenerate (`regenerated`) instead of keeping a copy. Building that added `clm-sync` to `clm-land`'s dependencies, and `clm-sync` depends on `clm-health` for its reconsider step, so `clm/health-isolated` failed: `clm-land -> clm-sync -> clm-health` (`backlog/land-derived-specs-reaches-clm-health-through-sync`). In the triage of 2026-10-31 the human chose to carve the regenerate out into a child node with its own crate, keeping `health-isolated` as it is, over taking `land` out of the crates that rule names.

So `sync` gains the child `sync.regenerate`, crate `clm-regenerate`, below `clm-sync` and `clm-land` both:

- **What moves.** `regenerated` and its `Tree` and `Regenerated`, the own-violations filter and the extraction it uses; `stages.rs`, `stale.rs` and `merge.rs`, the stages, the stale-delta report and both merges, the other-deltas merge included; and `lock.rs`, the `index.lock` retry, which the merges' commits need and which `clm-sync`'s own `git` commands then call from `clm-regenerate`. The requirements that state them move with their text unchanged (`nested-splits`): `regenerated-tree`, `stale-delta-diff`, `stale-delta-merge`, `other-delta-merge` and `retry-index-lock`, with the Design paragraphs **Stages**, **Merge**, **Other deltas** and **A transient `index.lock`**. A new `git-through-layer` scans the crate and checks that `clm-sync` keeps no copy.
- **What stays.** Everything else `clm sync` does, its reconsider step and `clm-health` with it, and `scratch.rs`, the temporary worktree `HEAD` is judged in (`judged-by-check`), which the regenerate never uses. Mirroring the tree into `specs/` and the `Re-apply` commit stay `clm sync`'s; `regenerated` never writes `specs/` in the repository. `extract-through-layer` stays `sync`'s, whose scan reads `clm-sync`.
- **No re-export.** `clm-land` depends on `clm-regenerate` itself, and `clm-sync` calls it directly, so a `pub use` in `clm-sync` would have no caller (`no-dead-code`).

The cut was checked with `clm survey --nodes` before it was written (`splits-import-one-way`): `merge.rs`, `stale.rs`, `stages.rs` and `lock.rs` as the child, the rest of `clm-sync` as the parent depending on it, exits 0. Their uses of `lib.rs`'s helpers (`git`, `run`, `io`, `exists_at`, `own_violations`) go with them, `git` and `run` being `lock.rs`'s already.

**How the builds divide the move.** `sync` and `land` depend on the child, so the child builds first (`nested-splits`): it creates `crates/clm-regenerate`, moves the code, deletes `clm-sync`'s copies, rewires `clm-sync` onto it, adds it to the workspace's `Cargo.toml`, and re-cites the tests that cite the moved requirements with its own IDs, moving them out of `crates/clm-sync/tests/sync.rs` into a file of their own (`crates/clm-sync/tests/regenerate.rs`), with the fixture helpers both files use in `crates/clm-sync/tests/common/mod.rs`, which holds no markers and stays `sync`'s. A file counts toward every node it cites (`tests-count-where-cited`), so leaving them in `sync.rs` would count that whole 119 KB file toward the child too. `sync` has nothing left to build once the child has rewired it. `land`'s build then switches derived-specs to `clm_regenerate::regenerated` and cites `derived-specs` at its new version.

Rejected:
- **Take `land` out of `clm/health-isolated`.** It would let a land read what `clm health` reports, which the rule exists to prevent (`health-advises`).
- **Make `clm-health` an optional feature of `clm-sync`.** Cargo unifies features across the workspace, so `clm-land` would still compile against it whenever `clm` does, and the rule's test reads the dependency graph, not features.
- **Keep the requirements in `sync` and move only the code.** A fix to them would then be built on `impl/sync` in another node's `paths`.
