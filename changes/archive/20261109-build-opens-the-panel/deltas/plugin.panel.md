---
component: plugin.panel
base: { contract: "d5509b6144fcec21" }
---

## MODIFIED Contract
The Claude Code plugin `clm-panel`, listed in this repo's marketplace after `clm`: the `/clm:view` command, which answers in the chat with `clm status` or `clm view <name>`, and with no argument opens a pane titled `CLM` with four tabs: Spec, Flight and Blockers, showing those views of `origin/main` as of the pane's last fetch (`views-rendered-by-clm`), and Capabilities, showing `clm capabilities` and a capability's page, read-only. It opens the same pane, on Flight, when the `clm:build` skill starts and the pane isn't open (`build-opens-panel`). It displays what `clm` prints and decides nothing about the repository. Under the Flight chart's component lines it adds the one kind of fact `clm` can't see: the builds its own session launched that are running or failed, with their nested subagents, status, steps, context, tokens, estimated cost and time.

## ADDED Requirements

### Requirement: build-opens-panel
The hooks module SHALL hook Claude Code's `skill.prompt` event, which fires when a skill's prompt is expanded for the model, whether the person typed its slash command or the model called the Skill tool, and for the `clm` plugin's `build` skill (named `clm:build`, or `build` with `pluginName` `clm`) SHALL return what `next(e)` resolves to, unchanged. When that skill's prompt is expanded, the session's `cwd` is in a CLM repository as `/clm:view` finds one, and `$.ui.panes()` lists no pane whose `id` is the panel's, it SHALL open the pane as `/clm:view` with no argument does: on the Flight tab with no capability page selected, its redraw timer started and a redraw begun. When the pane is listed, open or waiting for room, it SHALL leave it as it is, its tab and capability page unchanged; outside a CLM repository and for any other skill it SHALL open nothing. The rule deciding whether to open SHALL be a plain function in `plugin-panel/src/` taking the skill's name, its `pluginName`, the panes listed and whether `cwd` is in a CLM repository, tested with `vitest` as the panel's other rules are; a failure while opening SHALL leave the skill's prompt as `next(e)` resolved it. `plugin-panel/declarations/claude-code/index.d.ts` SHALL declare the `skill.prompt` event as far as the module uses it (`type-check`).

#### Scenario: build opens the closed panel
- GIVEN a session in a CLM repository with `clm-panel` loaded and no pane `CLM` listed
- WHEN the `clm:build` skill's prompt is expanded
- THEN the pane `CLM` is opened on the Flight tab, and the prompt the model reads is the one `next(e)` resolved to

#### Scenario: the open panel is left alone
- GIVEN the same session with the pane `CLM` listed, showing the Spec tab
- WHEN the `clm:build` skill's prompt is expanded
- THEN no pane is opened and the tab is still Spec

#### Scenario: not a CLM repository
- GIVEN a session whose `cwd` has no `specs/SPEC.md` up to its git root and no pane `CLM` listed
- WHEN the `clm:build` skill's prompt is expanded
- THEN no pane is opened

#### Scenario: another skill
- GIVEN a session in a CLM repository with no pane `CLM` listed
- WHEN the `clm:propose` skill's prompt is expanded
- THEN no pane is opened

#### Serves
- capability/watch-beside-chat
