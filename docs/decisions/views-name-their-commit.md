---
title: "The views name the commit they describe; the queue is clm next's"
date: 2026-10-26
status: accepted
---

`clm status` and `clm view` describe the checkout: its working tree and the remote-tracking refs as of the last fetch, without fetching, so they stay offline and fast. `clm next`, `clm claim` and `clm run` fetch and read `origin/main`. When a session's checkout is an old commit of `main`, the two disagree, and nothing said so: a session that read `clm status` (as the session-start hook prints it) told the human the queue was empty while `clm next` listed a ready node.

So every view names the commit it describes, and says when that commit is behind `origin/main`: `clm status` begins its summary with `commit <hash>`, and the charts begin with the same line, followed by `⚠ <n> behind origin/main` when the checkout lacks `n` of `origin/main`'s commits as of the last fetch. The CLM pane, which fetches every 2 minutes and redraws every 30 seconds, then shows the warning as soon as a fetch moves `origin/main`, since it only displays what `clm view` prints (`views-rendered-by-clm`). And the `implement` skill takes the queue's state from `clm next`, never from `clm status`, `clm view` or the session-start state, and never refreshes them by pulling or syncing its own checkout, which may hold the human's work in progress.

Rejected:
- **The views read `origin/main`'s tree**, as `clm next` does, through `extract_tree`. They would always agree with `clm next`, but a session on a `spec/*` or `impl/*` branch could no longer see its own work in `clm status` or `clm view spec`, and every redraw of the pane would extract a tree.
- **The views fetch.** They would stop being offline and fast, and the pane and the session-start hook would wait on the network; the pane already fetches on its own cadence.
- **The session-start hook pulls or fetches.** Pulling moves the human's checkout under them; fetching makes every session's start wait on the network, and a warning is enough once the views say what they describe.

The warning compares with `origin/main` as of the last fetch, so a checkout that hasn't fetched in a while can still be behind without a warning; that is why the queue's authority stays with `clm next`, which fetches.
