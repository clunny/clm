---
title: "One meaning per colour on every surface"
date: 2026-10-24
status: accepted
---

Every surface that colours CLM's state, the ANSI charts and SVG of `clm view`, the Claude Code panel that shows them, `clm machine`'s rows and chart, and the web app, uses the same five colours with the same meanings:

| Colour | Means |
|---|---|
| green | active, converged, current, proven, complete, a finished subagent, an `ok` machine reading |
| sky | unbuilt, ready |
| amber | claimed, behind, oversized, waiting on you, a running subagent, a modified element, a `warn` machine reading |
| rose | blocked, waiting, obsolete, deprecated, a removed element, a failed subagent, a change that needs review, a `critical` machine reading |
| indigo | the accent: headings, a selected PR's touched nodes |

The mapping is a table in `clm-view` (`chart::Style::of`) and tokens on the web app's `:root`, one per colour in a light and a dark palette. A new state is added to a row here first, then to both.

Rules the table follows:
- **Red means blocked, amber means look.** Rose is for what blocks work: a blocked or waiting node, what the next build must remove. Amber asks for attention without blocking anything by itself: an over-budget node (`oversized-is-a-state`), or an entry waiting on the human, in the queue chart's `waiting on you` section as in the web app's Waiting on you row. Work that is simply not yet built is sky, not a warning.
- **No black or white as a state.** The dark theme's background is near black and the light theme's near white, so neither can mean anything.
- **Colour is never the only cue.** Every state is also a word or a glyph (`█ ▓ ░ ✕`, `claimed`, `blocked`), so a chart reads in a monochrome terminal and for anyone who can't tell red from green or amber from rose. The panel's build rows colour a running subagent like a claim, each with its glyph (`● ✓ ✗`). `clm machine`'s rows (`machine-load-measured-by-clm`) dim an `unknown` reading and write each state but `ok` as its word after the figure, so the row reads without colour. The one exception is `clm machine`'s chart cells (`machine-cells-keep-their-state`): each takes the colour of the state its sample was judged at, with no other cue. The current reading's state is still written as its word, and a past cell's state is history that would otherwise have no cue at all.
- **A bar is segmented by state, labelled current of total.** A requirement cited at an older version is `behind`, amber, and never fills a green cell: the graph must not read as done while `clm pending` lists work.

Rejected:
- **Black for missing, red for blocked**, proposed on 2026-10-24. Black collides with the dark theme, and red already meant blocked; the ask was really that the spec graph show behind and blocked work at all, which the segmented bar and the queue-state ring do.
- **A separate counter per state on each node.** Exact but noisy; the per-state counts stay in the `spec` JSON and the inspector.
