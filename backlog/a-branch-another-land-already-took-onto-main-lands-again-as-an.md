---
title: "A branch another land already took onto main lands again as an empty merge recorded spec"
kind: bug
components: [land]
---

`clm land` for `impl/ci` (head 492a886e) waited in the queue ("waiting for another land's check ... 1 ahead" / "2 ahead") and then printed `landed 998a911f... spec`. But `main` already held that head: `cbf3a863 Merge impl/ci into main` (parents e10ac865, 492a886e, 15:18:49, `Checked:` trailer, Gate-Rules 7) landed it about six minutes earlier, apparently while this land was waiting, and this `clm land` printed nothing about it. `998a911f` (parents f8a1a90d, 492a886e, 15:25:19) is a second merge of the same head with an empty diff against its first parent, and it was recorded as a `spec` land.

Expected: a land whose head `main` already contains, by the time it takes the lock or after the fetch, lands nothing, makes no empty merge commit and appends no second `landed` record. It either says the branch is already on `main` and names the commit that landed it, or (when a batch it waited in took it) prints that land's `landed <sha> <kind>` line, as `finish_found` does.

Seen in wave 3's `ci` build, on main after the `land` build that added batch lands and branch deletion.
