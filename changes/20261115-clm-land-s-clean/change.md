---
status: applied
resolves: [clm-land-s-clean-up-runs-git-from-a-directory-it-already-removed]
---

## Intent

This change deletes one backlog item, `clm-land-s-clean-up-runs-git-from-a-directory-it-already-removed`, as accepted behaviour. It changes no requirement and nothing is built.

The item reports that a `clm land` run in a build's worktree landed its branch and then warned twice: it could not delete the landed branch on `origin` (`git remote: No such file or directory`) and could not clear the build's `target/` (`git rev-parse --show-toplevel: No such file or directory`).

`clm land` removes no worktree. The build loop does: every step of `clm build` runs `clm wrap-up`, which removes a build's clean worktree as soon as `main` holds its branch (`orchestrator.checkpoint/checkpoint`). That moment is the land's push, so a wrap-up run by another session can remove the worktree while the `clm land` started in it is still appending its record and cleaning up. The land's last two steps then run git in a directory that is gone, and each warns without failing, as `land/landed-branch-cleanup` and `land/land-frees-build-target` say a failed delete does.

The behaviour is accepted because nothing is left undone:

- The land stands. `main` holds the merge commit, the `landed` record is appended before the clean-up, and the command prints `landed <sha> <kind>` and exits 0.
- The `target/` the land would have cleared went with the worktree that held it.
- The wrap-up that removed the worktree also deletes each landed `impl/*` branch on `origin`, under the same requirement. On the day the item was filed `origin` held no `impl/archive/*` branch, so the branch the item names didn't stay.

What remains is two warning lines on a land that succeeded. The plan in force (`backlog/a-passing-check-proves-less-than-the-spec-claims`) says a defect's default answer during the freeze is to accept it, and a requirement that keeps a land independent of its worktree after the push would cost a build and its tests to remove those two lines.

## Approaches

- **Taken: accept, and delete the item.** The two steps that warn are done by the wrap-up that caused the warnings.
- **Not taken: run the land's steps after the push from the main worktree.** It would remove the warnings, and needs a new requirement of `land`, since none states what a land does once its worktree is gone. It is outside the plan's freeze and fixes nothing that stays broken.
- **Not taken: have `clm wrap-up` leave a worktree a land is still running in.** Wrap-up would need to know of the running land, which no lock or record tells it after the push, and a kept worktree is what the wrap-up exists to remove.

## Edge cases

- Not covered, and not reported: a `clm land` that waits for the land lock, is landed by the lock's holder (`land/batched-lands`) and has its worktree removed before it wakes would fail its next fetch and exit 1 on a branch that landed. No session has reported it; it is filed when one does.
