---
status: done
resolves:
  - panel-s-pane-render-hook-throws-when-the-engine-aborts-a
---

# A superseded pane draw ends quietly

## Intent

The CLM panel's pane draw ends quietly when a newer draw replaces it. Opening the pane with `/clm:view` calls `$.ui.open`, which draws, and then starts a redraw; the engine aborts the first draw's `clm view` run, `runClm` re-throws the abort on purpose so `cachedRun` doesn't keep it as `clm`'s failure, and the pane's `ui.render` hook lets that rejection escape, so the transcript shows `clm-panel: ui.render hook skipped: threw HooksError: ... aborted`. The pane itself is right, drawn by the newer draw. The new requirement `plugin.panel/superseded-draw` states what no requirement said: a draw the engine aborted for a newer one resolves without drawing and without throwing.

## Problem

- `register.tsx`'s pane hook awaits `hasOriginMain` and `runClm` through `cachedRun`; both re-throw a rejection matching `ABORTED` (`/\baborted$/`) so no cache keeps it, and nothing catches it above them.
- The hook's comment ("A draw's run the engine cut short is asked again") holds for the run cache but not for the hook.
- `views-panel` says nothing about a superseded draw, so the fix needs a written rule (`code-fix-needs-a-written-rule`).

## Approaches

1. **Catch the abort in the pane hook (chosen).** The hook's work that may reject with an aborted `$` call ends, when it does, without drawing (an empty node), and any other rejection propagates as now. The abort's re-throw from `runClm` and `hasOriginMain` stays, so the caches still forget the aborted run and the newer draw runs it again. The test of this rule lives on a pure function in `plugin-panel/src/panel.ts` (for instance one that runs a draw and maps an aborted rejection to "nothing to draw"), which `register.tsx` wraps around the hook's body, as the panel's other rules are.
2. **Don't redraw right after `$.ui.open`.** Rejected: the redraw after opening is what bumps the generation and runs `fetchIfDue`, which `views-panel` requires when the pane opens; dropping it would leave the pane on an unfetched `origin/main` until the first turn ends. And any overlapping redraw (a tab press during a 30-second redraw, a turn ending mid-draw) hits the same abort, so the hook has to handle it anyway.
3. **Return the abort from `runClm` as a `Run` instead of throwing.** Rejected: `cachedRun` would then keep the aborted run as the generation's answer and the newer draw would show it, which is why the re-throw exists.

## Outcome

- `plugin.panel/superseded-draw` (added): a draw aborted for a newer one resolves without drawing or throwing; aborted runs are still not cached; other failures are drawn as before.

## Edge cases

- An aborted call outside `clm view` (the `git rev-parse` of `hasOriginMain`, the JSON model runs started beside the chart) is caught the same way, since they share the hook's body.
- A rejection that isn't an abort (a hook bug) still throws, so a real failure stays visible.

## Build notes

Settled from the builder check:

- **What "draws nothing" returns.** The pane hook is declared `Promise<RenderNode>`, and `RenderNode` is `RenderElement | string`, so a superseded draw returns `''`. The hand-written declaration stays as it is. The engine already ignores a superseded draw's result in favour of the newer draw. The hook throwing is the only symptom seen.
- **The pure function.** `plugin-panel/src/panel.ts` exports `isAborted(e: unknown): boolean`, which tests `ABORTED` against the error's message. `runClm`, `hasOriginMain` and `fetchIfDue` reuse it in place of their copies of that test (`reuse-first`). It also exports a wrapper such as `drawOrNothing<T>(draw: () => Promise<T>): Promise<T | undefined>`, which resolves to `undefined` for an aborted rejection and rethrows any other. `register.tsx` wraps the whole pane hook body in it and maps `undefined` to `''`. The names are the builder's to choose.
- **Scope.** Only the Pane `ui.render` hook changes. `redraw`'s `fetchIfDue` already maps an abort to nothing. The `command.run` and `session.start` hooks are untouched.
- **Tests.** `register.tsx` isn't unit-tested, so the tests cover the pure function:
  - An aborted rejection resolves to nothing.
  - Any other rejection rethrows.
  - The existing tests that `runClm` and `hasOriginMain` rethrow an abort, and that `cachedRun` forgets a rejected run, cover the newer draw running `clm view` again. They cite the new requirement too.

## Open questions

None.
