---
component: plugin.panel
base: { contract: "baf398a894137cfd", design: "c38074f6d964e862" }
---

## MODIFIED Contract
The Claude Code plugin `clm-panel`, listed in this repo's marketplace after `clm`: the `/clm:view` command, which answers in the chat with `clm status` or `clm view <name>`, and with no argument opens a pane titled `CLM` with four tabs, Queue, Spec, Flight and Blockers, showing those views of `origin/main` as of the pane's last fetch (`views-rendered-by-clm`). Above every tab it shows the three rows `clm machine --format ansi` draws, charting the machine's CPU load, available memory and free disk over the last 30 minutes (`machine`), run every 10 seconds. It opens the same pane, on Queue, when the `clm:build` skill starts and the pane isn't open (`build-opens-panel`). It displays what `clm` prints and decides nothing about the repository or the machine. Under the Queue chart's running entries it adds the one kind of fact `clm` can't see: the runs its own session launched for the build loop that are running or failed, with their nested subagents, status, steps, context, tokens, estimated cost and time. Above the Queue chart its buttons Drain and Stop now run `clm build stop --drain` and `clm build stop --now`, which only record the human's request for the loop (`orchestrator.loop`).

## MODIFIED Design
**The CLM panel** (`views-rendered-by-clm`). The panel is its own plugin, `clm-panel`, in `plugin-panel/`. Keeping it out of `clm` means `clm`'s command hooks never share a plugin with a hooks module, and a person on a Claude Code too old for hooks modules simply doesn't install it. It needs `clm`: its `plugin.json` lists `clm` under `dependencies`, and it runs the `clm` plugin's `bin/clm` shim, so it gets the same binary and `version-skew` handling as the Bash tool, or `clm` on `PATH` when it can't find the shim. Its rules (which command to run for a surface and width, ANSI escapes to coloured cells, when to redraw and when to fetch, what to answer for each `/clm:view` argument, and the build rows below) are plain functions in `plugin-panel/src/`, taking a fake of the few `$` calls and events they use, and tested with `vitest` under `npm test`, which CI runs (`ci`). `register.tsx` only wires them to `$`. The `/clm:view` command is the `clm` plugin's, not this module's: `$.command.register` takes a bare name run as `/<name>`, so a hooks module can't register a `clm:`-namespaced command, and `clm-panel`'s own would be `clm-panel:`. `plugin/commands/view.md` makes `/clm:view` exist (`plugin`), and the module hooks `command.run` for `clm:view` and answers it without calling `next`, so no model turn starts. The view tabs run `clm` through `$.process.run` at the session's `cwd`, so they describe what has landed, whichever branch or worktree the session is on. Whether `refs/remotes/origin/main` exists is checked with `git rev-parse --verify -q` after each fetch; the chart's first line names the commit, when it was fetched and the session's branch (`view`'s `view-at-rev`). On the terminal surface the ANSI escapes become coloured text cells; the SVG is drawn with `Svg`. `clm view` itself never fetches. The fetch-failed line's `<age>` is since that fetch started, as `<n>m` under an hour and `<n>h` from then on, as `view-at-rev` writes it. When `clm` isn't installed the pane shows the shim's message instead of a chart. Outside a CLM repository means no `specs/SPEC.md` found as `session-start` finds it. Without `clm-panel`, the command file has the model run `clm status` or `clm view <name>` through the Bash tool, so the panel's answer that `clm` isn't installed (`panel-package`) arises only with the `clm` plugin installed but its shim not found and no `clm` on `PATH`.

