---
title: "clm unreferenced skips an item a clm:remove-by marker holds"
date: 2026-11-22
status: accepted
---

`expand-contract` lands a contract change in steps: the provider offers the new beside the old, marked `clm:remove-by <change-id>`; each user moves off the old in a requirement of its own; and the provider removes the old once every move is built (`rename-adds-then-removes`). Between the last user's move and the removal, the old path has no user by design. When it is a public item of a library that only another crate used, `clm unreferenced` (`public-item-detector`) reports it, `clm check` fails, and the last user's move can't land: it can't add an `--allow` entry (`clm.toml` is `config`'s file, and a build may only drop one, `a-build-drops-its-allowance`), and it can't remove the item from the provider's crate (`path-ownership`). The removal can't land first either, since the user still calls the item. `verify`'s move off `clm_land::run_check` (`20261122-the-check-runner-s`) met exactly this.

Now the detector doesn't list an item a `clm:remove-by` marker holds (`check.unreferenced/removal-marked`). The marker already says the item is going, and `check/remove-by-markers` already fails it once its change isn't live, so the skip is bounded by the plan to remove the item and needs no list of its own. It skips nothing a project hasn't already declared dead by its own hand.

Rejected:
- **The change adds an `--allow` entry for the old path, which the removing build drops.** The entry would be stale, and a finding, from the change's land until the last user moves, so it fails `clm check` at once; and a spec branch that edits `clm.toml` is `mixed`.
- **One build moves the user and removes the old path.** It breaks `path-ownership`, as `expand-contract` rejected.
- **The provider's own tests keep calling the old path.** A use from the crate's own `tests/` counts, so this works, and it kept `clm_approval`'s old calls alive this wave; but it needs a test that exists only to keep the detector quiet, a test the removing build must then delete under an approved list, and every expand-contract has to remember it. The marker states the same intent once.
- **Skip only while the named change is live, read by the detector.** `check.unreferenced` depends on `vcs` alone and runs in projects with no specs; `check/remove-by-markers` already judges liveness, so reading it twice adds a dependency for nothing.
