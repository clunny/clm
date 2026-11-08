---
title: "clm approve calls a lost ref race being offline"
kind: bug
components: [land.approval]
origin: 20261108-overlapping-lands-fails-under
---

`clm approve` starts with `Repo::fetch` (`crates/clm-approval/src/lib.rs`, called from `approve.rs`), which returns `offline: origin can't be reached; push and land when it can` on any failure of `git fetch origin`. A land in the same clone that pushes its ledger branch or deletes its landed branch while the approve's fetch runs moves a remote-tracking ref under it, and git refuses the fetch with `cannot lock ref ... is at <x> but expected <y>`, though `origin` is reachable.

Change `20261108-overlapping-lands-fails-under` fixed the same race for `clm land` and `clm reconnect` (`land/offline-land`: tried again after 1 and 2 seconds, and `offline` only when `git ls-remote origin` fails too). No requirement of `land.approval` states it for `clm approve`.

Expected: `clm approve` tries a failed fetch again the same way, and says it is offline only when `origin` can't be reached.