**Build rows** (`views-rendered-by-clm`, `builds-in-subagents`). Every build is a background subagent of the human's session, so the panel sees it through the hooks events. The rules are pure functions in `plugin-panel/src/agents.ts`, over a list of runs kept in an atom, with the time in another.
- **Runs.** The `agent.spawn` hook, after `next`, records a run: `id` the started `agentId`, `parentId` the event's `parentAgentId`, `type` its `subagentType`, `description`, `model` as started, `startedAt`. The build loop's launcher names the run on its first line, `loop run <name>` (`orchestrator.loop/step-actions`); the model's `description` never decides. Five lines, not the whole prompt, so a subagent whose long prompt quotes a launcher (a reviewer's diff, a builder check's change) isn't taken for a run; the launcher's lines aren't otherwise read, so its wording can change without the panel. A run whose normalised description a listed run already has is round `n+1` (a build re-launched after it stopped, or a reviewer's fix round). The list keeps the last 200 runs. `session.end` with reason `clear` or `resume` empties it; quitting doesn't, and after a reload the atom holds it, so `session.start` reconciles it with `$.agent.list()`: a run that is `running` or `waiting` whose agent the engine doesn't list is dropped, since nothing will finish it, and the waiting reconciliation below runs over the rest.
- **Updates.** `turn.step` with an `agentId` is one model request of that run: its context is that request's tokens (input, output, cache read and cache creation), its tokens and cost accumulate, its model and effort follow the request, and its step count grows. `turn.complete` with an `agentId` ends the run's turn; on an answer, after `next` resolves, the hook asks `$.agent.list()` whether the agent is `waiting` (held on background work it owns, such as a `clm land --check-only` it ran in the background); a run that saw no step takes the turn's own usage either way. `turn.start` with a listed run's `agentId` is the engine resuming that agent's loop when its background work ends, or a message waking it; it keeps the run's stats, as a resuming `agent.spawn` does. A `turn.step` of a `waiting` or `done` run does the same, in case the engine continues a loop without a new turn. **Waiting reconciliation:** the engine's status may lag the hook, so in the pane's 30-second timer, before its redraw, and at `session.start`, the runs are reconciled with `$.agent.list()` (never in a draw; when the call fails the runs stay as they were): a `done` run whose agent is listed `waiting` is `waiting` again, and a `waiting` run whose agent is listed with any status but `waiting` or `running`, or not listed, is `done`, ended at that time. `tool.call` with an `agentId` feeds that run's task list: `TodoWrite` replaces it, `TaskCreate` appends a pending task, `TaskUpdate` changes a task's status or subject or deletes it; `k` is the completed tasks, `n` the total, and the current step is the first `in_progress` task, else the first `pending`.
- **Cost.** An estimate: a table in `agents.ts` of USD per million tokens, input, output, cache read and cache write, keyed by a regular expression on the model ID, most specific first, with a fallback rate for a model it doesn't know; a context window per family (200k for Haiku, 1M otherwise) gives the fill percentage. The table carries the date it was last checked against the list prices, as `effort`'s does. It holds the same rows, order and fallback as `effort/prices` (`crates/clm-effort/src/prices.rs`), so the pane's estimate and `clm capabilities`' agree; changing a price changes both tables in one change.
- **Drawing.** On the terminal surface the rows go after the Queue chart's running entry whose name the run's launcher names (`build-rows`), the entry found as the line under the `running` heading that starts with two spaces and the name, whole or as much as fits followed by `…`, each top-level run newest first, followed by its subtree depth-first, children oldest first, indented two cells per level up to four levels, a child's first line prefixed `└`. A top-level run is one whose parent isn't listed. A row is three lines: the status glyph and the description (`colour-meanings`), ` · waiting` dimmed while waiting, and on a failed run's first line, last, the `Dismiss` button (`role` `dismiss`, `key` `dismiss:<id>`); the type, model name (`Fable 5.1`, `Opus 5.5`, from the model ID), effort and `round n` when above 1, dimmed; and a bar of ten cells with its stats, dimmed: the bar is the run's task progress `k/n` in amber when it keeps a task list, else its context fill dimmed; then `k/n <current step> · ` when there is a list, `ctx <p>% · <tokens> tok ≈$<cost> <elapsed>`, tokens as `850`, `85k` or `1.3M`, cost to cents under $10 and tenths above, elapsed as `m:ss` or `h:mm:ss`. `Dismiss` marks the run dismissed in the runs atom; a run that isn't drawn stays in the list, for rounds and the totals. A run that takes its parent's place sits under the nearest drawn ancestor, or, with none, at depth 0 under its top-level ancestor's component line. The `agents` heading is indigo. The totals line reads `≈$<cost> · <tokens> tokens · <time> · <model> ×<n>` per model, most used first, the time spanning the first start to the last end. On the other surfaces the pane has only the SVG and can't tell which components it lists, so components follow in the order of their first build.
- **Ticking.** The tick is a one-second `$.clock.every` setting the time atom.

**Inspector and blocker chains** (`views-rendered-by-clm`). On the terminal surface the charts can be pressed into, as the web app's graph can. The panel draws nothing of its own about the repository: every fact comes from `clm view <name> --format json`, and every line from `clm view <name> --format ansi`. The rules are pure functions in `plugin-panel/src/inspector.ts`, over the ANSI lines `ansiLines` gives, the parsed JSON models and three atoms; `register.tsx` only turns what they return into elements.
- **Which line is which.** `view` pins the lines the pane presses (`view/spec-component-lines`, `view/flight-component-lines`, `view/blockers-root-lines`): on the Spec tab every line after the checkout line is a component's (the chart begins with a header line exactly when the JSON's top-level `checkout` or `at` is present, so the pane skips one line then); on Flight, the lines `component-heading` finds that start with two spaces and aren't `  none`; on Blockers, the lines after `blockers` that start in the first column and aren't `nothing is blocked`, each followed by its tree's lines. Matching by position lets a cut ID (`…`) match. The JSON runs beside the chart, at the same `cwd` with the same `clm`, and is cached per generation as the chart is (`viewKey` takes the format). A count that differs means the two runs straddled a change to the repository; the next generation tries again. Such a draw leaves `selected` and `unfolded` as they were. Colours follow `chart.rs`'s ANSI table, as `ansiLines` reads it: green, sky as `cyan`, amber as `yellow`, rose as `red` and indigo as `blue`.
- **Pressable lines.** Each such line is drawn as a `Button` with `plain`, the line's styled runs as its `Text` children, and `key` `node:<id>` or `root:<name>`, so the surface finds it again after a redraw; the person presses it with a click, or moves the focus ring onto it and presses Enter. A press changes an atom and nothing else.
- **Atoms.** `selected` (a component ID or `null`), `contract` (a boolean) and `unfolded` (a list of root names) are kept like `tab` across a reload, each under its own `shape`, declared in `types/index.d.ts`. `selected` is kept but not drawn on Blockers. No draw writes an atom, since only presses and the hooks events do. The selected component's line is drawn `bold`.
- **The inspector's lines**, in `colour-meanings` (each tab fetches the other's JSON only while a component is selected): first the ID in indigo, bold, and the title, then the `Close` button (`role` `dismiss`); then `<status>`, `owners <owners>` and `depends on <ids>` (or `depends on nothing`), joined by ` · `; then `requirements <current>/<total> current`, followed by `, <n> behind`, `, <n> unbuilt` and `, <n> obsolete` for each non-zero count and `  ⚠ oversized` in amber; then the work in flight: `converged` in green, or the state, coloured as the chart colours it, and its priority, then `pushed <h>h ago, lapses in <h>h` when claimed, `blocked by <blocker> (<requirements>)` for each blocker, then one line per pending requirement, indented two cells: its ID, its state (sky `unbuilt`, amber `behind`, rose `obsolete`), and each `<file>:<line>` of an obsolete one's markers.
- **Changes, backlog and the contract** (`inspector-detail`). The `changes` and `backlog` headings are indigo and ` none` plain, each entry indented two cells, in the model's order, `complete` in green and a backlog item's kind dimmed. A contract line longer than the pane's columns is wrapped at spaces, a word longer than the width cut across lines, with one character to a cell as `lineText` and `--width` assume; blank lines within the text are kept; and nothing is rendered as markdown, since the terminal shows text and the panel draws no view of its own. The rules return this detail as a part of its own beside the inspector's lines, so the lines `component-inspector` pins keep their positions and `register.tsx` places the `Button` after them; `ChartInput` carries the pane's columns for the wrapping, 100 when `bodyColumns` isn't a positive number, as `viewArgs` falls back. The contract is hidden at first because it runs to paragraphs and would push the chart the person just pressed off the pane.
- **Blocker chains.** A folded root's line alone is drawn, whose `holds up <n>` says how much is beneath it.

