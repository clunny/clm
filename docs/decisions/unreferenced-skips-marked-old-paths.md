---
title: "clm unreferenced skips an item a clm:remove-by marker holds"
date: 2026-11-22
status: accepted
---

`expand-contract` lands a contract change in steps: the provider offers the new beside the old, marked `clm:remove-by <change-id>`; each user moves off the old in a requirement of its own; and the provider removes the old once every move is built (`rename-adds-then-removes`). Between the last user's move and the removal, the old path has no user by design. When it is a public item of a library that only another crate used, `clm unreferenced` (`public-item-detector`) reports it, `clm check` fails, and the last user's move can't land: it can't add an `--allow` entry (`clm.toml` is `config`'s file, and a build may only drop one, `a-build-drops-its-allowance`), and it can't remove the item from the provider's crate (`path-ownership`). The removal can't land first either, since the user still calls the item. `verify`'s move off `clm_land::run_check` (`20261122-the-check-runner-s`) met exactly this.

Now the detector doesn't list an item a `clm:remove-by` marker holds (`check.unreferenced/removal-marked`). The marker already says the item is going, and `check/remove-by-markers` already fails it once its change isn't live, so the skip is bounded by the plan to remove the item and needs no list of its own. 

**Why this mark is not the one `a-build-drops-its-allowance` rejected.** That decision rejected a mark in the item's own file in place of the `clm.toml` entry, because a build could excuse its own item and the allowance would stop being a decision made outside the build. A build can write a `clm:remove-by` marker too, above an unused item it owns, naming any live change. The difference is that the marker's excuse is not the build's to keep: it lasts only while a change the build doesn't control stays live, and `check/remove-by-markers` fails `clm check` once that change is archived, done or abandoned, so the item must then go or gain a user. A rejected mark there would have excused the item for good. A false marker buys at most the life of one change, and the spec auditor and code review read every marker a diff adds against the change it names.

Rejected:
- **The change adds an `--allow` entry for the old path, which the removing build drops.** The entry would be stale, and a finding, from the change's land until the last user moves, so it fails `clm check` at once; and a spec branch that edits `clm.toml` is `mixed`.
- **One build moves the user and removes the old path.** It breaks `path-ownership`, as `expand-contract` rejected.
- **The provider's own tests keep calling the old path.** A use from the crate's own `tests/` counts, so this works, and it kept `clm_approval`'s old calls alive this wave; but it needs a test that exists only to keep the detector quiet, a test the removing build must then delete under an approved list, and every expand-contract has to remember it. The marker states the same intent once.
- **Skip only while the named change is live, read by the detector.** `check.unreferenced` depends on `vcs` alone and runs in projects with no specs; `check/remove-by-markers` already judges liveness, so reading it twice adds a dependency for nothing.
