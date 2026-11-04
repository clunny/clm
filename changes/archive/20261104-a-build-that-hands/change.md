---
status: done
resolves: [a-build-that-hands-back-a-waiting-line-can-end-without-a-closing]
---

# A build's return with no closing line is checked against origin before it counts as a stop

## Intent

When a build's subagent ends without a closing line, `/clm:build` counts it as a stop and builds the component again, even when the build's `clm land` ran and its branch is on `origin/main`. This change has `clm run --finish <component>`, which `build` already runs for every return and which already decides whether `impl/<component>` landed, print that decision as `landed impl/<component>`, and has the build skill read it for a build with no closing line: landed means it is treated as landed, told to the human and not rebuilt; otherwise it is a stop, as now. Decision: `unclosed-build-reads-finish`.

## Problem

A build handed back a result ending in `waiting: <command>`, as `orchestrator.run/background-work-stopped` tells it to while its land runs. Its land then merged its branch, and the subagent ended with no further closing line. `plugin/build-waiting-builds` tells the session to treat the next result as the build's return, and the skill counts a return without `landed <sha>` or `stopped:` as a stop. Read literally, the session would tell the human the build stopped and launch it again. The session found out from `origin` by hand that it had landed. The prompt already requires the closing line, so the gap is in how `build` reads a return that lacks one, not in the prompt.

## Approaches

1. **`clm run --finish` prints whether the branch landed, and `build` reads it (chosen).** The finish already computes "landed" for the worktree tidy, by the rule the places use, and runs for every return, so this costs no extra step and keeps one definition of "landed" in code (`reuse-first`).
2. **The skill reads `origin` itself.** It writes a git rule out in prose (branch gone and merged into main) that duplicates `run-finish`'s, and a deleted branch looks the same as one never pushed.
3. **A stricter prompt only.** The prompt already says the final message carries the closing line; repeating that does nothing for a subagent that ends without writing one.

## Outcome

- `orchestrator.run/run-finish` prints `landed impl/<component>` when the branch has landed, before its `removed` and `kept` lines, and no `landed` line otherwise.
- `plugin/build-unclosed-builds`: the build skill judges a build with no closing line, that no waiting or limit rule holds back, by that line.

## Constraints

- The new line is added output: `clm runner` passes `--finish` output through, and nothing else parses it.
- For a build with no closing line the skill reads only `clm run --finish`'s output, never a `git` command of its own aimed at `origin` or the build's worktree. This sits beside `plugin/build-reads-origin`, which has the skill learn a build's progress from `origin/impl/<component>`; the skill's wording keeps the two apart: progress from `origin`, the landed verdict from the finish.
- `orchestrator.run`'s Contract names the new line, so a `plugin` build's one-hop context carries it (`ancestor-scoped-context`).
- `run-finish` prints `landed impl/<component>` after the `freed` line and before any worktree line, also when no worktree has the branch checked out.

## Edge cases and workarounds

- A build that landed without a closing line gives no commit; the human is told it landed without its closing line. Accepted in `unclosed-build-reads-finish`.

## Open questions

None.
