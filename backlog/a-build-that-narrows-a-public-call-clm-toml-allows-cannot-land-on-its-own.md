---
title: "A build that narrows a public call clm.toml allows cannot land on its own"
kind: bug
components: [check.unreferenced, config]
---

A library's public call that nothing outside the library uses is kept by an `--allow` entry in `clm.toml`, which is `config`'s file. When a later requirement makes that call private, the build of the library's component narrows it, and the entry then skips no finding: a stale entry is itself a finding, so `clm check` fails (`unreferenced-allow-then-narrow`). That build may not delete the entry, since `clm.toml` is not its file (`path-ownership`, `land/build-holds-own-files`). Nor can `config` delete the entry first, because while the call is still public the detector reports it with no entry to skip it. So neither order lands one component at a time: only a branch that changes both components' files can, which the claim's own-files rule refuses for a build. This matters once every code branch changes one component's files, as the routes agreed on 2026-11-21 make it.

The expand end was misreported on 2026-11-21 as the same problem: a build that added a public call another component would adopt was believed unable to land alone. It can: the detector counts a use in the crate's own tracked `tests/` files, so a providing build lands a new public call by proving it from its tests. The report came from a `clm check` run while the build's new test file was still untracked, and the detector reads only tracked files, so it did not see the use.

This repository's one entry was removed another way the same day, by proving the call from its crate's own tests and deleting the entry on one small code branch with no claim, so nothing here waits on this item today; the next narrowing of an allowed call will.

Two designs were drafted and not taken.

A mark in the item's own file, saying the call is kept and why, in place of the `clm.toml` entry. It would let the narrowing build delete its own allowance in its own files. Rejected: a build could excuse its own item, so the allowance would no longer be a decision anyone outside the build made.

The component's Contract as the allowance, with nothing reported while the component is changing (a requirement of it pending). The "offered" half, a call the Contract names counting as used, was reviewed and is not needed, since a call is proven from the crate's own tests. The "changing" half hides a component's items for as long as any requirement of it stays blocked, and needs a limit before it can be taken.
