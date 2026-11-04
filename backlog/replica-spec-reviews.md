---
title: "Spec-branch reviews from the replica, if they show up over the CPU budget"
kind: idea
components: [web.worker.review, web.worker.replica]
---

`20261102-web-repo-replica` left `web.worker.review` computing a spec branch's review per request, from live state cached for 60 seconds, because measured traffic didn't show it over the 10 ms budget. If `wrangler tail` shows `exceededCpu` on `/branches/.../review` or `/branches`, move them into the repo's `Replica` object: it would also hold each `spec/*` branch's changed spec files, applied on the branch's pushes, and compute the reviews at its batched recompute.