**Machine load** (`machine-charts`, `machine-load-measured-by-clm`). The rows are `clm`'s, as the views are: `clm machine --format ansi --width <the pane's columns>` (`machine/machine-history`) keeps the 30-minute history in the clone, judges each reading and draws the rows, and the pane shows its lines as `ansiLines` reads them, on every surface, since they are text. The sampling is its own `$.clock.every` of 10 seconds, started and stopped with the redraw timer; a sample that finds the previous run still going is skipped, since a run that measures the builds' worktrees can take seconds. The pane keeps the last output in memory only, since the history is `clm`'s, and a sample redraws from the views' cache, never rerunning `clm view`. The rows go above the tab's content and the fetch-failed line, so they read the same on every tab. The rules are pure functions in `plugin-panel/src/machine.ts`; `register.tsx` only wires them.

**Type-check** (`type-check`, `panel-types-hand-written`). The type-check runs first because the tests strip types, and `claude plugin validate` checks only what the engine would refuse to load. `tsconfig.json` types the module as the engine does (`target` and `lib` `es2023`, `module` `esnext`, `moduleResolution` `bundler`, `strict`, `noUncheckedIndexedAccess`, `noEmit`, `skipLibCheck`, `jsx` `react` with `h` and `Fragment`); `allowImportingTsExtensions` is there since the module imports its files by their `.ts` names. The declarations are never a copy of the ones Claude Code writes, which no package ships and which aren't this repository's to redistribute. When `tsc` and `claude plugin validate` disagree about the module, the file is what's wrong, since the validator reads the module as the engine does. `.claude-plugin/types/`, where the engine lays its own when it hot-reloads the panel from a checkout, stays ignored. The lockfile pins `typescript`, so `npm ci` brings `tsc` and the check needs nothing else on the machine; it runs inside `npm test` rather than as `npx tsc`, which unattended sessions may not run (`unattended-commands`), and a human typing `npm test` here gets the same check as a land. The tests type under the same options as the module, with `vitest`'s own types from `node_modules`, so they use only what the module's environment has (no `Buffer`: the engine has no Node.js), and a fake they pass is the type the rule takes. `claude plugin validate` passes with `declarations/` and `tsconfig.json` present: it reads the manifest, the module and the contract, nothing else.

**Queue tab and its buttons** (`queue-tab`, `queue-stop-buttons`). The Queue tab is the build loop's plan as `clm view queue` renders it (`view/queue-view`), so the pane shows what the loop acts on and decides nothing (`views-rendered-by-clm`). The tab has no SVG form, so on surfaces that draw the SVG for the other tabs it shows the `text` chart. The buttons only record the human's request with `clm build stop`; the loop's `clm build wait` sees it within seconds and wakes the session, which acts on it. They are drawn whatever the loop's state, since the request also stops a loop the next session adopts; `clm build stop` needs no lease.

## ADDED Requirements
### Requirement: queue-tab
The Queue tab SHALL show `clm view queue --at origin/main`, with `--format ansi --width <the pane's columns>` on the terminal surface and `--format text` on other surfaces, redrawn and fetched as the other tabs are (`views-panel`), and with the build rows under its running entries (`build-rows`). Its lines SHALL NOT be pressable, it SHALL draw no inspector, and the pane SHALL run no `clm view queue --format json` for it.

