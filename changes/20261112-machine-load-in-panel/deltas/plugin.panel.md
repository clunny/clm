---
component: plugin.panel
base: { frontmatter: "08ef719d4d565f68", contract: "adc865046284e11c", design: "427417a1d1ba1ce0" }
---

## MODIFIED Frontmatter
```yaml
title: Panel
status: active
owners: ["@clunny"]
depends_on: [view, effort, machine]
paths: ["plugin-panel/**"]
successor: ""
serves: [capability/watch-beside-chat, capability/see-project-state]
```

## MODIFIED Contract
The Claude Code plugin `clm-panel`, listed in this repo's marketplace after `clm`: the `/clm:view` command, which answers in the chat with `clm status` or `clm view <name>`, and with no argument opens a pane titled `CLM` with three tabs, Spec, Flight and Blockers, showing those views of `origin/main` as of the pane's last fetch (`views-rendered-by-clm`). Above every tab it draws three rows charting the machine's CPU load, available memory and free disk over the last 30 minutes, from `clm machine` (`machine`), sampled every 10 seconds. It opens the same pane, on Flight, when the `clm:build` skill starts and the pane isn't open (`build-opens-panel`). It displays what `clm` prints and decides nothing about the repository or the machine. Under the Flight chart's component lines it adds the one kind of fact `clm` can't see: the builds its own session launched that are running or failed, with their nested subagents, status, steps, context, tokens, estimated cost and time.

## MODIFIED Design
**The CLM panel** (`views-rendered-by-clm`). The panel is its own plugin, `clm-panel`, in `plugin-panel/`. Keeping it out of `clm` means `clm`'s command hooks never share a plugin with a hooks module, and a person on a Claude Code too old for hooks modules simply doesn't install it. It needs `clm`: its `plugin.json` lists `clm` under `dependencies`, and it runs the `clm` plugin's `bin/clm` shim, so it gets the same binary and `version-skew` handling as the Bash tool, or `clm` on `PATH` when it can't find the shim. Its rules (which command to run for a surface and width, ANSI escapes to coloured cells, when to redraw and when to fetch, what to answer for each `/clm:view` argument, and the build rows below) are plain functions in `plugin-panel/src/`, taking a fake of the few `$` calls and events they use, and tested with `vitest` under `npm test`, which CI runs (`ci`). `register.tsx` only wires them to `$`. The `/clm:view` command is the `clm` plugin's, not this module's: `$.command.register` takes a bare name run as `/<name>`, so a hooks module can't register a `clm:`-namespaced command, and `clm-panel`'s own would be `clm-panel:`. `plugin/commands/view.md` makes `/clm:view` exist (`plugin`), and the module hooks `command.run` for `clm:view` and answers it without calling `next`, so no model turn starts. The view tabs run `clm` through `$.process.run` at the session's `cwd`, so they describe what has landed, whichever branch or worktree the session is on. Whether `refs/remotes/origin/main` exists is checked with `git rev-parse --verify -q` after each fetch; the chart's first line names the commit, when it was fetched and the session's branch (`view`'s `view-at-rev`). On the terminal surface the ANSI escapes become coloured text cells; the SVG is drawn with `Svg`. `clm view` itself never fetches. The fetch-failed line's `<age>` is since that fetch started, as `<n>m` under an hour and `<n>h` from then on, as `view-at-rev` writes it. When `clm` isn't installed the pane shows the shim's message instead of a chart. Outside a CLM repository means no `specs/SPEC.md` found as `session-start` finds it. Without `clm-panel`, the command file has the model run `clm status` or `clm view <name>` through the Bash tool, so the panel's answer that `clm` isn't installed (`panel-package`) arises only with the `clm` plugin installed but its shim not found and no `clm` on `PATH`.

