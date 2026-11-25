---
title: "A replica another release saved waits for its idle alarm before it fetches the files it forgot"
kind: bug
components: [web.worker.replica]
---

Found by the code review of `replica/markers-of-this-release`'s build. Since that build, `Replica::load` of a state another release saved (every store after a deploy) counts its markers again: it forgets each scanned file whose text it doesn't keep, so `pending_fetch` lists them, and it is `stale()`. `Object::load` (`crates/clm-web-replica/src/object.rs`) keeps whatever alarm the store already holds, and `view()` sets one only when none is set, so the re-fetch and recompute wait for an idle alarm that may be far off. Until something saves the replica, each wake of the object loads the old state again and runs the rescan again, while the snapshot stays at the saved views.

Expected: when the loaded replica is `stale()` or has files to fetch, `Object::load` brings the alarm forward to the next batch, as `batch()` does, so a new release's re-index starts at once.
