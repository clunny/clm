---
title: "A build cut off by a usage or rate limit is resumed, not rebuilt"
date: 2026-10-31
status: superseded
superseded_by: resume-sound-builds
---

`builds-in-subagents` has `implement` treat a build subagent that ends without a closing line as a stop: run `clm run --finish <node>`, then build the node again with `clm run <node>` and a new subagent. That fits a build that gave up or ran out of context. It doesn't fit one that an API usage or rate limit cut off: there the build's claim, worktree and context are all intact, and nothing about the work went wrong. In one session every build ended at once on the limit; rebuilding them would have thrown six builds' context away and restarted each from its last push. The session instead resumed each subagent with `SendMessage` once the limit reset, and all six landed. One had to be told that its own review subagent, killed by the same limit, would never report back.

So a build whose subagent ends on the API's limit error hasn't stopped. `implement` runs no `clm run --finish` for it, so it keeps its place under `max_builds` and its worktree. Once the limit has reset it resumes that same subagent with `SendMessage`, saying the limit cut it off, that it carries on where it was, and that any review or check subagent of its own that the limit killed must be run again. The resumed build returns like any other and is finished then. When it can't be resumed, because `SendMessage` fails or the subagent went with its session, it counts as a stop and the existing retry rule applies.

Rejected:
- **Keep the retry rule** (`clm run --finish`, then a new build). It works, but it costs each build's context and repeats its work since the last push, for an interruption that wasn't the build's.
- **Run `clm run --finish` and then resume with `SendMessage`.** The finish frees the build's place, so the resumed build would run outside the clone's cap and another launch could take its place.
- **A `clm` command that records a build as paused.** Nothing in `clm` needs to know: the place is already held until `--finish`, and only the session that launched the subagent can resume it.

Accepted edge cases: the session that runs `implement` is usually cut off by the same limit, so the resume happens on its next turn after the reset, which may need the human to say "continue". A limit that outlasts the claim's four-hour lapse lets another session take the node in the meantime; the resumed build then meets that work when it syncs or pushes, and when it can't go on it stops, which the retry rule handles. Neither needs a workaround.
