---
status: applied
---

## Intent

Make what waits on the human catch their eye in the CLM panel. In `clm view queue --format ansi`, every entry line under `waiting on you` is drawn amber, the colour the web app's Waiting on you row already uses, and the panel's Queue tab runs that `ansi` chart on every surface (desktop, VS Code and mobile as well as the terminal), so the highlight shows wherever the pane is.

## Problem

The Queue tab lists questions, human tasks and changes awaiting approval under `waiting on you`, but its entries are drawn in the same plain colour as everything else, so they are easy to miss in a busy queue. On surfaces other than the terminal the tab runs `--format text`, which carries no colour at all, only because the queue has no SVG form.

## Approaches

- **Colour in `clm view`, ANSI on every surface (chosen).** `view` colours the entries (`views-rendered-by-clm`: the panel decides nothing), and the panel runs the Queue tab's `ansi` chart on every surface, as it already does for `clm machine`'s rows, which are text too. The text-only path, and its separate grouping of build rows under name lines, goes, so build rows nest under their running entries on every surface.
- **Colour in `clm view` only.** Smaller, but the highlight would show only on the terminal surface.
- **The panel colours the section itself.** It would have to parse the chart's sections and decide a colour, against `views-rendered-by-clm`, and `clm view queue --format ansi` in a terminal would stay plain.

Amber, not rose: `colour-meanings` keeps rose for what is blocked; an entry waiting on the human asks for attention, like `oversized`, and the web app's Waiting on you row is already amber. The heading `waiting on you` stays indigo, and the words remain, so the chart still reads without colour.

## Outcome

- `view/queue-view`: entry lines under `waiting on you` are amber in `ansi`; `text` and `json` are unchanged.
- `plugin.panel`: the Queue tab runs `--format ansi --width <columns>` on every surface (`views-panel`, `queue-tab`); build rows go under their running entries on every surface (`build-rows`); the text-only drawing path is removed (`no-dead-code`).
- `docs/decisions/colour-meanings.md` adds "waiting on you" to the amber row.