#### Scenario: queue on the terminal
- GIVEN an open pane 90 columns wide on the terminal surface showing the Queue tab
- WHEN it redraws
- THEN it runs `clm view queue --at origin/main --format ansi --width 90` and shows its lines in colour

#### Serves
- capability/watch-beside-chat

#### Needs
- view.repo/view-queue

### Requirement: queue-stop-buttons
Above the Queue tab's chart the pane SHALL draw two buttons, `Drain` (`key` `loop:drain`) and `Stop now` (`key` `loop:now`), which run `clm build stop --drain` and `clm build stop --now` in the session's `cwd` when pressed, and then redraw. When the command exits non-zero the pane SHALL show its stderr above the chart until the next press or redraw. While the plan's loop line says a stop is requested, the pane SHALL draw the pressed button's label dimmed with `requested`. Its build adds the capability test of `capability/watch-beside-chat`'s scenario *stop from the pane* to `capabilities/panel/watch-beside-chat.test.ts` (`capability-tests-follow-rules`).

#### Scenario: drain pressed
- GIVEN an open pane on the Queue tab with a fake `clm`
- WHEN the person presses `Drain`
- THEN the pane has run `clm build stop --drain`, then `clm view queue`, and drawn the chart

#### Scenario: stop requested
- GIVEN a queue chart whose loop line is `stop requested: now`
- WHEN the pane draws
- THEN the `Stop now` button reads `Stop now · requested`, dimmed

