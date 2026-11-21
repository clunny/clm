---
title: "The build loop has no way to hand its runs the human's notes for a build"
kind: deferred
components: [orchestrator.loop]
---

Step: `/clm:build`, carrying out the `launch` actions `clm build` prints.

What happened: the human opened the session with a file of notes that every build of the wave needs beyond its spec (code to mend, citations that move, tests to keep), and asked for `/clm:build`. The `build` skill says to give each subagent the launch file's text unchanged, and neither `clm run`'s prompt nor the loop's launch file has any place to name such a file. To follow the human, the session had to add a line to each launch prompt, departing from the skill's instruction.

Expected: a way to attach notes for the runs the loop launches (for example, a file `clm build start` takes, or one `clm run` reads), so that every build, including a relaunch or a later session's adoption of the loop, gets the notes without the session editing what the loop prints.

Rule involved: `build-loop-in-clm` (the loop decides every launch and its prompt), and the `build` skill's instruction to launch with the prompt file's text unchanged.

**Deferred** (2026-11-21, the human): taken up once wave 2 of phase 5's build has drained (`backlog/a-passing-check-proves-less-than-the-spec-claims.md`). Until then, `clm triage` lists it as `deferred` and the build loop drafts nothing for it.
