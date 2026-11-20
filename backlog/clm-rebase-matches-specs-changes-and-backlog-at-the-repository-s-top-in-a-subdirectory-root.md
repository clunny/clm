---
title: "clm rebase matches specs/, changes/ and backlog/ at the repository's top in a subdirectory root"
kind: bug
components: [sync]
origin: 20261119-clm-rebase-regenerates-a
---

Outside the rename path, `crates/clm-sync/src/lib.rs` still reads some paths from the git repository's top, not the CLM root's (`fingerprint-covers-the-clm-root`). In a CLM repository that is a subdirectory `tool/` of its git repository:

- the merge-driver attributes `sync` writes map `specs/** merge=clm-keep` (step 2, `sync-owns-merge-drivers`); a pattern holding a slash in `core.attributesFile` is matched from the repository's top, so `tool/specs/` never takes the driver and a `specs/` conflict stops the rebase instead of keeping `origin/main`'s side;
- step 1's refusals (`uncommitted`) read `git status --porcelain` paths, which are relative to the repository's top, and test them for the prefixes `specs/`, `changes/` and `backlog/`, so an uncommitted edit under `tool/specs/` on a spec branch is set aside by the autostash instead of refused.

Found by reading the code while designing `20261119-clm-rebase-regenerates-a`, which makes `regenerate-renames` and the added-change listing it shares with steps 2 and 4 read the root's trees; not run. Expected: every path `clm rebase` matches or prints is the CLM root's, with a path outside it named as `gate/paths-under-the-root` names one. No requirement of `sync` states this for steps 1 and 2 yet, so the fix needs a design.
