---
title: "Reconsider plugin.panel/build-opens-panel"
kind: question
components: [plugin.panel]
---

`plugin.panel/build-opens-panel` scores 3 against the reconsider threshold of 3 (workarounds 1, rewritten 2, scenarios 0, exceptions 0).

Workaround flags:
- backlog/workaround-plugin.panel-build-opens-panel-1.md: Sampling timer not built: waits on machine-charts

Archived changes that modified it:
- 20261111-slim-flagged-specs
- 20261112-machine-load-in-panel

Scenarios: 4 (setting 6). Exception clauses: 0 (setting 4).

Three ways out, each a spec change: drop the requirement, change it, or redesign around it. Deleting this item without one brings it back on the component's next build.