#### Serves
- capability/watch-beside-chat

#### Needs
- orchestrator.loop/build-stop

## MODIFIED Requirements
### Requirement: view-command-in-chat <!-- base: 35f95b15ba3dcd62 -->
`/clm:view <name>`, for `name` one of `status`, `queue`, `spec`, `flight` and `blockers`, SHALL answer with the output of `clm status` or `clm view <name> --at origin/main`, run at the session's `cwd` with the `clm` the Design names, in a code block shown in the transcript without starting a model turn. When `clm` exits non-zero, it SHALL answer with its stderr instead; with any other name, with the list of names.

#### Scenario: flight in the chat
- GIVEN a session in a CLM repository, and a `clm` whose `view flight --at origin/main` prints a chart
- WHEN the person runs `/clm:view flight`
- THEN the transcript shows that chart in a code block, and no model request is made

#### Scenario: unknown name
- WHEN the person runs `/clm:view everything`
- THEN the answer lists `status`, `queue`, `spec`, `flight` and `blockers`

#### Serves
- capability/see-project-state

#### Needs
- view.repo/view-queue

### Requirement: views-panel <!-- base: 2ea8bd17b7b10a2b -->
`/clm:view` with no argument SHALL open a pane titled `CLM` with tabs Queue, Spec, Flight and Blockers, showing Queue first; a tab kept across a reload that is none of the four SHALL read as Queue. The tabs SHALL show the output of `clm view <tab> --at origin/main`, with `--format ansi --width <the pane's columns>` on the terminal surface, drawn in colour, with its component lines pressable and the Blockers tab's chains folded until pressed (`component-inspector`, `blocker-chains`), and with `--format svg` on other surfaces, except the Queue tab, which has no SVG form and runs `--format text` there. It SHALL redraw when it opens, when the tab changes, when a turn ends, and every 30 seconds while it is placed, and SHALL run `git fetch --quiet origin` at most once every 2 minutes while it is placed. A press SHALL NOT rerun a `clm` command whose output the pane already holds for this generation. Every `clm view` the pane runs SHALL pass `--at origin/main` while `refs/remotes/origin/main` exists, and no `--at` otherwise. When the latest fetch failed, the pane SHALL draw above the chart an amber line `⚠ fetch failed <age> ago:` and the first line of git's stderr, until a fetch succeeds. When `clm` exits non-zero, the pane SHALL show its stderr. The pane SHALL draw no `Image`, offer no `Picture` button and never run `clm view` with `--format png`. Outside a CLM repository, `/clm:view` SHALL say so and open no pane.

#### Scenario: terminal pane
- GIVEN a session on the terminal surface in a CLM repository, with a pane 80 columns wide
- WHEN the person runs `/clm:view`
- THEN the pane runs `clm view queue --at origin/main --format ansi --width 80` and shows its text, coloured as its escapes say

#### Scenario: desktop pane
- GIVEN the same session on the desktop surface
- WHEN the person opens the Spec tab, and then the Queue tab
- THEN the pane runs `clm view spec --at origin/main --format svg` and draws the SVG, and then `clm view queue --at origin/main --format text` and shows its text

#### Scenario: four tabs
- GIVEN a pane whose kept tab, from an earlier version of the panel, is `capabilities`
- WHEN the module is reloaded and the pane draws
- THEN its tab row holds Queue, Spec, Flight and Blockers only, and it shows Queue

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
- THEN it runs `clm view queue` without `--at` and shows its chart

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
- WHEN the pane draws each of the Queue, Spec, Flight and Blockers tabs
- THEN its tab row has no `Picture` button, it draws no `Image`, and none of its `clm` runs passes `--format png`

