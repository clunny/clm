---
title: "Panel's pane render hook throws when the engine aborts a superseded draw"
kind: bug
components: [plugin.panel]
---

Step: `/clm:view` with no name (opening the CLM panel), and any pane redraw that overlaps a draw still running.

What happened: the transcript shows `clm-panel: ui.render hook skipped: threw HooksError: clm-panel: $.process.run(<plugin>/bin/clm) aborted`. Opening the panel calls `$.ui.open`, which draws the pane, and then immediately `redraw`, which starts a second draw. The engine cancels the first draw's `clm view` run, `runClm` re-throws the abort on purpose (so `cachedRun` doesn't keep it as clm's failure), and the pane's `ui.render` hook lets that rejection escape. The engine then reports the hook as having thrown. The panel itself draws correctly from the newer draw.

Expected: a draw that a newer draw replaced ends quietly. The pane's `ui.render` hook should treat a rejection matching the aborted pattern as superseded and return without drawing, rather than throwing. Optionally, `/clm:view` should not start a redraw right after `ui.open`, which already draws.

Rule involved: the pane's comment says a draw's run the engine cut short is asked again, which holds for the run cache but not for the hook, which still throws.