**Build rows** (`views-rendered-by-clm`, `builds-in-subagents`). Every build is a background subagent of the human's session, so the panel sees it through the hooks events. The rules are pure functions in `plugin-panel/src/agents.ts`, over a list of runs kept in an atom, with the time in another.
- **Runs.** The `agent.spawn` hook, after `next`, records a run: `id` the started `agentId`, `parentId` the event's `parentAgentId`, `type` its `subagentType`, `description`, `model` as started, `startedAt`. `clm run` prints a launcher of at most five lines that names the prompt file and `git switch impl/<component>` as its first step (`orchestrator.run/build-prompt-file`), on its second line, and a whole build prompt names it on its first line; the model's `description` never decides. Five lines, not the whole prompt, so a subagent whose long prompt quotes a build's first step (a reviewer's diff, a builder check's change) isn't taken for a build; the launcher's lines aren't otherwise read, so its wording can change without the panel. A run whose normalised description a listed run already has is round `n+1` (a build re-launched after it stopped, or a reviewer's fix round). The list keeps the last 200 runs. `session.end` with reason `clear` or `resume` empties it; quitting doesn't, and after a reload the atom holds it, so `session.start` reconciles it with `$.agent.list()`: a run that is `running` or `waiting` whose agent the engine doesn't list is dropped, since nothing will finish it, and the waiting reconciliation below runs over the rest.
- **Updates.** `turn.step` with an `agentId` is one model request of that run: its context is that request's tokens (input, output, cache read and cache creation), its tokens and cost accumulate, its model and effort follow the request, and its step count grows. `turn.complete` with an `agentId` ends the run's turn; on an answer, after `next` resolves, the hook asks `$.agent.list()` whether the agent is `waiting` (held on background work it owns, such as a `clm land --check-only` it ran in the background); a run that saw no step takes the turn's own usage either way. `turn.start` with a listed run's `agentId` is the engine resuming that agent's loop when its background work ends, or a message waking it; it keeps the run's stats, as a resuming `agent.spawn` does. A `turn.step` of a `waiting` or `done` run does the same, in case the engine continues a loop without a new turn. **Waiting reconciliation:** the engine's status may lag the hook, so in the pane's 30-second timer, before its redraw, and at `session.start`, the runs are reconciled with `$.agent.list()` (never in a draw; when the call fails the runs stay as they were): a `done` run whose agent is listed `waiting` is `waiting` again, and a `waiting` run whose agent is listed with any status but `waiting` or `running`, or not listed, is `done`, ended at that time. `tool.call` with an `agentId` feeds that run's task list: `TodoWrite` replaces it, `TaskCreate` appends a pending task, `TaskUpdate` changes a task's status or subject or deletes it; `k` is the completed tasks, `n` the total, and the current step is the first `in_progress` task, else the first `pending`.
- **Cost.** An estimate: a table in `agents.ts` of USD per million tokens, input, output, cache read and cache write, keyed by a regular expression on the model ID, most specific first, with a fallback rate for a model it doesn't know; a context window per family (200k for Haiku, 1M otherwise) gives the fill percentage. The table carries the date it was last checked against the list prices, as `effort`'s does. It holds the same rows, order and fallback as `effort/prices` (`crates/clm-effort/src/prices.rs`), so the pane's estimate and `clm capabilities`' agree; changing a price changes both tables in one change.
- **Drawing.** On the terminal surface the rows go after each component line (`view/flight-component-lines`, found as `component-heading` says, whose ID is whole or as much as fits followed by `…`), each top-level build newest first, followed by its subtree depth-first, children oldest first, indented two cells per level up to four levels, a child's first line prefixed `└`. A top-level run is one whose parent isn't listed. A row is three lines: the status glyph and the description (`colour-meanings`), ` · waiting` dimmed while waiting, and on a failed run's first line, last, the `Dismiss` button (`role` `dismiss`, `key` `dismiss:<id>`); the type, model name (`Fable 5.1`, `Opus 5.5`, from the model ID), effort and `round n` when above 1, dimmed; and a bar of ten cells with its stats, dimmed: the bar is the run's task progress `k/n` in amber when it keeps a task list, else its context fill dimmed; then `k/n <current step> · ` when there is a list, `ctx <p>% · <tokens> tok ≈$<cost> <elapsed>`, tokens as `850`, `85k` or `1.3M`, cost to cents under $10 and tenths above, elapsed as `m:ss` or `h:mm:ss`. `Dismiss` marks the run dismissed in the runs atom; a run that isn't drawn stays in the list, for rounds and the totals. A run that takes its parent's place sits under the nearest drawn ancestor, or, with none, at depth 0 under its top-level ancestor's component line. The `agents` heading is indigo. The totals line reads `≈$<cost> · <tokens> tokens · <time> · <model> ×<n>` per model, most used first, the time spanning the first start to the last end. On the other surfaces the pane has only the SVG and can't tell which components it lists, so components follow in the order of their first build.
- **Ticking.** The tick is a one-second `$.clock.every` setting the time atom.

**Inspector and blocker chains** (`views-rendered-by-clm`). On the terminal surface the charts can be pressed into, as the web app's graph can. The panel draws nothing of its own about the repository: every fact comes from `clm view <name> --format json`, and every line from `clm view <name> --format ansi`. The rules are pure functions in `plugin-panel/src/inspector.ts`, over the ANSI lines `ansiLines` gives, the parsed JSON models and three atoms; `register.tsx` only turns what they return into elements.
- **Which line is which.** `view` pins the lines the pane presses (`view/spec-component-lines`, `view/flight-component-lines`, `view/blockers-root-lines`): on the Spec tab every line after the checkout line is a component's (the chart begins with a header line exactly when the JSON's top-level `checkout` or `at` is present, so the pane skips one line then); on Flight, the lines `component-heading` finds that start with two spaces and aren't `  none`; on Blockers, the lines after `blockers` that start in the first column and aren't `nothing is blocked`, each followed by its tree's lines. Matching by position lets a cut ID (`…`) match. The JSON runs beside the chart, at the same `cwd` with the same `clm`, and is cached per generation as the chart is (`viewKey` takes the format). A count that differs means the two runs straddled a change to the repository; the next generation tries again. Such a draw leaves `selected` and `unfolded` as they were. Colours follow `chart.rs`'s ANSI table, as `ansiLines` reads it: green, sky as `cyan`, amber as `yellow`, rose as `red` and indigo as `blue`.
- **Pressable lines.** Each such line is drawn as a `Button` with `plain`, the line's styled runs as its `Text` children, and `key` `node:<id>` or `root:<name>`, so the surface finds it again after a redraw; the person presses it with a click, or moves the focus ring onto it and presses Enter. A press changes an atom and nothing else.
- **Atoms.** `selected` (a component ID or `null`), `contract` (a boolean) and `unfolded` (a list of root names) are kept like `tab` across a reload, each under its own `shape`, declared in `types/index.d.ts`. `selected` is kept but not drawn on Blockers. No draw writes an atom, since only presses and the hooks events do. The selected component's line is drawn `bold`.
- **The inspector's lines**, in `colour-meanings` (each tab fetches the other's JSON only while a component is selected): first the ID in indigo, bold, and the title, then the `Close` button (`role` `dismiss`); then `<status>`, `owners <owners>` and `depends on <ids>` (or `depends on nothing`), joined by ` · `; then `requirements <current>/<total> current`, followed by `, <n> behind`, `, <n> unbuilt` and `, <n> obsolete` for each non-zero count and `  ⚠ oversized` in amber; then the work in flight: `converged` in green, or the state, coloured as the chart colours it, and its priority, then `pushed <h>h ago, lapses in <h>h` when claimed, `blocked by <blocker> (<requirements>)` for each blocker, then one line per pending requirement, indented two cells: its ID, its state (sky `unbuilt`, amber `behind`, rose `obsolete`), and each `<file>:<line>` of an obsolete one's markers.
- **Changes, backlog and the contract** (`inspector-detail`). The `changes` and `backlog` headings are indigo and ` none` plain, each entry indented two cells, in the model's order, `complete` in green and a backlog item's kind dimmed. A contract line longer than the pane's columns is wrapped at spaces, a word longer than the width cut across lines, with one character to a cell as `lineText` and `--width` assume; blank lines within the text are kept; and nothing is rendered as markdown, since the terminal shows text and the panel draws no view of its own. The rules return this detail as a part of its own beside the inspector's lines, so the lines `component-inspector` pins keep their positions and `register.tsx` places the `Button` after them; `ChartInput` carries the pane's columns for the wrapping, 100 when `bodyColumns` isn't a positive number, as `viewArgs` falls back. The contract is hidden at first because it runs to paragraphs and would push the chart the person just pressed off the pane.
- **Blocker chains.** A folded root's line alone is drawn, whose `holds up <n>` says how much is beneath it.

