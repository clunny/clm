---
status: applied
resolves: [clm-next-prints-nothing-when-the-clone-s-build-places-are-full]
---

## Intent

Let a session tell a full build cap from an empty queue. When the clone's build places are all held and work is ready, `clm next` still prints nothing to stdout and exits 0, but writes one line to stderr, `no free build place: <held> of max_builds = <cap> in use, <n> ready`, in the words `clm run` already uses when it exits 3. `/clm:build` reads that line as no free place, as it reads `clm run` exiting 3, and tells the human the cap is reached instead of draining to the wrap-up as if nothing were left (`backlog/clm-next-prints-nothing-when-the-clone-s-build-places-are-full`).

## Problem

`orchestrator/next` prints at most `max_builds` minus the places that count, and nothing when none is free (`build-places`). Its output is then the same as an empty queue's: nothing, exit 0. `plugin.build/build-finishes-builds` tells the skill to report a full cap only when `clm run` exits 3, and the skill runs `clm run` only for what `clm next` printed, so when other sessions' builds or drafts held every place before the session's first build, the report can never appear: the skill went on to the wrap-up, and finding the cause took digging through `git worktree list` and `clm config list`. No requirement states that a caller can tell the two cases apart, so the fix is a spec change (`code-fix-needs-a-written-rule`).

## Approaches

1. **A stderr line, exit 0** (chosen). Stdout and the exit code stay as they are, so every caller that reads the list keeps working, and the `no place free` scenario still holds for stdout. The line appears only when the cap alone holds ready work back, so an empty queue stays silent. It reuses `clm run`'s wording (`reuse-first`), so the skill and the human see one message for one cause.
2. **An exit code of its own** (such as 3, as `clm run`). Rejected: an exit other than 0 reads as a failure to every caller that checks it (`clm next --wait`'s own first look, scripts, the skill's "exits 1 or 2" handling), for a state that is not an error, and it would break the `no place free` scenario's exit 0.
3. **Print the line whenever the cap cuts the list** (also when it prints some components). Rejected (`yagni`): a session launching what was printed is not misled, and it runs `clm next` again after each build returns.
4. **Have `clm next --wait` say so too.** Rejected: a watch waits for a place to free as it waits for work (`next-wait`'s `a place frees`), the watch is offered only after step 5's summary, which now names the full cap, and a line every interval would only repeat it. The `next` requirement states that a waiting look writes no such line, so `next-wait` itself is unchanged.

## Outcome

- `orchestrator` Contract: `clm next` says on stderr when it prints nothing only because no place is free.
- `orchestrator/next` (modified): the stderr line, its format, when it appears and when it doesn't (empty queue; a waiting `--wait` look), with scenarios `no place free` (modified), `full cap, empty queue` and `waiting on a full cap` (added). The code: `clm_orchestrator::next` returns the held and ready counts beside the list (or a type carrying them), and `crates/clm`'s `next` command writes the line; `next_wait` doesn't. `crates/clm/tests/run.rs` calls `clm_orchestrator::next` and moves with it.
- `plugin.build/build-finishes-builds` (modified): the skill treats a `clm next` that prints nothing with a `no free build place` line as no free place, never as an empty queue, and with none of its own builds running tells the human the cap is reached and by how many places, naming `max_builds` and `clm.local.toml`. Needs `orchestrator/next`, so the skill names the line only once `clm next` writes it.

## Constraints

- `build-places`, `reuse-first`, `yagni`, `code-fix-needs-a-written-rule`.
- No edge cases or workarounds known.
