---
title: "One meaning per colour on every surface"
date: 2026-10-24
status: accepted
---

Every surface that colours CLM's state, the ANSI charts and SVG of `clm view`, the Claude Code panel that shows them, and the web app, uses the same five colours with the same meanings:

| Colour | Means |
|---|---|
| green | active, converged, current, proven, complete, a finished subagent |
| sky | specd, ready |
| amber | claimed, behind, oversized, a running subagent, a modified element |
| rose | blocked, waiting, obsolete, deprecated, a removed element, a failed subagent, HARD |
| indigo | the accent: headings, a selected PR's touched nodes |

The mapping is a table in `clm-view` (`chart::Style::of`) and tokens on the web app's `:root`, one per colour in a light and a dark palette. A new state is added to a row here first, then to both.

Rules the table follows:
- **Red means attention.** Rose is for what a person or another node is waiting on, or what the next build must remove. Work that is simply not yet built is sky, not a warning.
- **No black or white as a state.** The dark theme's background is near black and the light theme's near white, so neither can mean anything.
- **Colour is never the only cue.** Every state is also a word or a glyph (`█ ▓ ░ ✕`, `claimed`, `blocked`), so a chart reads in a monochrome terminal and for anyone who can't tell red from green or amber from rose.
- **A bar is segmented by state, labelled current of total.** A requirement cited at an older version is `behind`, amber, and never fills a green cell: the graph must not read as done while `clm pending` lists work.

Rejected:
- **Black for missing, red for blocked**, proposed on 2026-10-24. Black collides with the dark theme, and red already meant blocked; the ask was really that the spec graph show behind and blocked work at all, which the segmented bar and the queue-state ring do.
- **A separate counter per state on each node.** Exact but noisy; the per-state counts stay in the `spec` JSON and the inspector.

Updated 2026-10-25: `interactive` left the table with the queue state it named (`builds-in-subagents`); `oversized` joined amber, since an over-budget node asks for attention without blocking anything (`oversized-is-a-state`).

Updated 2026-10-25: the panel's build rows (`20261025-panel-build-rows`) colour a running subagent amber, like a claim, a finished one green and a failed one rose, each with its glyph (`● ✓ ✗`).

Updated 2026-11-12: `clm machine`'s rows (`machine-load-measured-by-clm`) colour a reading `ok` green, `warn` amber and `critical` rose, and dim `unknown`; each state but `ok` is also written as its word after the figure, so the row reads without colour.