**Machine load** (`machine-charts`, `machine-load-measured-by-clm`). The charts decide nothing: `clm machine --format json` (`machine/machine-report`) takes the readings and judges each `green`, `amber`, `rose` or `unknown`, and the pane keeps them and draws them. The readings are the atom `machine`, a list of at most 180 samples, each the time it was taken and the JSON's `cpu`, `memory` and `disk` objects, under its own `shape` in `types/index.d.ts`, kept across a reload like `tab`, so a reload doesn't empty the charts; `session.end` leaves it, since the machine's load isn't the session's. The sampling is its own `$.clock.every` of 10 seconds, started and stopped with the redraw timer; a sample that finds the previous run still going is skipped, since a run that measures the builds' worktrees can take seconds. A sample appends to the atom and redraws from the views' cache, never rerunning `clm view`. A row is the label padded to four cells, a space, the chart, a space and the figure: the chart is the newest readings that fit, one cell each, newest on the right, each cell the `k`th of `▁▂▃▄▅▆▇█`, `k` its share of a full scale times 8 rounded up, at least 1 and at most 8, the scale being (cpu `load1 / cores` against 2, so a full block is twice the cores; memory `1 − available / total`; disk `1 − available / total`), so for all three a taller block is a heavier load. The figure is the latest reading's: cpu `<load1 / cores>×` to one decimal, memory `<available> avail` and disk `<available> free`, bytes as `850M`, `12G` or `1.3T` (powers of 1024, one decimal under 10). The row's colour is the latest reading's state, as `colour-meanings` gives green, amber and rose, and `unknown` is dimmed. The rows are text on every surface, placed above the tab's content and the fetch-failed line, so they read the same on Spec, Flight and Blockers. The rules are pure functions in `plugin-panel/src/machine.ts`; `register.tsx` only wires them. Builds' disk is measured by `clm machine`, not the panel, since only `clm` knows the clone's build places.

