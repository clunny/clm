---
description: Read the repository's health with clm health, summarise it, and have the spec-slimmer propose cuts for the components it marks as bloated. Use when the user asks how healthy the project or its specs are, or what to reconsider or slim down. Changes nothing.
---

# Project health

Read the repository's health and say what it means, so the human can decide what to take to `/clm:propose` (`health-advises`). This skill only reads: edit no spec, file no backlog item and land nothing from it.

1. Run `clm health`. It prints one line per finding, `<path>: health/<signal>: <message>`, grouped by signal with a summary line for each, and exits 0 whatever it finds; when it exits 1 or 2, tell the user what it printed and stop.
2. Summarise each section for the human in a few lines, saying what its signal means:
   - `check`, `pending`, `oversized`, `lapsed-claim`, `stalled-change` and `unanswered`: the state, what `clm check`, `clm pending`, `clm size` and `clm status` each show part of, and the claims, changes and questions nothing is moving;
   - `uncited-decision`: an accepted decision nothing names, which may be obsolete;
   - `unserved`: an entry of `ARCHITECTURE.md` or `CAPABILITIES.md` no rule serves, which its own tests may still prove, or which no component builds yet;
   - `history-words`, `design-heavy` and `outlier-size`: spec text that narrates history rather than describing the system, a Design longer than its Contract and requirements together, and a component far off the median in size per requirement;
   - `unexercised`: code no test runs, from the lcov file the settings name;
   - `workaround`, `rewritten`, `many-scenarios` and `many-exceptions`: the signals that a requirement is fighting its implementation, from builds' flags and the archive;
   - `reconsider`: the reconsider candidates. Name each with its score and its breakdown, and whether its `reconsider-<component>-<slug>` question is filed. A score counts a requirement's flags, the changes that rewrote it and its excess scenarios and exception clauses, and starts only once a build has flagged it.
3. For each component a `design-heavy`, `history-words`, `outlier-size` or `unexercised` line names, launch the `spec-slimmer` subagent with that component's ID and `model: "sonnet"` (`model-per-launch`), once per component however many lines name it, in parallel, and wait for each to return. A line names a file: `specs/<path>/SPEC.md` is the component whose ID is `<path>` with `/` read as `.`, and a code file belongs to the component whose `paths` hold it (`clm context <component>` shows a component's `paths`). List the cuts each proposes, with its quotes, under the component.
4. End by pointing at `/clm:propose` for acting on any of it, such as answering a reconsider question by dropping, changing or redesigning around the requirement, or making the cuts, and at `clm guide using-clm` for what the signals mean.

Never push `main` by any means but `clm land`. Run `clm approve` only after the human says yes in this conversation, and only through the guard's confirmation; this skill never runs it.
