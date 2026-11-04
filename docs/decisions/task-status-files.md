---
title: "Each task's status lives in its own file, written only by that task's PR"
date: 2026-10-22
status: superseded
superseded_by: node-reconciliation
---

Refines `generated-plans`. A task's `status` and `blocked` are the only plan fields a task PR writes. Kept in the shared `plan.md`, they put every parallel task PR of a change on adjacent lines of one file, so each merge makes the others conflict. So each task gets `changes/<id>/tasks/<node>.md`: frontmatter `status` and optional `blocked`, and a free-notes body. `plan.md` holds only what `clm plan` derives, plus the notes on approach.

- **One writer per file.** Only a task's own PR writes its status file: it adds it as `done` when the task is built, or with `blocked` when the task pauses (`pause-affected-tasks`, `blocked-by-backlog`). A task with no file is `todo`, so `clm plan` never creates or edits one. It only deletes the files of tasks it removes. Files with different writers never conflict, on GitHub or locally. The one accepted exception: a re-plan that removes a task whose PR is still open conflicts with that PR, but that work is moot anyway.
- **Absent means `todo`.** No file to create keeps `clm plan` out of status files entirely. It also lets the model switch to status files before `clm plan` and the other readers do, without breaking `main` in between.
- **Instead of a merge driver.** `20261022-sync` designed a `plan.md` merge driver for `clm sync`. GitHub's merge doesn't run custom drivers, so a conflicting PR would still need a session to rebase and push it. Separate files remove the conflict instead of automating its repair, and need no driver code or attributes setup.
- **Blocked stays put.** `clm plan` no longer clears a `blocked` that stopped blocking. The queue already ignores a `blocked` whose change was approved or whose backlog item was deleted, so the stale value is harmless, and leaving it keeps `clm plan` out of status files.
- **Archive untouched.** Archived plans stay in the old format, and the model doesn't parse them, since nothing reads an archived task's status.

Rejected:
- Deriving status from merged PRs or branches. That's state outside committed files, against `git-is-truth`.
- One file per task holding the whole task, `requirements` and `context` included. Then a re-plan would rewrite files that task PRs also write, which brings the conflict back.