**Type-check** (`type-check`, `panel-types-hand-written`). The type-check runs first because the tests strip types, and `claude plugin validate` checks only what the engine would refuse to load. `tsconfig.json` types the module as the engine does (`target` and `lib` `es2023`, `module` `esnext`, `moduleResolution` `bundler`, `strict`, `noUncheckedIndexedAccess`, `noEmit`, `skipLibCheck`, `jsx` `react` with `h` and `Fragment`); `allowImportingTsExtensions` is there since the module imports its files by their `.ts` names. The declarations are never a copy of the ones Claude Code writes, which no package ships and which aren't this repository's to redistribute. When `tsc` and `claude plugin validate` disagree about the module, the file is what's wrong, since the validator reads the module as the engine does. `.claude-plugin/types/`, where the engine lays its own when it hot-reloads the panel from a checkout, stays ignored. The lockfile pins `typescript`, so `npm ci` brings `tsc` and the check needs nothing else on the machine; it runs inside `npm test` rather than as `npx tsc`, which unattended sessions may not run (`unattended-commands`), and a human typing `npm test` here gets the same check as a land. The tests type under the same options as the module, with `vitest`'s own types from `node_modules`, so they use only what the module's environment has (no `Buffer`: the engine has no Node.js), and a fake they pass is the type the rule takes. `claude plugin validate` passes with `declarations/` and `tsconfig.json` present: it reads the manifest, the module and the contract, nothing else.

## MODIFIED Requirements

