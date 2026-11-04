---
title: "This repository turns on clm unreferenced with an allow-list of every current finding, then narrows node by node"
date: 2026-10-27
status: accepted
---

`clm unreferenced`'s first run on this repository lists about a hundred `pub` items across ten crates, each its own node (`public-item-detector`). Clearing them first would hold the detector back behind ten builds, and some of the items can't be narrowed at all: a type another crate receives from a function it calls, without naming the type, is reported, but `pub(crate)` would make that function's signature a `private_interfaces` warning (`unreferenced-signature-types`).

- **Allow every current finding, by item, in the same build that adds the detector** (`config/unreferenced-detector`). From then on a new unused `pub` item in any crate fails `clm check`. An entry names one item, never a crate, so a crate with pending entries is still checked for new ones.
- **Every entry carries its reason in a `clm.toml` comment**: *kept* (live by a route the name match can't see, saying which) or *pending* (naming the node's `backlog/narrow-public-items-<node>` bug).
- **The list only shrinks.** A stale entry is a finding (`check.unreferenced/allow-list`), so the build that narrows an item must delete its entry. When the detector learns a false-positive class, the kept entries it covers go in a later branch, after `clm` is reinstalled from the `main` that holds the fix: `clm land` runs the tests with the branch's `clm` and `clm check` with the installed one, so no single branch satisfies both (the ordering `settings-before-use` gives a new key).
- **`config`'s own finding is kept, not pending.** `clm_config::Setting` is the type of every public field of `Settings`; other crates read those fields without naming it, so it can't be narrowed, and `config`'s build narrows anything else of its own.
- **One `kind: bug` item per node**, so each clean-up is a code-only fix on its own `impl/<node>` branch (`code-fix-resolves-bug`). Those builds edit `clm.toml`, `config`'s file, to delete their entries, so each item names `config` among its `nodes` too; it is an exception to a build never editing another node's files (`contract-removal-needs`), like the root's dispatch files and, since 2026-10-29, a removal's forced fixes (`architecture/path-ownership`), and the price of keeping the list in one place. Each node's entries are a group of their own, so parallel builds' deletions rarely conflict. The items are filed now, with the change, and work in either order: a node narrowed before the detector lands simply has no entries.
- **The detector runs the `clm` on `PATH`**, and `this_repository_is_clean` puts the binary under test first on it (`clm/clean-check-runs-own-clm`), since the release workflow's `prepare` job installs no `clm`. Elsewhere it is the clone's `clm` built from `main` (`dogfood-from-main`), which already has the command, so nothing waits on an install, unlike a new settings key (`settings-before-use`).

Rejected:
- **Narrow every crate first, then add the detector with no allow-list.** Nine other nodes' builds before anything is enforced, and the kept items need entries anyway.
- **`--allow <crate>` for each crate with findings.** Shorter, but it switches the detector off for most of the workspace until each clean-up lands.
- **Fix the detector's false positives first** (`20261027-unreferenced-false-positives`, `unreferenced-signature-types`). Those are `check.unreferenced` spec changes; once one lands and `clm` is reinstalled from that `main`, the kept entries it makes stale are removed in a later branch, so adoption needn't wait.
- **Install `clm` in `.github/scripts/detector-tools.sh`.** It would replace the developer's own `clm` on every check run (`no-shared-install`), and the build under test is the right binary for the test anyway; `ci/detector-tools` exempts a detector run by `clm` instead.