#### Needs
- view.repo/view-queue

### Requirement: build-rows <!-- base: 2a02f823d4ef5ff9 -->
The Queue tab SHALL show, under each entry of the chart's `running` section, the runs this session launched for that entry: a subagent belongs to the entry `<name>` when one of the first five lines of its prompt is `loop run <name>` (`orchestrator.loop/step-actions`), the first such line deciding. The Flight tab SHALL draw no runs. Each run it draws SHALL show its status (`●` running or waiting, `✗` failed, coloured amber and rose, and `waiting` while waiting), its description, type, model name, effort and round, a bar, its task progress `k/n` and current step when it keeps a task list, its context fill, and the subagents it launched indented under it, oldest first, each with the same facts. Which runs are drawn is `build-row-lifecycle`'s, and their cost `build-row-cost`'s. Drawn runs that belong to no running entry SHALL follow the chart under an `agents` heading, drawn only when a row follows it. On a surface that draws the other tabs' SVG, the rows SHALL follow the Queue tab's text chart, grouped under lines holding the entries' names.

#### Scenario: build under its node
- GIVEN an open Queue pane on the terminal surface whose chart's running entries are `build:git` then `build:init`, and a subagent spawned with description `build git` and a prompt whose first line is `loop run build:git`, which has made two model requests and whose `TodoWrite` lists five tasks, two completed and one in progress, and which has spawned a subagent with description `code review`, whose `parentAgentId` is the build's `agentId`
- WHEN the pane draws
- THEN the lines between `build:git`'s line and `init`'s hold `●` and `build git`, the model's name, a bar, `2/5` and the in-progress task's text, and `ctx`, then `code review`'s row, indented and prefixed `└`, with its own status and stats

#### Scenario: launcher prompt
- GIVEN an open Queue pane on the terminal surface whose chart's running entries are `build:git` then `build:init`, a subagent spawned with the launcher the loop's `launch build:git` named, whose first line is `loop run build:git`, and another whose prompt's first five lines hold no `loop run` line, and whose tenth line is `loop run build:git`
- WHEN the pane draws
- THEN the first's row is between `build:git`'s line and `build:init`'s, and the second's follows the chart under `agents`, not `build:git`'s line

#### Scenario: not a build
- GIVEN a running subagent whose prompt has no `loop run` line
- WHEN the pane draws, and again after its turn completes with reason `answer`
- THEN the first time its row follows the chart under the heading `agents`, under no component line, and the second time neither the row nor the heading is drawn, while the totals line counts it both times

#### Scenario: round
- GIVEN a build of `git` whose turn failed, not dismissed, and a second subagent spawned later with the same description `build git`
- WHEN the pane draws
- THEN the second run's row holds `round 2` and sits above the first's under `build:git`

#### Scenario: desktop
- GIVEN the same build on the desktop surface
- WHEN the pane draws
- THEN the Queue tab's text chart is followed by a line holding `build:git` and then the build's row

#### Needs
- orchestrator.loop/step-actions

### Requirement: build-opens-panel <!-- base: dcdf45bec4f87b83 -->
The hooks module SHALL hook Claude Code's `skill.prompt` event, which fires when a skill's prompt is expanded for the model, whether the person typed its slash command or the model called the Skill tool, and for the `clm` plugin's `build` skill (named `clm:build`, or `build` with `pluginName` `clm`) SHALL return what `next(e)` resolves to, unchanged. When that skill's prompt is expanded, the session's `cwd` is in a CLM repository as `/clm:view` finds one, and `$.ui.panes()` lists no pane whose `id` is the panel's, it SHALL open the pane as `/clm:view` with no argument does: on the Queue tab, its redraw and sampling timers started and a redraw begun. When the pane is listed, open or waiting for room, it SHALL leave it as it is, its tab unchanged; outside a CLM repository and for any other skill it SHALL open nothing. A failure while opening SHALL leave the skill's prompt as `next(e)` resolved it.

#### Scenario: build opens the closed panel
- GIVEN a session in a CLM repository with `clm-panel` loaded and no pane `CLM` listed
- WHEN the `clm:build` skill's prompt is expanded
- THEN the pane `CLM` is opened on the Queue tab, and the prompt the model reads is the one `next(e)` resolved to

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

