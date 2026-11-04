---
title: "Triage asks what to work through a menu, and brainstorms ideas with the human instead of drafting them"
date: 2026-10-30
status: accepted
---

`/clm:triage` drafted every group `clm triage` printed, bugs and ideas alike, and listed the rest. The human couldn't say "only the bugs today", the questions waiting on them sat at the bottom of the summary, and a branch whose design was finished and only waited for their approval wasn't offered at all. An idea fared worst: whether it is worth doing, and in what shape, is the human's call, and a draft that can't ask either guessed at it or stopped on it, so ideas came back as changes nobody had agreed to, or as `deferred`.

So triage opens with a menu. `clm triage` sorts its lines into forms the menu maps onto: `designed` (a proposed change whose design is finished, waiting to land), `open` (a proposed change still being designed), `main` (applied, waiting to be built), `group` (bug groups), `human` (questions) and `idea`. The skill offers *Approvals* (`designed`), *Needs your input* (`human`), *Bugs* (`group`), *Blockers* (any line whose items free requirements) and *Ideas* (`idea`), with counts, and works only what the human picks. Bugs are still drafted unattended in the background; questions are put to the human one at a time and drafted once answered (`answers-reach-drafts`); ideas are brainstormed in the human's own session, one at a time, through `propose`'s brainstorming (`design-brainstorming`), and end proposed, deferred with the condition they wait on, or dropped.

Rejected: a `clm triage --only <kind>` filter, which the skill doesn't need since it reads the line forms, and which no other caller asks for yet (`yagni`); drafting ideas and folding them for review, which spends a whole draft session and a review on a design the human never shaped.

`clm draft`'s rule that a draft defers an idea waiting on the human's judgement (`draft-defers-waiting-items`) goes, since no draft takes an idea any more.
