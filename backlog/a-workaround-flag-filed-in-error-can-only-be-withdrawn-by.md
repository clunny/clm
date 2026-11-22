---
title: "A workaround flag filed in error can only be withdrawn by deleting its file"
kind: bug
components: [check.health]
---

`clm workaround <requirement>` files a flag as a backlog item, and nothing withdraws one filed in error: the command only files, and the decision `workaround-flags` names no way to take a flag back. A session that flagged a workaround by mistake had to delete the item's file by hand on a branch. Expected: a stated way to withdraw a flag that was never true, apart from settling one that was (`workaround-flags`, `redesign-before-workaround`).