#### Needs
- view.repo/view-queue

### Requirement: build-row-lifecycle <!-- base: 2de7d406f5697813 -->
Only running, waiting and failed runs SHALL be drawn among the build rows (`build-rows`). A run whose turn ends with an answer SHALL be waiting while the engine lists its agent as `waiting` on background work it owns, and `done` otherwise, the panel checking again at each 30-second redraw; a run whose turn ends with any other reason SHALL be failed; a run that finishes `done` SHALL leave the rows at once; and a failed run SHALL keep its row, with a `Dismiss` button that removes it, until pressed. A listed run whose agent starts a turn again (`turn.start` with its `agentId`) SHALL be running again and drawn, dismissed or not. A drawn run whose parent isn't drawn SHALL take its parent's place. Clearing or resuming the session SHALL empty the list.

#### Scenario: finished and failed builds
- GIVEN an open Queue pane on the terminal surface whose chart's running entries are `build:git` then `build:init`, a build of `git` with a nested subagent `code review` whose turns complete with reason `answer` while `$.agent.list()` lists their agents `completed`, and a build of `init` whose turn completes with a reason other than `answer`
- WHEN the pane draws
- THEN no row holds `build git` or `code review`, `build:init`'s line follows `build:git`'s, the `init` build's row shows `✗` in rose and a `Dismiss` button, and the totals line still counts all three runs' tokens and cost

#### Scenario: dismissed and cleared
- GIVEN a failed build not dismissed
- WHEN the person presses its `Dismiss` and the pane draws, and in a second run a `session.end` with reason `clear` arrives instead and the pane draws
- THEN after the press no row is drawn for it while the totals line still counts it, and after the clear no build row and no totals line is drawn

#### Scenario: failed child of a finished build
- GIVEN a build of `git` that finished `done`, whose nested subagent `code review` failed
- WHEN the pane draws
- THEN `code review`'s row follows `build:git`'s line at the top level, unindented and without `└`

#### Scenario: waiting on its own check
- GIVEN a build of `git` whose turn completes with reason `answer` while `$.agent.list()` lists its agent with status `waiting`
- WHEN the pane draws
- THEN its row is under `build:git`'s line with `●` and `waiting`, and the elapsed time keeps growing each second

#### Scenario: resumed on its own
- GIVEN a build of `git` whose turn completed with reason `answer` while its agent was listed `completed`, so it is drawn no more
- WHEN a `turn.start` with its `agentId` arrives and the pane draws
- THEN its row is under `build:git`'s line again with `●` and no `waiting`, keeping its earlier tokens

#### Scenario: waiting reconciled
- GIVEN a build drawn `waiting`, and another whose turn completes with reason `answer` while its agent is still listed `running`, so it finishes `done`
- WHEN the pane's next 30-second redraw comes, with `$.agent.list()` listing the first's agent `completed` and the second's `waiting`
- THEN no row holds the first, the totals line still counts it, and the second's row is drawn again with `waiting`

### Requirement: component-heading <!-- base: 900ca21551beedb8 -->
Where the pane finds the Flight chart's component lines, for the inspector (`component-inspector`), it SHALL take the lines between the heading `components` and the heading `changes`, and, when the chart has no `components` heading, between the old heading and `changes`: the old word for components (`component-words`), which a `clm` built before `view/flight-component-lines` gave its new heading still prints, so the pane works with the `clm` the session runs, older or newer (`component-words`). Labels the pane draws itself SHALL hold no old word `component-words` doesn't keep.

#### Scenario: the new heading
- GIVEN a Flight chart whose lines are a `components` heading, `  a …` and `  b …`, then `changes` and `  none`, and the `flight` model listing `a` and `b`
- WHEN the pane draws the Flight tab
- THEN `a`'s and `b`'s lines are pressable, matched to `a` and `b`

#### Scenario: an older clm
- GIVEN the same chart with the old heading in place of `components`
- WHEN the pane draws the Flight tab
- THEN `a`'s and `b`'s lines are pressable, matched to `a` and `b`

#### Serves
- architecture/plain-terms
