---
component: plugin.panel
base: { design: "88287aea8719fed7" }
---

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

**Capabilities** (`capabilities-tab`, `review-capability-cost`). The tab decides nothing. It runs `clm` through the same cache as the views, and draws text, not a chart, so it takes no `--width` and draws no SVG; its lines wrap at the pane's edge rather than being cut, since a page's prose runs past it. A row's slug must match as well as its position, since the list prints slugs whole and two runs that straddled a rename can agree on the count alone. Each row is a `plain` `Button` with the key `capability:<slug>`; pressing it sets the atom `capability`, a slug or `null`, `null` at first, under its own `shape` in `types/index.d.ts`. While it holds a slug the page's run is started together with the list's runs. `/clm:view` opens the tab on the list, as it opens the pane on Flight. The page's `decide:` line stays text since `clm capabilities decide` is what the guard asks the human about (`plugin.hooks.guard/approve-guard`); the person decides in the chat or on the web. The rules are pure functions in `plugin-panel/src/capabilities.ts`; `register.tsx` only wires them.

**Type-check** (`type-check`, `panel-types-hand-written`). The type-check runs first because the tests strip types, and `claude plugin validate` checks only what the engine would refuse to load. `tsconfig.json` types the module as the engine does (`target` and `lib` `es2023`, `module` `esnext`, `moduleResolution` `bundler`, `strict`, `noUncheckedIndexedAccess`, `noEmit`, `skipLibCheck`, `jsx` `react` with `h` and `Fragment`); `allowImportingTsExtensions` is there since the module imports its files by their `.ts` names. The declarations are never a copy of the ones Claude Code writes, which no package ships and which aren't this repository's to redistribute. When `tsc` and `claude plugin validate` disagree about the module, the file is what's wrong, since the validator reads the module as the engine does. `.claude-plugin/types/`, where the engine lays its own when it hot-reloads the panel from a checkout, stays ignored. The lockfile pins `typescript`, so `npm ci` brings `tsc` and the check needs nothing else on the machine; it runs inside `npm test` rather than as `npx tsc`, which unattended sessions may not run (`unattended-commands`), and a human typing `npm test` here gets the same check as a land. The tests type under the same options as the module, with `vitest`'s own types from `node_modules`, so they use only what the module's environment has (no `Buffer`: the engine has no Node.js), and a fake they pass is the type the rule takes. `claude plugin validate` passes with `declarations/` and `tsconfig.json` present: it reads the manifest, the module and the contract, nothing else.

## MODIFIED Requirements

### Requirement: blocker-chains <!-- base: 8b24bb4f42d118cb -->
On the terminal surface the Blockers tab SHALL draw each root's line (`view/blockers-root-lines`) as a pressable line holding the same text and colours, matched to the root by its position among those lines and the `blockers` model's `roots`, from `clm view blockers --format json` run as the chart is, and SHALL draw the lines of a root's tree only while that root is unfolded. Every root SHALL start folded. Pressing a root's line SHALL fold or unfold that root alone. Which roots are unfolded SHALL be kept, by root name, across redraws and a reload, and a root the model doesn't list SHALL be drawn as nothing and keep its state for when it comes back. When the lines can't be matched (their count differs from the model's, or the JSON run fails or doesn't parse), the tab SHALL draw every line, unfolded and not pressable. Other surfaces SHALL draw the chart's SVG whole, every chain in it.

#### Serves
- capability/see-project-state

#### Needs
- view/blockers-root-lines

#### Scenario: folded at first
- GIVEN a terminal pane on the Blockers tab whose `clm view blockers --format ansi` prints `blockers`, a root line for `gate` followed by two tree lines, and a root line for `host` followed by one, and whose `clm view blockers --format json` lists roots `gate` then `host`
- WHEN the pane draws
- THEN it draws `blockers` and the two root lines, pressable, and none of the tree lines

#### Scenario: unfold one
- GIVEN that pane
- WHEN the person presses `gate`'s line
- THEN `gate`'s two tree lines follow its line, and `host`'s tree line is not drawn

#### Scenario: fold again
- GIVEN `gate` unfolded
- WHEN the person presses `gate`'s line, and in a second run the module is reloaded instead
- THEN after the press none of the tree lines is drawn, and after the reload `gate`'s two tree lines are drawn

#### Scenario: failed run keeps the chains
- GIVEN `gate` unfolded, and a redraw whose `clm view blockers --format json` exits 1, then a later one that succeeds
- WHEN the pane draws each
- THEN the first draws every line, unfolded and not pressable, and the second draws `gate` unfolded and `host` folded

#### Scenario: nothing blocked
- GIVEN a `clm view blockers --format ansi` that prints `blockers` and `nothing is blocked`, and a model with no roots
- WHEN the pane draws
- THEN it draws both lines, with no pressable line

#### Scenario: chains on the desktop
- GIVEN the same repository, on the desktop surface
- WHEN the pane draws the Blockers tab
- THEN it draws `clm view blockers --format svg`'s SVG

### Requirement: superseded-draw <!-- base: 261a69111d77f4b9 -->
A pane draw whose `$` call the engine aborts, because a newer draw replaced it, SHALL end without throwing and draw nothing of its own, so the engine reports no hook as having thrown, and the newer draw SHALL draw the pane. An aborted call is one whose rejection's message ends in `aborted`. A run the engine aborted SHALL NOT be kept as the answer of `clm` or of git, so the newer draw runs it again. Any other failure of a draw SHALL be shown in the pane.

#### Scenario: opening replaces the first draw
- GIVEN a session in a CLM repository with no pane open
- WHEN the person runs `/clm:view`, and the engine aborts the first draw's `clm view` run once the redraw that opening starts replaces that draw
- THEN the first draw's hook resolves without drawing and without throwing, and the second draw runs `clm view` again and shows its chart

#### Scenario: another failure still shows
- GIVEN an open pane whose `clm view` rejects with a message that doesn't end in `aborted`
- WHEN it redraws
- THEN the pane shows why `clm` couldn't start

### Requirement: build-opens-panel <!-- base: 4b2e0c99e7bb7020 -->
The hooks module SHALL hook Claude Code's `skill.prompt` event, which fires when a skill's prompt is expanded for the model, whether the person typed its slash command or the model called the Skill tool, and for the `clm` plugin's `build` skill (named `clm:build`, or `build` with `pluginName` `clm`) SHALL return what `next(e)` resolves to, unchanged. When that skill's prompt is expanded, the session's `cwd` is in a CLM repository as `/clm:view` finds one, and `$.ui.panes()` lists no pane whose `id` is the panel's, it SHALL open the pane as `/clm:view` with no argument does: on the Flight tab with no capability page selected, its redraw timer started and a redraw begun. When the pane is listed, open or waiting for room, it SHALL leave it as it is, its tab and capability page unchanged; outside a CLM repository and for any other skill it SHALL open nothing. A failure while opening SHALL leave the skill's prompt as `next(e)` resolved it.

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
