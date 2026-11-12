---
title: "Sampling timer not built: waits on machine-charts"
kind: workaround
components: [plugin.panel]
requirement: plugin.panel/build-opens-panel@dcdf45be
---

build-opens-panel's text says opening starts the sampling timer, which machine-charts (blocked by machine/machine-history) introduces. Only the redraw timer is started until machine-charts is built; that build must start the sampling timer in openPane.
