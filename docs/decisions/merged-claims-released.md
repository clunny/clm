---
title: "A claim whose tip is in origin/main is released"
date: 2026-10-23
status: accepted
---

Refines `lease-claims`. A claim used to be live while its branch had a commit within the expiry, and that included the hours after its PR merged. Until `clm checkpoint` deleted the branch, a merged claim hid the item's re-plan, and because a claimed item counts as progress, it also held back other changes through `prefer_after` and priority.

Now a claim whose tip is an ancestor of `origin/main` is *merged*, and it is released at once, the same way a lapsed claim is. `clm claim` takes it over with a lease on that tip. Ancestry is a sound test for claims because:
- every claim is a new commit on top of `origin/main`, so a live claim is never in `origin/main`;
- both merge modes land a PR as a merge commit (`push` merges with `--no-ff`, `auto-merge` with `gh pr merge --merge`), so a merged head always is.

Git answers this offline on the refs the fetch just updated, so `next` doesn't need to call `gh` (`git-is-truth`, `reuse-first`). `clm checkpoint` uses a stricter test, because it also deletes `spec/*` branches, and a spec branch just cut from `main` passes the ancestry test before it has merged: the tip must also be off `origin/main`'s first-parent chain (`checkpoint-before-done`). If a repository ever squash-merged, merged claims would stay live until they lapsed, as before, and nothing would be lost.

Leftovers follow the same test. `clm run` removes a clean worktree or local branch of the item whose tip is in `origin/main` before claiming. Anything with uncommitted, untracked or unmerged work still stops it. In `push` mode, CI deletes a merged `impl/*` or `spec/*` head branch, leased on the head it merged, as a courtesy: the queue no longer depends on that deletion happening.
