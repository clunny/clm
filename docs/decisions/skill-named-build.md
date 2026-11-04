---
title: "The skill that drains the queue is /clm:build, renamed in one step with no alias"
date: 2026-11-03
status: accepted
---

`/clm:implement` became `/clm:build`, since CLM already calls this work a build (`builds-in-subagents`, `max_builds`, build places, `unbuilt`). The cost of the word is that "build" can read as compiling, so the skill's description says it builds approved specs and does not compile code (`plugin/skill-named-build`).

**No alias.** `expand-contract` protects callers in other components. Nothing calls a slash command but a person, and the skills that mention it ship with it, so a stub and a contract change would guard a break no component has. Existing adopters' `CLAUDE.md` keeps the old name, since `clm init` never overwrites it, and they see an unknown skill with `/clm:build` listed beside it.

**Hand-written deltas, not `clm rename`.** The directory really moves, so the requirements naming its path need a build that moves their tests. A replayed term swap would re-cite those tests while the code still read the old path (`rename-by-replay`).

**What keeps its name.** `impl/*` branches, a label baked into the gate, claims and `clm land`; the `implementation` view and its route, which mean the state of the code against the spec. Archived changes and decisions keep the old name, as history.
