---
title: "clm computes and draws the views; the panel and the chat only display them"
date: 2026-10-23
status: accepted
---

The views of the spec graph and the work in flight (`spec`, `flight`, `blockers`) are computed once, by the `view` node, from the spec graph, `clm_check::report` and `queue_from_refs`. `clm view <name>` draws each one in every form a surface needs: a box-drawing chart (plain, or coloured with ANSI), SVG, and JSON. The Claude Code panel and the chat only show what `clm` printed, and the web app draws its own interactive graphs from the same JSON, computed in its Worker by the same crate. Every surface shows the same facts, and every rule about what a view contains is in Rust, where tests prove it (`proven-by-tests`).

**What the panel adds of its own.** Under the chart's node lines the panel shows what exists nowhere but in its own Claude Code session: the builds it launched, their nested subagents, and their status, steps, context, tokens, cost and time, read from the hooks events. `clm` can't see a session and the web app has none, so this is the one kind of fact the panel draws itself. Everything derived from git stays `clm`'s, and `view` pins the shape of the node lines the panel matches so a chart change can't break the match silently.

**The panel is interactive on the terminal surface without drawing a view itself.** It keeps `clm`'s ANSI lines and makes the ones `view` pins pressable: a node's line on the Spec and Flight tabs selects it and opens an inspector over the chart, built from `clm view spec --format json` and `clm view flight --format json` (title, status, owners, dependencies, requirement counts, work in flight); a root's line on the Blockers tab unfolds the chain it holds up, every root starting folded. Lines are matched to the model by position, which `view` pins (`spec-node-lines`, `flight-node-lines`, `blockers-root-lines`), so the panel parses no chart text beyond finding those lines. What the models don't hold (a node's contract, its open changes, its backlog items) stays out of the panel until a model carries it (`backlog/panel-inspector-contract`). The SVG, which the pane's desktop and web surfaces draw, takes no presses.

Rejected:
- **The panel draws for itself**, in TypeScript from the JSON, perhaps sharing the web's layout, or lays out its own tree for the inspector. A second full renderer to keep in step with the web, outside the Rust tests that prove requirements.
- **A list of node buttons beside the chart.** It repeats every node a second time in a narrow pane.
- **Text only, no panel.** The cheapest, but no live view and no graphics, which the human asked for.
- **Pictures in the terminal** (kitty, Ghostty, iTerm2 graphics; `clm view --format png`, the SVG rasterised with `resvg`, behind a picture mode in the pane). They don't pass through every multiplexer, and nothing but the terminal used them: the `web` Worker links only `clm-view`'s models, with its default features off, and draws its own graphs, and the desktop and web surfaces of the pane draw the SVG. In the terminal the coloured box-drawing chart already shows every fact, works everywhere and is the only form that can be pressed into, so pictures cost a rasteriser, a font lookup, a second drawing path in the panel and a share of `view`'s size budget for something not needed.
