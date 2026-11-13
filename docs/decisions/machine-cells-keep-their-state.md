---
title: "Each machine chart cell keeps the colour of the state its sample was judged at"
date: 2026-11-12
status: accepted
---

Each sample `clm machine` keeps in `<git common dir>/clm/machine-samples` records, beside each reading's share, the state that run judged it at, and each chart cell is painted in its own sample's state; the label, figure and state word take the current reading's. This amends `machine-load-measured-by-clm`'s "a history, not a gauge", whose rows were painted whole in the latest state, so a spell of amber or rose turned green the moment the load fell back and the chart lost the history it was there to show.

- **The state is recorded, not judged again when drawing.** The disk's state depends on the measured `peak` and the free places at that moment, and the CPU's `critical` on the 5-minute load, none of which the share holds; recording the raw readings and the record to judge later would store more to reach the same answer.
- **A sample written before samples held states is no sample**, dropped like a malformed line when the file is next written. Such lines are at most 30 minutes old, so the cost is one half-hour of chart, once, after the upgrade. Reading them as `unknown` would paint them dimmed, which `colour-meanings` reads as "no reading", false here; reading them as `ok` would paint green over moments that may have been amber or rose, the fault this fixes. Either would also keep a parser for the old form that is dead 30 minutes after the upgrade (`no-dead-code`, `yagni`).
- **A past cell's state is shown by colour alone**, the one exception to `colour-meanings`' "colour is never the only cue". The block's height already shows the share, so a second glyph per state would make one cell mean two things at once, and the pane draws each cell one character wide. The current reading's state is still written as its word, and a past cell's state had no cue at all before this, so a monochrome terminal or a reader who can't tell the colours apart loses nothing they had.
