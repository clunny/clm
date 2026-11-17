---
title: "The build loop drafts a component's bug fix while it builds that component"
kind: bug
components: [orchestrator.loop]
---

What happened: in one `clm build step`, the loop printed a `launch build:<component>` action and, beside it, a `launch draft:` action for a `kind: bug` item whose fix lies in that same component. The build run claimed `impl/<component>` first. The draft run's `clm claim <component> --fix backlog/<slug>` was then refused ("`<component>` is claimed: `origin/impl/<component>` has a live commit"), so it stopped with nothing landed. Reported with `--cause blocked`, the loop printed the same draft launch again at once, while the build still held the claim, so the second draft will likely be refused the same way.

Expected: the loop does not launch a draft whose fix needs a claim on a component that a running build of its own holds (or holds it back until that build lands), and does not relaunch a draft refused for that reason while the claim is still live.

Rules involved: `build-loop-in-clm` (the loop decides launches and order), `code-fix-resolves-bug` (the fix takes `clm claim <component> --fix`).