### Requirement: views-panel <!-- base: a97b0ae7e3b77139 -->
`/clm:view` with no argument SHALL open a pane titled `CLM` with tabs Spec, Flight and Blockers, showing Flight first; a tab kept across a reload that is none of the three SHALL read as Flight. The tabs SHALL show the output of `clm view <tab> --at origin/main`, with `--format ansi --width <the pane's columns>` on the terminal surface, drawn in colour, with its component lines pressable and the Blockers tab's chains folded until pressed (`component-inspector`, `blocker-chains`), and with `--format svg` on other surfaces. It SHALL redraw when it opens, when the tab changes, when a turn ends, and every 30 seconds while it is placed, and SHALL run `git fetch --quiet origin` at most once every 2 minutes while it is placed. A press SHALL NOT rerun a `clm` command whose output the pane already holds for this generation. Every `clm view` the pane runs SHALL pass `--at origin/main` while `refs/remotes/origin/main` exists, and no `--at` otherwise. When the latest fetch failed, the pane SHALL draw above the chart an amber line `⚠ fetch failed <age> ago:` and the first line of git's stderr, until a fetch succeeds. When `clm` exits non-zero, the pane SHALL show its stderr. The pane SHALL draw no `Image`, offer no `Picture` button and never run `clm view` with `--format png`. Outside a CLM repository, `/clm:view` SHALL say so and open no pane.

#### Scenario: terminal pane
- GIVEN a session on the terminal surface in a CLM repository, with a pane 80 columns wide
- WHEN the person runs `/clm:view`
- THEN the pane runs `clm view flight --at origin/main --format ansi --width 80` and shows its text, coloured as its escapes say

#### Scenario: desktop pane
- GIVEN the same session on the desktop surface
- WHEN the person opens the Spec tab
- THEN the pane runs `clm view spec --at origin/main --format svg` and draws the SVG

#### Scenario: three tabs
- GIVEN a pane whose kept tab, from an earlier version of the panel, is `capabilities`
- WHEN the module is reloaded and the pane draws
- THEN its tab row holds Spec, Flight and Blockers only, and it shows Flight

#### Scenario: refresh and fetch
- GIVEN an open pane
- WHEN a turn ends, and again 30 seconds later, and three turns end within a minute
- THEN it reruns `clm view` each time, and runs `git fetch --quiet origin` no more than once in those 2 minutes

#### Scenario: press reruns nothing
- GIVEN a terminal pane on the Spec tab with `a` selected, having run `clm view spec --format ansi`, `clm view spec --format json` and `clm view flight --format json` this generation
- WHEN the person presses `b`'s line, and in a second run presses `Contract`
- THEN no `clm` command has run again

#### Scenario: fetch fails
- GIVEN an open pane whose `git fetch --quiet origin` exits 128 with `fatal: unable to access` on stderr
- WHEN it redraws, and again after a later fetch succeeds
- THEN the first chart is preceded by an amber line starting `⚠ fetch failed` and holding `fatal: unable to access`, and the second has no such line

#### Scenario: no origin/main
- GIVEN an open pane in a repository with no `refs/remotes/origin/main`
- WHEN it redraws
- THEN it runs `clm view flight` without `--at` and shows its chart

#### Scenario: clm fails
- GIVEN an open pane whose `clm view` exits 1 with violations on stderr
- WHEN it redraws
- THEN it shows those violations

#### Scenario: not a CLM repository
- GIVEN a session whose `cwd` has no `specs/SPEC.md` up to its git root
- WHEN the person runs `/clm:view`
- THEN it says the directory isn't a CLM repository and no pane opens

#### Scenario: no pictures
- GIVEN a session on the terminal surface whose resolved elements include `Image`, with a pane 80 columns wide
- WHEN the pane draws each of the Spec, Flight and Blockers tabs
- THEN its tab row has no `Picture` button, it draws no `Image`, and none of its `clm` runs passes `--format png`

