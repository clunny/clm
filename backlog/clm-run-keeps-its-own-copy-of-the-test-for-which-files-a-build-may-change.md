---
title: "clm-run keeps its own copy of the test for which files a build may change"
kind: idea
components: [orchestrator.run, gate]
origin: 20261117-the-app-s-service-worker-is-app-sw-js
---

`orchestrator.run/pending-prompt` lists apart a stale marker in a file the build may not change. `clm-run` decides that with its own ownership test, read from the merged tree's graph, while `clm land` decides it with `clm-gate`'s `foreign_files`, read from the merge base's. The two can disagree, and did while `gate/own-files` still let through a file that cites the building component: the prompt told `web`'s build that the land would refuse a deletion the land accepted, and the build filed a question nobody needed.

Once `gate/own-files` is built the two agree in this repository. One predicate in `clm-gate` that both call would keep them from drifting again. Found by `/code-review` on `impl/orchestrator.run`.
