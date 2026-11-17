---
title: "The build skill has no sentence for pausing a printed run"
kind: deferred
components: [plugin]
origin: 20261116-the-build-loop-shows
---

`plugin/skills/build/SKILL.md` tells the session never to decide a launch, an order, a retry or a resume itself, and names the command for each word of the human's it carries (take over, drain, stop, freed disk, give up a build), but none for "hold that run", so a human's hold has no command to carry it.

Once `orchestrator.loop/pause` is current, add to the paragraph that lists those words a sentence telling the session, when the human asks it to hold a run the loop printed and it hasn't launched, to run `clm build pause <name>` instead of carrying out the action, and `clm build pause --end <name>` when they say to go on; and, for a run already launched, to stop its task first and report it with `clm build returned <name> --result none --cause stopped`, then pause what the loop prints again. Resolve this item in that commit with the trailer `Resolves: backlog/the-build-skill-has-no-sentence-for-pausing-a-printed-run`.
