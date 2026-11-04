---
title: "Rebuild churn is read from the markers' git history, reported but never scored"
date: 2026-10-28
status: accepted
---

`clm health` counts a requirement's moves from the spec side: `rewritten` is the archived changes that modified it (`health-advises`). Nothing counted them from the code side, how many times a build re-cited the requirement's tests at a new version (`versioned-markers`), though every such citation is a line some commit added. A rewrite nobody has built yet has cost nothing; one built three times has cost three builds, and that is the churn a reader wants to see beside the flags.

So `clm health` has a signal `churn`: the distinct versions each requirement was ever cited at, read from the patches of every commit reachable from `HEAD` with one `git log -p`, each added line of a file `markers_in` reads handed to `markers_in`, so a marker counts where `clm coverage` counts it and Markdown never does (`reuse-first`). Nothing stores it: it is derived from git whenever the report runs (`git-is-truth`), at about ten seconds for this repository's 3,200 commits, a price a report command pays and no check does. The history isn't filtered to the files that cite the requirement today: the scan costs the same either way, and a path filter would lose a file's history before a rename. A requirement is listed at three or more versions, two re-citations, as `rewritten` lists two or more rewrites.

Churn is reported and never scored. It is the same moves `rewritten` counts, seen from the other side, so a score with both would count each built rewrite twice; and the score comes from `signals(graph, items)` with no git, because `clm sync` files the reconsider question from it on every `impl/*` branch and can't read the whole history each time. Should friction flags turn out too sparse for the score to reach the threshold, the condition the backlog item set for scoring churn, a spec change can weigh it: take the larger of rewrites and churn rather than their sum, and give `reconsider` the churn as an input `clm sync` computes once per run. Nothing here forecloses that.

Rejected:
- **Counting the lands that touched a requirement's files.** A test file holds many requirements' tests, so a land that touched the file says nothing about which requirement moved. The versions a marker cites are per requirement.
- **Filtering `git log` to the files that cite the requirement now.** No cheaper, and blind to a renamed file's past, so a move would reset the count.
- **A count that each land updates.** A second record of what git already holds (`git-is-truth`, `regenerate-dont-merge`), with a writer to coordinate.
- **Scoring churn beside rewrites.** Double counting, and git on `clm sync`'s path, as above.
