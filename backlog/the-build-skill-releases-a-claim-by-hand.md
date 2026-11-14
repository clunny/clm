---
title: "The build skill releases a given-up build's claim by hand instead of with clm build release"
kind: deferred
components: [plugin]
origin: 20261114-releasing-a-dead-session
---

`plugin/skills/build/SKILL.md` says: "When the human gives up a component's build for good, release its claim by deleting `impl/<component>` on `origin` with `clm branch delete`." That leaves the local `impl/<component>` behind, which makes every step of the loop skip the component (`backlog/releasing-a-dead-session-s-claim-leaves-the-build-loop-skipping`, resolved by change `20261114-releasing-a-dead-session`).

Once `orchestrator.loop/release` is current, replace that sentence with one telling the session, when the human gives up a component's build, to run `clm build release <component>`, saying that it discards the build's work and that the loop builds the component afresh once it is ready, and to show the human what a refusal prints, which only the human can settle. Drop "for good": a released component is built again whenever it is ready. Resolve this item in that commit with the trailer `Resolves: backlog/the-build-skill-releases-a-claim-by-hand`.