### Requirement: build-opens-panel <!-- base: 8c4ab410fd4199dc -->
The hooks module SHALL hook Claude Code's `skill.prompt` event, which fires when a skill's prompt is expanded for the model, whether the person typed its slash command or the model called the Skill tool, and for the `clm` plugin's `build` skill (named `clm:build`, or `build` with `pluginName` `clm`) SHALL return what `next(e)` resolves to, unchanged. When that skill's prompt is expanded, the session's `cwd` is in a CLM repository as `/clm:view` finds one, and `$.ui.panes()` lists no pane whose `id` is the panel's, it SHALL open the pane as `/clm:view` with no argument does: on the Flight tab, its redraw and sampling timers started and a redraw begun. When the pane is listed, open or waiting for room, it SHALL leave it as it is, its tab unchanged; outside a CLM repository and for any other skill it SHALL open nothing. A failure while opening SHALL leave the skill's prompt as `next(e)` resolved it.

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

## ADDED Requirements

### Requirement: machine-charts
While the pane is placed it SHALL run `clm machine --format json` when it opens and every 10 seconds, starting none while the previous run is still going, and SHALL keep the readings of the last 30 minutes, at most 180, oldest dropped first, across a reload and a cleared session. Above each tab's content, on every surface, it SHALL draw three rows of text, `cpu`, `mem` and `disk`: each a chart of one block from `▁▂▃▄▅▆▇█` per kept reading, newest on the right, as many of the newest as fit the pane's columns beside the label and the figure, scaled so that a taller block is a heavier load (cpu as `load1 / cores` against twice the cores, memory and disk as the used share of their total), then the latest figure: `<load1 / cores>×` to one decimal, `<available> avail` and `<available> free`, bytes as `850M`, `12G` or `1.3T`. Each row SHALL be drawn in the colour of its latest reading's state, green, amber or rose, and dimmed when it is `unknown`. A sample SHALL NOT rerun `clm view`. When `clm machine` exits non-zero or its output doesn't parse, the pane SHALL keep no reading for that sample and draw, while no later sample has succeeded, one dimmed line `machine: ` and the first line of its stderr, or `unreadable output`, in place of the rows when it holds no reading, and under them otherwise. With no reading and no failure yet, it SHALL draw no row.

#### Serves
- capability/watch-machine-load

#### Needs
- machine/machine-report

#### Scenario: the rows
- GIVEN a terminal pane 40 columns wide that has taken three samples, the last with `cpu` `load1` 8 of 8 cores `amber`, memory 4G available of 16G `green`, and disk 20G available of 100G `rose`
- WHEN the pane draws, on the terminal surface and again on the desktop surface
- THEN both times it draws rows `cpu`, `mem` and `disk` above the tab, each ending in three blocks, the last of `cpu` `▄`, and figures `1.0×`, `4.0G avail` and `20G free`, coloured amber, green and rose

#### Scenario: the newest fit
- GIVEN a pane 30 columns wide that has taken 180 samples
- WHEN it draws
- THEN each row is at most 30 cells wide and its chart ends with the newest sample

#### Scenario: thirty minutes kept
- GIVEN a pane that has taken 180 samples
- WHEN 10 seconds pass and the module is then reloaded
- THEN it holds 180 readings, the oldest of the earlier ones gone and the newest the one just taken

#### Scenario: a sample reruns no view
- GIVEN an open pane on the Flight tab that has run `clm view flight` this generation
- WHEN 10 seconds pass
- THEN it has run `clm machine --format json` once more and `clm view` no more

#### Scenario: one run at a time
- GIVEN a pane whose `clm machine` run takes 25 seconds
- WHEN 30 seconds pass
- THEN it has started one `clm machine` run

#### Scenario: unknown
- GIVEN a sample whose `cpu` state is `unknown`
- WHEN the pane draws
- THEN the `cpu` row is dimmed

#### Scenario: clm machine fails
- GIVEN a pane holding no reading, whose `clm machine` exits 1 with `boom` on stderr
- WHEN it draws, and again after a later sample succeeds
- THEN the first draw shows the dimmed line `machine: boom` and no row, and the second shows the rows and no such line

## REMOVED Requirements

### Requirement: capabilities-tab <!-- base: fd439d25eb9f37f7 -->
