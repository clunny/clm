---
title: "Take-back skipped on a rename branch pending a spec change"
kind: workaround
components: [sync]
requirement: sync/reapply@5dc2a995
---

The take-back on a branch that regenerates nothing would, read literally, undo the deltas a rename branch's Rename commit rebased; the build skips it on the rename path and files backlog/reapply-s-take-back-reads-as-undoing-a-rename-branch-s-rebased-deltas.md for the spec to say so.
