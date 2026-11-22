---
title: "A build may take a detector's --allow entry it makes stale out of clm.toml"
date: 2026-11-22
status: accepted
---

A public item that nothing outside its library uses is kept by an `--allow` entry in `clm.toml`, `config`'s file (`unreferenced-allow-then-narrow`). When a requirement makes that item private, the build of the library's component narrows it, and the entry then skips no finding, which `check.unreferenced/allow-list` reports, so `clm check` fails. `unreferenced-allow-then-narrow` let that build delete the entry in the same diff, as an exception to `path-ownership` like the root's dispatch files. When the dispatch files became the root's `wiring`, the exception was not carried over: `gate/own-files` and `land/build-holds-own-files` count `clm.toml` as `config`'s, so the narrowing build can't take the entry out, and `config` can't take it out first, since the detector then reports the still-public item. Neither order lands one component at a time.

**`foreign_files` leaves out an edit of the root's `clm.toml` that only drops `--allow` pairs** (`gate/own-files`). The edit must delete whole lines and add none, and the file, read as TOML, must equal the merge base's with one or more `"--allow"` elements and the element after each taken out of `[[detectors]]` commands. Such an edit can excuse nothing: a dropped allowance only makes a detector stricter, and `clm land` runs `clm check` on the merged tree, which fails if the entry was still needed. So it needs no one outside the build to decide it, which is what kept the other designs out. The rule is the gate's, so every code branch of any name but a light one (`gate/light` lists its own files), and `clm land`'s claim test, which reads `foreign_files`, agree on it.

Rejected:
- **A mark in the item's own file in place of the `clm.toml` entry.** The narrowing build could delete its own allowance in its own files, but a build could also excuse its own item, so the allowance would no longer be a decision made outside the build.
- **The component's Contract as the allowance, with nothing reported while the component is changing.** Its "changing" half hides a component's items for as long as any of its requirements stays blocked, with no limit; its "offered" half isn't needed, since a call is proven from its crate's own tests.
- **`clm.toml` in the root's `wiring`.** Every build could then change any key of the file, `[land] check` among them, and add allowances for its own items.
- **A stale entry reported but not failing, or not failing while its item exists unexported.** Nothing would then make anyone take it out, and the list would stop shrinking (`no-dead-code`).
- **Lines read by the entry's crate.** Holding the build to entries for its own crates would map crate names to paths in the gate; a build dropping another crate's entry can only fail its own check, so the narrower rule buys nothing.
