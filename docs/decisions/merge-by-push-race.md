---
title: "Merge jobs race to push instead of queueing in a concurrency group"
date: 2026-10-23
status: superseded
superseded_by: land-locally
---

Refines `merge-tested-tree`. CI's `merge` job ran in a `merge-main` concurrency group with `cancel-in-progress: false`, so merges would queue. But GitHub keeps at most one running and one pending job per group, and a newly queued job cancels the pending one. When three or more PRs passed their checks together, every waiting merge but the newest was cancelled. Nothing restarted them, so those PRs stayed open and green until someone re-ran their workflows. With several sessions landing PRs at once, this happened several times a day.

The group was never what kept `main` safe. The merge job pushes without force, so a push onto a `main` that moved since the fetch is refused, and the job starts again on the new `main` and re-checks the merge. So the job now has no group, merges race, and the push settles each race. The retry cap goes from three to ten, so a burst of up to ten PRs all land. Every tree on `main` is still one that was checked.

The cost is runner time: a merge that loses the race runs the checks again on the new `main`, so a burst of N PRs runs about N²/2 checks instead of N.

Rejected:
- **A scheduled job that re-runs cancelled merges.** Scheduled workflows run at most every five minutes and are often late, and it adds a second moving part to keep the first one working.
- **A lock step that waits for other merge jobs.** It rebuilds a queue in shell on top of GitHub's API, with its own failure modes, for what the push race already gives.
- **CLM's own merge queue** (`backlog/clm-merge-queue`). It fixes the runner cost too, but it's a much larger piece of work. This keeps merges from being lost until it lands.
