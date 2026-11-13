---
status: proposed
resolves: [loop-wrap-up-fails-deleting-a-propose-branch-already-gone-from]
---

## Intent

`clm wrap-up` deletes the branches on `origin` that are done, each leased on the tip its fetch read. When another session deletes one of them between that fetch and the push, git refuses the lease and the wrap-up exits 2, though nothing was left to delete. A branch already gone from `origin` should count as done.
