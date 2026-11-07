---
title: "`clm run` resumes a branch exactly when `clm branch push` would accept it"
date: 2026-11-07
status: accepted
---

A build rewrites its branch at every rebase point: `clm rebase` rebases `impl/<component>` onto `origin/main`, the claim commit included, and only then does the build push. A build that stopped between the two (its check killed for low memory) left a local branch that no longer contained `origin/impl/<component>`, though it held everything that tip held plus the build's new commits. `clm run`'s resume rule asked for containment, so the retry exited 1 with "reconcile or delete it", the build skill named no way to reconcile, and recovering took pushing the branch by hand from the stopped build's worktree and then unlocking and removing that worktree (`backlog/stopped-build-with-unpushed-commits-blocks-its-retry`).

`clm run` now resumes a local `impl/<component>` when it has held `origin/impl/<component>`'s tip: the tip is an ancestor of the branch's tip or of an entry of its reflog. That is git's `--force-if-includes` test, which `clm branch push` already makes before it overwrites the remote branch (`host/branch-push`), so `clm run` resumes exactly the branches the resumed build's first push will accept. Containment is the special case where nothing was rewritten. Everything else in the resume stays: the clean worktree holding the branch is unlocked and removed, the place is reused, and the prompt says the branch has unlanded commits to rebase and land.

Rejected:
- **Have `clm run --finish` push a stopped build's branch.** It would make the step that frees a place also write to `origin`, for a branch whose build may have stopped part-way through something, and `--finish` runs from the orchestrating session's checkout, not on the checked-out branch `clm branch push` works on. Resuming locally reaches the same state at the build's own next push, which is the build's to make.
- **Recognise a rebased claim by structure (an empty commit whose patch matches).** It covers only the claim commit: a build that had pushed work and then rebased it has rewritten that work too, and telling rewritten commits from foreign ones by patch is guesswork the reflog makes unnecessary.
- **Have the build skill reconcile by hand.** That puts git plumbing into an orchestrating session that may not run git in a build's worktree, for a case `clm run` can judge itself.

Accepted edge cases:
- A clone whose reflogs are off (`core.logAllRefUpdates` false, the default in a bare repository) holds no reflog, so a rewritten branch is refused as before. Its builds' own `clm branch push` after a rebase is refused by `--force-if-includes` for the same reason, so the retry fails no differently from the build; nothing in this repository's use runs builds from a bare clone (`yagni`).
- A reflog entry git has expired (by default after 90 days) no longer counts. A stopped build is retried within the session, long before that.
- A branch with no `origin/impl/<component>` and work on it is still refused: nothing was pushed for it to have held, and resuming it would push a branch no claim took.
