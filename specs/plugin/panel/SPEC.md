---
title: Panel
status: active
owners: ["@clunny"]
depends_on: [view, effort]
paths: ["plugin-panel/**"]
successor: ""
serves: [capability/watch-beside-chat, capability/see-project-state]
---

## Contract
The Claude Code plugin `clm-panel`, listed in this repo's marketplace after `clm`: the `/clm:view` command, which answers in the chat with `clm status` or `clm view <name>`, and with no argument opens a pane titled `CLM` with four tabs: Spec, Flight and Blockers, showing those views of `origin/main` as of the pane's last fetch (`views-rendered-by-clm`), and Capabilities, showing `clm capabilities` and a capability's page, read-only. It displays what `clm` prints and decides nothing about the repository. Under the Flight chart's component lines it adds the one kind of fact `clm` can't see: the builds its own session launched that are running or failed, with their nested subagents, status, steps, context, tokens, estimated cost and time.

## Design
**The CLM panel** (`views-rendered-by-clm`). The panel is its own plugin, `clm-panel`, in `plugin-panel/`: `.claude-plugin/plugin.json`, and `hooks/hooks.json` naming one Claude Code hooks module, `./register.tsx`, under `modules`. It only displays what `clm` prints; what a view holds is `view`'s. Keeping it out of `clm` means `clm`'s command hooks never share a plugin with a hooks module, and a person on a Claude Code too old for hooks modules simply doesn't install it. It needs `clm`: its `plugin.json` lists `clm` under `dependencies`, and it runs the `clm` plugin's `bin/clm` shim, so it gets the same binary and `version-skew` handling as the Bash tool, or `clm` on `PATH` when it can't find the shim. Its rules (which command to run for a surface and width, ANSI escapes to coloured cells, when to redraw and when to fetch, what to answer for each `/clm:view` argument, and the build rows below) are plain functions in `plugin-panel/src/`, taking a fake of the few `$` calls and events they use, and tested with `vitest` under `plugin-panel/package.json`'s `npm test`, which CI runs (`ci`). `register.tsx` only wires them to `$`. The `/clm:view` command is the `clm` plugin's, not this module's: `$.command.register` takes a bare name run as `/<name>`, so a hooks module can't register a `clm:`-namespaced command, and `clm-panel`'s own would be `clm-panel:`. `plugin/commands/view.md` makes `/clm:view` exist (`plugin`), and the module hooks `command.run` for `clm:view` and answers it without calling `next`, so no model turn starts. With no argument the pane opens on Flight; the first three tabs are views, and Capabilities is described under its own heading below. The view tabs run `clm view <tab> --at origin/main` through `$.process.run` with the plugin's `bin/clm`, at the session's `cwd`, so they always describe what has landed, whichever branch or worktree the session is on. Whether `refs/remotes/origin/main` exists is checked with `git rev-parse --verify -q` after each fetch, so a repository with no remote or never fetched shows its checkout; the chart's first line names that commit, when it was fetched and the session's branch (`view`'s `view-at-rev`). The terminal surface takes `--format ansi --width <the pane's columns>` and turns its escapes into coloured text cells; the others take `--format svg`, drawn with `Svg`. `clm view` itself never fetches. The fetch-failed line's `<age>` is since that fetch started, as `<n>m` under an hour and `<n>h` from then on, as `view-at-rev` writes it; `fetchIfDue` keeps the latest fetch's start, exit status and stderr for it. When `clm` isn't installed the pane shows the shim's message instead of a chart. Outside a CLM repository means no `specs/SPEC.md` found as `session-start` finds it. Without `clm-panel`, the command file has the model run `clm status` or `clm view <name>` through the Bash tool, and both work in a terminal. So the panel's answer that `clm` isn't installed (`panel-package`) arises only with the `clm` plugin installed but its shim not found and no `clm` on `PATH`.

**Build rows** (`views-rendered-by-clm`, `builds-in-subagents`). Every build is a background subagent of the human's session, so the panel sees it through the hooks events, and the Flight tab shows it under the component it builds. The rules are pure functions in `plugin-panel/src/agents.ts`, over a list of runs kept in an atom, with the time in another.
- **Runs.** The `agent.spawn` hook, after `next`, records a run: `id` the started `agentId`, `parentId` the event's `parentAgentId`, `type` its `subagentType`, `description`, `model` as started, `startedAt`. A run is a **build of `<component>`** when one of the first five lines of its prompt names `git switch impl/<component>`, the first such line deciding: `clm run` prints a launcher of at most five lines that names the prompt file and `git switch impl/<component>` as its first step (`orchestrator.run/build-prompt-file`), on its second line, and a whole build prompt names it on its first line; the model's `description` never decides. Five lines, not the whole prompt, so a subagent whose long prompt quotes a build's first step (a reviewer's diff, a builder check's change) isn't taken for a build; the launcher's lines aren't otherwise read, so its wording can change without the panel. A run whose normalised description a listed run already has is round `n+1` (a build re-launched after it stopped, or a reviewer's fix round). The list keeps the last 200 runs. `/clear` and resume (`session.end` with reason `clear` or `resume`) empty it; quitting doesn't, and after a reload the atom holds it, so `session.start` reconciles it with `$.agent.list()`: a run that is `running` or `waiting` whose agent the engine doesn't list is dropped, since nothing will finish it, and the waiting reconciliation below runs over the rest.
- **Updates.** `turn.step` with an `agentId` is one model request of that run: its context is that request's tokens (input, output, cache read and cache creation), its tokens and cost accumulate, its model and effort follow the request, and its step count grows. `turn.complete` with an `agentId` ends the run's turn: `failed` when the reason isn't `answer`; on an answer, after `next` resolves, the hook asks `$.agent.list()`, and the run is `waiting` when the engine lists its agent with status `waiting` (`AgentStatus`: held on background work it owns, such as a `clm land --check-only` it ran in the background), with no `endedAt`, so its elapsed time keeps growing, and `done` at that time otherwise; a run that saw no step takes the turn's own usage either way. The pure `onComplete` takes the agent's listed status as an argument; `register.tsx` asks for it. `turn.start` with an `agentId` a listed run has is that agent's loop starting a turn again (the engine resuming it when its background work ends, or a message waking it): the run is `running` again, its `endedAt` and `dismissed` cleared, keeping its stats, as a resuming `agent.spawn` does, whatever its status was. A `turn.step` of a `waiting` or `done` run does the same, in case the engine continues a loop without a new turn. **Waiting reconciliation:** the engine's status may lag the hook, so in the pane's 30-second timer, before its redraw, and at `session.start`, the runs are reconciled with `$.agent.list()` (never in a draw; when the call fails the runs stay as they were): a `done` run whose agent is listed `waiting` is `waiting` again, and a `waiting` run whose agent is listed with any status but `waiting` or `running`, or not listed, is `done`, ended at that time. A run's status is `running`, `waiting`, `done` or `failed`. `tool.call` with an `agentId` feeds that run's task list: `TodoWrite` replaces it, `TaskCreate` appends a pending task, `TaskUpdate` changes a task's status or subject or deletes it; `k` is the completed tasks, `n` the total, and the current step is the first `in_progress` task, else the first `pending`.
- **Cost.** An estimate: a table in `agents.ts` of USD per million tokens, input, output, cache read and cache write, keyed by a regular expression on the model ID, most specific first, with a fallback rate for a model it doesn't know; a context window per family (200k for Haiku, 1M otherwise) gives the fill percentage. The rows say `≈` before every cost, and the table carries the date it was last checked against the list prices, as `effort`'s does. It holds the same rows, order and fallback as `effort/prices` (`crates/clm-effort/src/prices.rs`), so the pane's estimate and `clm capabilities`' agree; changing a price changes both tables in one change.
- **Drawing.** On the terminal surface the pane takes the `clm view flight --format ansi` output as lines of styled runs (`ansiLines`), and, between the `components` heading and the `changes` heading, after each line whose text starts with two spaces and a component ID (`view/flight-component-lines`: the full ID, or as much as fits followed by `…`), inserts the rows of that component's top-level builds, newest first, each followed by its subtree depth-first, children oldest first, indented two cells per level up to four levels, a child's first line prefixed `└`. A top-level run is one whose parent isn't listed. A row is three lines: the status glyph and the description (`●` amber while running or waiting, `✗` rose when failed, `colour-meanings`), and, while waiting, ` · waiting` dimmed after the description, and on a failed run's first line, last, a `Dismiss` button (`role` `dismiss`, `key` `dismiss:<id>`); the type, model name (`Fable 5.1`, `Opus 5.5`, from the model ID), effort and `round n` when above 1, dimmed; and a bar of ten cells with its stats, dimmed: the bar is the run's task progress `k/n` in amber when it keeps a task list, else its context fill dimmed; then `k/n <current step> · ` when there is a list, `ctx <p>% · <tokens> tok ≈$<cost> <elapsed>`, tokens as `850`, `85k` or `1.3M`, cost to cents under $10 and tenths above, elapsed as `m:ss` or `h:mm:ss`. `Dismiss` marks the run dismissed in the runs atom; a run that isn't drawn stays in the list, for rounds and the totals. A drawn run whose parent isn't drawn takes the parent's place in the tree: under the nearest drawn ancestor, or as a top-level row, at depth 0 of the rows' indent and under its top-level ancestor's component line, when there is none. After the chart, when any run belongs to no component line (a drawn subagent that isn't a build, such as `propose`'s builder check, or a build of a component the chart doesn't list), an `agents` heading in indigo and those rows follow; then, when the list isn't empty, a totals line over every run of the list, drawn or not: `≈$<cost> · <tokens> tokens · <time> · <model> ×<n>` per model, most used first, the time spanning the first start to the last end. On the other surfaces the pane has only the SVG and can't tell which components it lists, so the same rows follow the `Svg` with every build under a line holding its component's ID, components in the order of their first build, then the `agents` heading with the runs that aren't builds, and the totals.
- **Ticking.** While any run is `running` or `waiting`, a one-second `$.clock.every` sets the time atom, which redraws the pane; the pane's `clm view` output is cached per generation, so a tick reruns nothing, and `clm` runs only on the redraw cadence of `views-panel`.

**Inspector and blocker chains** (`views-rendered-by-clm`). On the terminal surface the charts can be pressed into, as the web app's graph can: a component's line on the Spec or Flight tab selects the component and opens an inspector over the chart, and a root's line on the Blockers tab unfolds the chain it holds up. The panel draws nothing of its own about the repository: every fact comes from `clm view <name> --format json`, and every line from `clm view <name> --format ansi`. The rules are pure functions in `plugin-panel/src/inspector.ts`, over the ANSI lines `ansiLines` gives, the parsed JSON models and three atoms, tested with `vitest` as the rest are; `register.tsx` only turns what they return into elements.
- **Which line is which.** `view` pins the lines the pane presses (`view/spec-component-lines`, `view/flight-component-lines`, `view/blockers-root-lines`): on the Spec tab every line after the checkout line is a component's (the chart begins with a header line exactly when the JSON's top-level `checkout` or `at` is present, so the pane skips one line then), in the `spec` model's order; on Flight, the lines between `components` and `changes` that start with two spaces and aren't `  none`, in the `flight` model's order; on Blockers, the lines after `blockers` that start in the first column and aren't `nothing is blocked`, in the `blockers` model's order, each followed by its tree's lines. The pane reads which component or root a line is by position, the nth such line being the nth entry of the model's `/nodes` or `/roots`, so a cut ID (`…`) matches. The JSON runs beside the chart, at the same `cwd` with the same `clm`, and is cached per generation as the chart is (`viewKey` takes the format), so a press or a tick reruns nothing. When the count of such lines differs from the model's (the two runs straddled a change to the repository), or the JSON doesn't parse, or its run exits non-zero, the tab falls back as `component-inspector` and `blocker-chains` say; the next generation tries again. Such a draw leaves `selected` and `unfolded` as they were. Colours follow `chart.rs`'s ANSI table, as `ansiLines` reads it: green, sky as `cyan`, amber as `yellow`, rose as `red` and indigo as `blue`.
- **Pressable lines.** Each such line is drawn as a `Button` with `plain`, the line's styled runs as its `Text` children, and `key` `node:<id>` or `root:<name>`, so the surface finds it again after a redraw; the person presses it with a click, or moves the focus ring onto it and presses Enter. On the Flight tab the build rows follow the component's line. A press changes an atom and nothing else, so it redraws without running `clm` again except for a JSON model this generation hasn't fetched yet.
- **Selection.** The atom `selected`, a component ID or `null`, is `null` at first and kept like `tab` across a reload, under its own `shape`, and declared in `types/index.d.ts` beside it. It is kept but not drawn on Blockers, and a selected component the `spec` model doesn't list draws no inspector and draws one again if it comes back; no draw writes an atom, since only presses and the hooks events do. The selected component's line is drawn `bold`.
- **The inspector.** The Spec and Flight tabs draw it between the tab row and the chart, from the `spec` and `flight` models (each tab fetches the other's JSON only while a component is selected), as lines of styled runs in `colour-meanings`: first the ID in indigo, bold, and the title, then a `Close` button (`role` `dismiss`); then `<status>`, `owners <owners>` and `depends on <ids>` (or `depends on nothing`), joined by ` · `; then `requirements <current>/<total> current`, followed by `, <n> behind`, `, <n> unbuilt` and `, <n> obsolete` for each non-zero count and `  ⚠ oversized` in amber when the model says so; then the work in flight. A component the `flight` model doesn't list reads `converged`, in green, and nothing more. One it lists reads its state, coloured as the chart colours it, and its priority, then `pushed <h>h ago, lapses in <h>h` when claimed, and `blocked by <blocker> (<requirements>)` for each blocker when blocked, then one line per pending requirement, indented two cells: its ID, its state (sky `unbuilt`, amber `behind`, rose `obsolete`), and, for an `obsolete` one, each `<file>:<line>` of its markers. When the other tab's model fails, the inspector keeps the selected component's ID, its `Close` button and what the tab's own model gives, and holds that run's stderr in place of the other model's part: on the Spec tab the work in flight, on the Flight tab the title, status, owners, dependencies, counts, changes, backlog and contract.
- **Changes, backlog and the contract** (`inspector-detail`, `panel-inspector-contract`). After the work in flight, the inspector draws what the `spec` model's component carries beyond its counts (`view/component-detail`), so the panel reads nothing under `specs/`, `changes/` or `backlog/` itself (`views-rendered-by-clm`). First `changes`, in indigo, then one line per entry, indented two cells, in the model's order: the change ID, its status, and `<current>/<total> requirements`, with `complete` in green after it when the entry says so; with no entry, the one line `changes none`, the heading in indigo and ` none` plain. Then `backlog` the same way: one line per item, indented two cells, `backlog/<slug>`, its kind dimmed and its title; or `backlog none`. Then a `Contract` button. The atom `contract` is a boolean, `false` at first and kept across a reload like `selected`, under its own `shape` in `types/index.d.ts`. While it is `true`, the contract's text follows the button as plain lines: each line of the text longer than the pane's columns is wrapped at spaces, a word longer than the width cut across lines, with one character to a cell as `lineText` and `--width` assume; blank lines within the text are kept; and nothing is rendered as markdown, since the terminal shows text and the panel draws no view of its own. The rules return this detail as a part of its own beside the inspector's lines (the `changes` and `backlog` lines, the button's variant, and the contract's lines when shown), so the lines `component-inspector` pins keep their positions and `register.tsx` places the `Button` after them; `ChartInput` carries the pane's columns for the wrapping, 100 when `bodyColumns` isn't a positive number, as `viewArgs` falls back. It is hidden at first because a contract runs to paragraphs and would push the chart the person just pressed off the pane. The spec branches on `origin` that touch the component, which the web's inspector lists as "In open PRs", stay out: no model carries them (`view`'s Design says why).
- **Blocker chains.** The atom `unfolded`, a list of root names, empty at first and kept across a reload under its own `shape`, holds the roots whose chains show. A folded root's line alone is drawn, whose `holds up <n>` says how much is beneath it. A name the `blockers` model doesn't list is ignored, and its root unfolded again if it comes back; as for `selected`, no draw writes the atom.
- **Other surfaces.** The `Svg` takes no presses, so the desktop and web surfaces have no pressable line, no inspector, and show every chain.

**Capabilities** (`capabilities-tab`, `review-capability-cost`). The fourth tab, after Blockers, shows what each capability costs and lets the person read one capability's page; it decides nothing. It runs `clm capabilities` and `clm capabilities --format json` through the same `clm` and cache as the views (cached per generation), and draws `clm`'s text as lines on every surface: `clm capabilities` prints text, not a chart, so the tab takes no `--width` and draws no SVG. Its rows are matched to slugs by position, as the inspector matches component lines: a row is a line before the one starting `unattributed` that starts with `⚑ ` or two spaces (the header never does), and the nth row is the JSON's nth `capabilities` entry, whose slug must also be the row's first word, since the list prints slugs whole and two runs that straddled a rename can agree on the count alone. A row whose slug is `decide` or `help` isn't pressable: `clm capabilities` reads those words as its subcommands, so no page is asked for by them. Each row is drawn as a `plain` `Button` with the key `capability:<slug>`; pressing it sets the atom `capability`, a slug or `null`, `null` at first, kept across a reload under its own `shape` in `types/index.d.ts`. While it holds a slug the tab runs `clm capabilities <slug>` instead, started together with the list's runs. The lines wrap at the pane's edge rather than being cut, since `clm capabilities` takes no `--width` and a page's prose runs past it. `/clm:view` opens the tab on the list, as it opens the pane on Flight. A slug kept across a reload that names a capability since removed makes `clm` exit non-zero, and the stderr shows under `Back`, so it can be left. The page's last line, `decide: clm capabilities decide ...`, stays text: the panel has no control that runs `clm capabilities decide`, which the guard asks the human about (`plugin.hooks.guard/approve-guard`), and the person decides in the chat or on the web. The rules are pure functions in `plugin-panel/src/capabilities.ts`; `register.tsx` only wires them.

**Type-check** (`type-check`, `panel-types-hand-written`). `npm test` runs `tsc -p .` before `vitest run`, so a type error in `hooks/`, `src/`, `types/` or `test/` fails the suite, and the land, before a test runs: the tests strip types, and `claude plugin validate` checks only what the engine would refuse to load. `tsconfig.json` types the module as the engine does (`target` and `lib` `es2023`, `module` `esnext`, `moduleResolution` `bundler`, `strict`, `noUncheckedIndexedAccess`, `noEmit`, `skipLibCheck`, `jsx` `react` with `h` and `Fragment`) plus `allowImportingTsExtensions`, since the module imports its files by their `.ts` names. `declarations/claude-code/index.d.ts` is the panel's own, written by hand: the parts of the plugin API the panel uses and nothing else. It is never a copy of the declarations Claude Code writes, which no package ships and which aren't this repository's to redistribute. When the panel starts using something the file lacks, the same change adds what it uses, reading the `plugin-authoring` skill's reference for the shape. When `tsc` and `claude plugin validate` disagree about the module, the file is what's wrong, since the validator reads the module as the engine does. `.claude-plugin/types/`, where the engine lays its own when it hot-reloads the panel from a checkout, stays ignored. `typescript` is a devDependency the lockfile pins, so `npm ci` brings `tsc` and the check needs nothing else on the machine; it runs inside `npm test` rather than as `npx tsc`, which unattended sessions may not run (`unattended-commands`), and a human typing `npm test` here gets the same check as a land. The tests type under the same options as the module, with `vitest`'s own types from `node_modules`, so they use only what the module's environment has (no `Buffer`: the engine has no Node.js), and a fake they pass is the type the rule takes. `$.ui.resolve(e)` gives each surface's element table as a union, so an element some surfaces lack (`Svg`) is read by narrowing the table, not destructured from it. The tests that prove `type-check` read the panel's files and type the erring file through the `typescript` package's own API (`ts.sys.readFile`; `createProgram` with a host that serves one in-memory file), which its own declarations type without Node.js's; that the panel itself type-checks is proven by `npm test`, whose first command it is, at every land. `claude plugin validate` passes with `declarations/` and `tsconfig.json` present: it reads the manifest, the module and the contract, nothing else.

## Requirements

### Requirement: views-panel
`/clm:view` with no argument SHALL open a pane titled `CLM` with tabs Spec, Flight, Blockers and Capabilities, showing Flight first. The Spec, Flight and Blockers tabs SHALL show the output of `clm view <tab> --at origin/main`, with `--format ansi --width <the pane's columns>` on the terminal surface, drawn in colour, with its component lines pressable and the Blockers tab's chains folded until pressed (`component-inspector`, `blocker-chains`), and with `--format svg` on other surfaces. It SHALL redraw when it opens, when the tab changes, when a turn ends, and every 30 seconds while it is placed, and SHALL run `git fetch --quiet origin` at most once every 2 minutes while it is placed. Every `clm view` the pane runs SHALL pass `--at origin/main` while `refs/remotes/origin/main` exists, and no `--at` otherwise. When the latest fetch failed, the pane SHALL draw above the chart an amber line `⚠ fetch failed <age> ago:` and the first line of git's stderr, until a fetch succeeds. When `clm` exits non-zero, the pane SHALL show its stderr. The pane SHALL draw no `Image`, offer no `Picture` button and never run `clm view` with `--format png`. Outside a CLM repository, `/clm:view` SHALL say so and open no pane.

#### Scenario: terminal pane
- GIVEN a session on the terminal surface in a CLM repository, with a pane 80 columns wide
- WHEN the person runs `/clm:view`
- THEN the pane runs `clm view flight --at origin/main --format ansi --width 80` and shows its text, coloured as its escapes say

#### Scenario: desktop pane
- GIVEN the same session on the desktop surface
- WHEN the person opens the Spec tab
- THEN the pane runs `clm view spec --at origin/main --format svg` and draws the SVG

#### Scenario: refresh and fetch
- GIVEN an open pane
- WHEN a turn ends, and again 30 seconds later, and three turns end within a minute
- THEN it reruns `clm view` each time, and runs `git fetch --quiet origin` no more than once in those 2 minutes

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

### Requirement: panel-package
`.claude-plugin/marketplace.json` SHALL list, after `clm`, a plugin `clm-panel` with source `./plugin-panel`. `plugin-panel/.claude-plugin/plugin.json` SHALL have a `version` equal to the cargo workspace version, as `plugin/.claude-plugin/plugin.json`'s is (`plugin/package`), so each release changes it and `claude plugin update` delivers the panel's fixes, and its cache directory names the same version as the `clm` plugin whose shim it prefers. `plugin-panel/hooks/hooks.json` SHALL name exactly one hooks module, `./register.tsx`, under `modules`, and no command hooks, and `plugin/hooks/hooks.json` SHALL name no hooks module. `plugin-panel/package.json` SHALL have a `test` script that runs the panel's tests, with `package-lock.json` committed and `.npmrc` setting `ignore-scripts=true`, as `web/` does (`unattended-commands`).

#### Scenario: two plugins
- GIVEN the repository
- WHEN a test reads the marketplace and both `hooks.json` files
- THEN the marketplace lists `clm` then `clm-panel`, `clm-panel`'s `hooks.json` holds only `modules: ["./register.tsx"]`, and `clm`'s holds no `modules`

#### Scenario: panel version
- GIVEN a commit that changes the workspace version but not `plugin-panel/.claude-plugin/plugin.json`
- WHEN `cargo test` runs
- THEN a test fails naming both versions

#### Scenario: no clm
- GIVEN the panel loaded with neither the `clm` plugin's shim nor `clm` on `PATH`
- WHEN the person runs `/clm:view flight`
- THEN it answers that `clm` isn't installed, naming the `clm` plugin

#### Serves
- capability/install-and-release
- capability/watch-beside-chat

#### Needs
- ci/release

### Requirement: view-command-in-chat
`/clm:view <name>`, for `name` one of `status`, `spec`, `flight` and `blockers`, SHALL answer with the output of `clm status` or `clm view <name> --at origin/main`, run at the session's `cwd` with the `clm` the Design names, in a code block shown in the transcript without starting a model turn. When `clm` exits non-zero, it SHALL answer with its stderr instead; with any other name, with the list of names.

#### Scenario: flight in the chat
- GIVEN a session in a CLM repository, and a `clm` whose `view flight --at origin/main` prints a chart
- WHEN the person runs `/clm:view flight`
- THEN the transcript shows that chart in a code block, and no model request is made

#### Scenario: unknown name
- WHEN the person runs `/clm:view everything`
- THEN the answer lists `status`, `spec`, `flight` and `blockers`

#### Serves
- capability/see-project-state

### Requirement: build-rows
The Flight tab SHALL show, under the chart line of each component, the runs this session launched that are builds of that component: a subagent is a build of `<component>` when one of the first five lines of its prompt names `git switch impl/<component>`, the first such line deciding. Each run it draws SHALL show its status (`●` running or waiting, `✗` failed, coloured amber and rose, and `waiting` while waiting), its description, type, model name, effort and round, a bar, its task progress `k/n` and current step when it keeps a task list, its context fill, tokens, estimated cost marked `≈` and elapsed time, and the subagents it launched indented under it, oldest first, each with the same facts. Drawn runs that belong to no component line SHALL follow the chart under an `agents` heading, drawn only when a row follows it, then a totals line of cost, tokens, time and runs per model over every run of the session, drawn or not. Only running, waiting and failed runs SHALL be drawn. A run whose turn ends with an answer SHALL be waiting while the engine lists its agent as `waiting` on background work it owns, and `done` otherwise, the panel checking again at each 30-second redraw; a run that finishes `done` SHALL leave the rows at once; and a failed run SHALL keep its row, with a `Dismiss` button that removes it, until pressed. A listed run whose agent starts a turn again (`turn.start` with its `agentId`) SHALL be running again and drawn, dismissed or not. A drawn run whose parent isn't drawn SHALL take its parent's place. Clearing or resuming the session SHALL empty the list. While a run is running or waiting the pane SHALL redraw every second without running `clm` again. On a surface that draws the SVG, the rows SHALL follow it, grouped under lines holding the component IDs. `/clm:view flight` in the chat SHALL answer with `clm`'s output alone.

#### Scenario: build under its node
- GIVEN an open Flight pane on the terminal surface whose chart lists components `git` then `init`, and a subagent spawned with description `build git` and a prompt whose first line is `git switch impl/git`, which has made two model requests and whose `TodoWrite` lists five tasks, two completed and one in progress
- WHEN the pane draws
- THEN the lines between `git`'s line and `init`'s hold `●` and `build git`, the model's name, a bar, `2/5` and the in-progress task's text, `ctx`, `tok`, `≈$` and an elapsed time

#### Scenario: nested subagent
- GIVEN that build has spawned a subagent with description `code review`, whose `parentAgentId` is the build's `agentId`
- WHEN the pane draws
- THEN `code review`'s row follows the build's, indented and prefixed `└`, with its own status and stats, and `init`'s line follows it

#### Scenario: finished build
- GIVEN the build's turn completes with reason `answer` while `$.agent.list()` lists its agent `completed`
- WHEN the pane draws
- THEN no row holds `build git` or `code review`, `init`'s line follows `git`'s, and the totals line still counts both runs' tokens and cost

#### Scenario: failed build dismissed
- GIVEN a build whose turn completes with a reason other than `answer`
- WHEN the pane draws, then the person presses its `Dismiss`, then the pane draws again
- THEN the first drawing shows its row with `✗` in rose and a `Dismiss` button, and the second shows no row for it while the totals line still counts it

#### Scenario: failed child of a finished build
- GIVEN a build of `git` that finished `done`, whose nested subagent `code review` failed
- WHEN the pane draws
- THEN `code review`'s row follows `git`'s line at the top level, unindented and without `└`

#### Scenario: cleared
- GIVEN a failed build not dismissed
- WHEN a `session.end` with reason `clear` arrives and the pane draws
- THEN no build row and no totals line is drawn

#### Scenario: not a build
- GIVEN a running subagent whose prompt has no `git switch impl/` line
- WHEN the pane draws, and again after its turn completes with reason `answer`
- THEN the first time its row follows the chart under the heading `agents`, under no component line, and the second time neither the row nor the heading is drawn, while the totals line counts it both times

#### Scenario: round
- GIVEN a build of `git` whose turn failed, not dismissed, and a second subagent spawned later with the same description `build git`
- WHEN the pane draws
- THEN the second run's row holds `round 2` and sits above the first's under `git`

#### Scenario: ticking without clm
- GIVEN a running build and a pane that has run `clm view flight` once this generation
- WHEN a second passes
- THEN the pane redraws with a larger elapsed time, and `clm` has not run again

#### Scenario: cost estimate
- GIVEN a run whose one model request used a million input tokens on a model the price table knows
- WHEN the pane draws
- THEN its row holds `≈$` followed by that model's input rate, and `1.0M tok`

#### Scenario: desktop
- GIVEN the same build on the desktop surface
- WHEN the pane draws
- THEN the SVG is followed by a line holding `git` and then the build's row

#### Scenario: chat unchanged
- GIVEN a running build
- WHEN the person runs `/clm:view flight`
- THEN the answer is `clm view flight`'s output in a code block, with no build row

#### Scenario: launcher prompt
- GIVEN an open Flight pane on the terminal surface whose chart lists components `git` then `init`, and a subagent spawned with the launcher `clm run git` prints, whose first line says to read `.clm/prompts/git.md` and whose second line names `git switch impl/git` as its first step
- WHEN the pane draws
- THEN that subagent's row is between `git`'s line and `init`'s, and no `agents` heading is drawn

#### Scenario: quoted first step
- GIVEN a subagent whose prompt's first five lines name no `git switch impl/`, and whose tenth line is `git switch impl/git`
- WHEN the pane draws
- THEN its row follows the chart under `agents`, not `git`'s line

#### Scenario: waiting on its own check
- GIVEN a build of `git` whose turn completes with reason `answer` while `$.agent.list()` lists its agent with status `waiting`
- WHEN the pane draws
- THEN its row is under `git`'s line with `●` and `waiting`, and the elapsed time keeps growing each second

#### Scenario: resumed on its own
- GIVEN a build of `git` whose turn completed with reason `answer` while its agent was listed `completed`, so it is drawn no more
- WHEN a `turn.start` with its `agentId` arrives and the pane draws
- THEN its row is under `git`'s line again with `●` and no `waiting`, keeping its earlier tokens

#### Scenario: waiting ends
- GIVEN a build drawn `waiting`
- WHEN `$.agent.list()` lists its agent `completed` and the pane's 30-second redraw comes
- THEN no row holds it, and the totals line still counts it

#### Scenario: status lags the answer
- GIVEN a build whose turn completes with reason `answer` while its agent is still listed `running`, so it finishes `done`
- WHEN its agent is listed `waiting` at the next 30-second redraw
- THEN its row is drawn again with `waiting`

### Requirement: panel-windows
The panel SHALL read a `$.plugin.root` or session `cwd` that starts with a drive letter and a separator (`C:\` or `C:/`) as a Windows path: it SHALL turn each `\` in it into `/` before it looks for the CLM repository (`views-panel`) or the shim, and stop at the drive's root, `C:/`, which it looks in too; the `cwd` it runs commands at stays as given. With such a `$.plugin.root`, when it has found the `clm` plugin's `bin/clm`, it SHALL run that `sh` script through Git Bash, as `[<bash>, <shim>, ...args]`, where `<bash>` is `bin/bash.exe` under the directory three levels above the one `git --exec-path` prints, trimmed and with `\` read as `/` (`C:/Program Files/Git/mingw64/libexec/git-core` gives `C:/Program Files/Git/bin/bash.exe`). It SHALL run `git --exec-path` only then, and at most once while the module is loaded, keeping its answer, a failure included, until the module reloads. When that command can't start, exits non-zero, prints a path with fewer than three levels, or `<bash>` isn't a file, it SHALL run `clm` from PATH, as when it finds no shim (`panel-package`). It SHALL never run a bare `bash` or `sh`, which on Windows may start WSL's. With any other `$.plugin.root` it SHALL run the shim directly, as now, and not run `git --exec-path`.

#### Scenario: shim through Git Bash
- GIVEN a `$.plugin.root` of `C:\Users\u\.claude\plugins\cache\clm\clm-panel\0.7.0`, a directory `C:/Users/u/.claude/plugins/cache/clm/clm/0.7.0` holding a file `bin/clm`, a `git --exec-path` that prints `C:/Program Files/Git/mingw64/libexec/git-core`, and a file `C:/Program Files/Git/bin/bash.exe`
- WHEN the pane runs `clm view flight` twice
- THEN both runs' argv is `C:/Program Files/Git/bin/bash.exe`, `C:/Users/u/.claude/plugins/cache/clm/clm/0.7.0/bin/clm`, then the `clm view` arguments, and `git --exec-path` ran once

#### Scenario: no Git Bash
- GIVEN the same root and shim, and, in turn, a `git --exec-path` that exits 1, one that can't start, and one that prints the path above with no `C:/Program Files/Git/bin/bash.exe` file
- WHEN the pane runs `clm view flight` twice in each
- THEN each run's argv is `clm` then the `clm view` arguments, no argv starts with `bash` or `sh`, and `git --exec-path` ran once in each

#### Scenario: repository on a drive
- GIVEN a session `cwd` of `C:\work\repo\crates` and a file `C:/work/repo/specs/SPEC.md`, with the plugin root of the scenarios above
- WHEN the person runs `/clm:view flight`
- THEN it answers with the view, not that the directory isn't a CLM repository

#### Scenario: not on Windows
- GIVEN a `$.plugin.root` of `/plugins/clm/plugin-panel` and a file `/plugins/clm/plugin/bin/clm`
- WHEN the pane runs `clm view flight`
- THEN its argv starts with `/plugins/clm/plugin/bin/clm`, and `git --exec-path` never runs

#### Serves
- capability/watch-beside-chat
- capability/install-and-release

### Requirement: blocker-chains
On the terminal surface the Blockers tab SHALL draw each root's line (`view/blockers-root-lines`) as a pressable line holding the same text and colours, matched to the root by its position among those lines and the `blockers` model's `roots`, from `clm view blockers --format json` run as the chart is, and SHALL draw the lines of a root's tree only while that root is unfolded. Every root SHALL start folded. Pressing a root's line SHALL fold or unfold that root alone, without rerunning a `clm` command whose output the pane already holds for this generation. Which roots are unfolded SHALL be kept, by root name, across redraws and a reload, and a root the model doesn't list SHALL be drawn as nothing and keep its state for when it comes back. When the lines can't be matched (their count differs from the model's, or the JSON run fails or doesn't parse), the tab SHALL draw every line as before, unfolded and not pressable. Other surfaces SHALL draw every chain as before.

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
- THEN `gate`'s two tree lines follow its line, `host`'s tree line is not drawn, and no `clm` command has run again

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
- THEN it draws `clm view blockers --format svg`'s SVG, as before

### Requirement: type-check
`plugin-panel/package.json`'s `test` script SHALL run `tsc -p .` before `vitest run`, with `typescript` among its devDependencies, so a type error in a file `plugin-panel/tsconfig.json` includes fails `npm test` before any test runs. `plugin-panel/tsconfig.json` SHALL include `hooks`, `src`, `types` and `test`, type them as the engine does (the options this component's Design lists, plus `allowImportingTsExtensions`), and name `declarations` as its one type root and `claude-code` as its one type, so `'claude-code'` resolves to `plugin-panel/declarations/claude-code/index.d.ts`: the panel's own declarations of the parts of the plugin API it uses, written by hand, whose first line says so (`panel-types-hand-written`). `plugin-panel/vendor/` SHALL NOT exist. The panel and its tests SHALL type-check under that configuration.

#### Scenario: a type error fails the suite
- GIVEN a file that uses a type it never imports
- WHEN the compiler types it with `plugin-panel/tsconfig.json`'s options and type root
- THEN it reports an error naming the file and its line

#### Scenario: the suite type-checks first
- GIVEN the repository
- WHEN a test reads `plugin-panel/package.json` and `plugin-panel/tsconfig.json`
- THEN the `test` script runs `tsc -p .` before `vitest run`, `typescript` is a devDependency, and the tsconfig includes `hooks`, `src`, `types` and `test` with `declarations` its one type root and `claude-code` its one type

#### Scenario: the declarations are the panel's own
- GIVEN the repository
- WHEN a test reads the first line of `plugin-panel/declarations/claude-code/index.d.ts` and looks for `plugin-panel/vendor/`
- THEN the line is `// The parts of Claude Code's plugin API this panel uses, written by hand.`, and `plugin-panel/vendor/` doesn't exist

#### Serves
- architecture/deterministic-enforcement

### Requirement: inspector-detail
While a component is selected (`component-inspector`), the inspector SHALL hold, after the work in flight and from the `spec` model's component (`view/component-detail`): under `changes` one line per entry of its `changes` with the change's ID, status and `<current>/<total> requirements`, and `complete` when the entry says so, or `changes none`; under `backlog` one line per item of its `backlog` with `backlog/<slug>`, its kind and its title, or `backlog none`; and a `Contract` button, `secondary` while the contract is hidden and `primary` while it shows, which SHALL show or hide the component's `contract` text below it as plain lines no wider than the pane's columns, broken at spaces. The contract SHALL be hidden at first, and whether it shows SHALL hold across components and a reload. Pressing the button SHALL NOT rerun a `clm` command whose output the pane already holds for this generation (`panel-inspector-contract`).

#### Serves
- capability/see-project-state

#### Needs
- view/component-detail

#### Scenario: changes and backlog
- GIVEN a terminal pane 80 columns wide on the Spec tab with `a` selected, whose `clm view spec --format json` gives `a` the `changes` entry `20260101-c` with status `applied`, current 1, total 2 and complete false, and the `backlog` item `q` of kind `question` titled `Ask`
- WHEN the pane draws
- THEN the inspector holds, after the work in flight, a line holding `20260101-c`, `applied` and `1/2`, a line holding `backlog/q`, `question` and `Ask`, and a `secondary` `Contract` button, and none of `a`'s `contract` text

#### Scenario: nothing touches it
- GIVEN `b` selected, whose `changes` and `backlog` in the `spec` model are empty
- WHEN the pane draws
- THEN the inspector holds `changes none` and `backlog none`

#### Scenario: contract shown
- GIVEN `a` selected, its `contract` being a paragraph of 300 characters, the pane 80 columns wide and having run its `clm` commands this generation
- WHEN the person presses `Contract`, and in a second run the module is reloaded after that press
- THEN both times the button is `primary` and the paragraph follows it on lines no wider than 80 columns, broken at spaces, and no `clm` command has run again

#### Scenario: contract hidden again
- GIVEN the contract shown
- WHEN the person presses `Contract` again, and then presses `b`'s line
- THEN after the first press the button is `secondary` and no contract text is drawn, and `b`'s inspector draws its button `secondary` too

#### Scenario: spec model fails on the Flight tab
- GIVEN `a` selected on the Flight tab, and a `clm view spec --format json` that exits 1 with `boom` on stderr
- WHEN the pane draws
- THEN the inspector holds `boom` and no `changes`, `backlog` or `Contract` button

### Requirement: capabilities-tab
The pane SHALL have a Capabilities tab, after Blockers, that runs `clm capabilities` and `clm capabilities --format json`, cached per generation as the views are, and draws `clm capabilities`' text as lines on every surface. Each row of the list (a line after the header and before the `unattributed` line, starting with `⚑ ` or two spaces) SHALL be pressable, matched to a capability by its position among the rows and the JSON's `capabilities` and by its slug, except a row slugged `decide` or `help`, which `clm capabilities` reads as subcommands; when the counts or the slugs differ, or the JSON run fails or doesn't parse, the list SHALL be drawn with nothing pressable. Pressing a row SHALL show that capability's page, the text of `clm capabilities <slug>`, under a `Back` button that returns to the list; which page shows SHALL be kept across a reload. When `clm` exits non-zero the tab SHALL show its stderr, with the `Back` button while a page was asked for. The tab SHALL have no control that runs `clm capabilities decide`.

#### Serves
- capability/review-capability-cost
- capability/watch-beside-chat

#### Needs
- effort/capabilities-list
- effort/capability-page

#### Scenario: the list
- GIVEN a pane on the Capabilities tab, whose `clm capabilities` prints a header, rows for `adopt-clm` and `⚑ git-host`, and the `unattributed` and `architecture only` lines, and whose `clm capabilities --format json` lists `adopt-clm` then `git-host`
- WHEN the pane draws, on the terminal surface and again on the desktop surface
- THEN both times it runs `clm capabilities`, draws its lines as text, and the two rows are pressable, the second as `git-host`

#### Scenario: counts differ
- GIVEN that list, and a `clm capabilities --format json` that lists three capabilities, and in a second run one that lists `adopt-clm` then `keep-a-backlog`
- WHEN the pane draws
- THEN both times it draws the list with nothing pressable

#### Scenario: a capability's page
- GIVEN that list
- WHEN the person presses the `⚑ git-host` row, and the module is then reloaded
- THEN both times the tab runs `clm capabilities git-host` and draws a `Back` button and its lines, and pressing `Back` draws the list again

#### Scenario: nothing decides
- GIVEN a page whose last line is `decide: clm capabilities decide git-host --value high|medium|low --decision keep|cut|redesign|accept`
- WHEN the pane draws
- THEN that line is text, and no pressable line or button names `decide`

#### Scenario: clm fails
- GIVEN a `clm capabilities` that exits 1 with `boom` on stderr
- WHEN the pane draws
- THEN it shows `boom`

### Requirement: component-heading
Where the pane finds the Flight chart's component lines, for the inspector (`component-inspector`) and for the runs drawn under a line (`build-rows`), it SHALL take the lines between the heading `components` and the heading `changes`, and, when the chart has no `components` heading, between the old heading and `changes`: the old word for components (`component-words`), which a `clm` built before `view/flight-component-lines` gave its new heading still prints, so the pane works with the `clm` the session runs, older or newer (`component-words`). Labels the pane draws itself SHALL hold no old word `component-words` doesn't keep.

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

### Requirement: component-inspector
On the terminal surface the pane SHALL draw each component's line of the Spec chart (`view/spec-component-lines`) and of the Flight chart (`view/flight-component-lines`) as a pressable line holding the same text and colours, matched to the component by its position among those lines and the `spec` or `flight` model's `/nodes`, from `clm view <tab> --format json` run as the chart is. Pressing a component's line SHALL select it; pressing the selected component's line again, or the inspector's `Close` button, SHALL clear the selection. While a component is selected, the Spec and Flight tabs SHALL draw, between the tab row and the chart, an inspector holding, from the `spec` model, the component's ID, title, status, owners, `depends_on` and requirement counts (current of total, then behind, unbuilt and obsolete when non-zero, and `⚠ oversized` when the model says so), and, from the `flight` model, its work in flight: `converged` when the model doesn't list it, else its state, priority, claim, its blockers and each pending requirement's ID and state, reading no `waiting_on` field and knowing no `waiting` state (`needs-alone-order`), with the markers of an obsolete one. The selection SHALL hold across the Spec and Flight tabs and across a reload, and a selected component the `spec` model doesn't list SHALL draw no inspector. A press SHALL NOT rerun a `clm` command whose output the pane already holds for this generation. When the lines can't be matched (their count differs from the model's, or the JSON run fails or doesn't parse), the tab SHALL draw its chart with no pressable line and no inspector. Other surfaces SHALL have no pressable line and no inspector.

#### Serves
- capability/see-project-state

#### Needs
- view/spec-component-lines
- check.coverage/state-names
- view/spec-view

#### Scenario: select a node
- GIVEN a terminal pane 80 columns wide on the Spec tab, whose `clm view spec --format ansi` prints the checkout line and lines for `r`, `a` and `b`, and whose `clm view spec --format json` lists `r`, `a` and `b` in that order, `a` with title `Alpha`, owners `@o`, `depends_on` `[b]`, 3 current of 4 and 1 unbuilt, and whose `clm view flight --format json` lists `a` as `ready` with pending requirement `a/x` `unbuilt`
- WHEN the person presses `a`'s line
- THEN the pane draws, above the chart, lines holding `a` and `Alpha`, `@o` and `depends on b`, `3/4` and `1 unbuilt`, `ready` and `a/x` with `unbuilt`, and a `Close` button, and `a`'s line is still drawn in the chart

#### Scenario: no waiting field
- GIVEN that pane, with `clm view flight --format json` listing `a` as `ready` with no `waiting_on` field
- WHEN the person presses `a`'s line
- THEN the inspector draws `ready` and `a/x`, and no line holding `waiting on`

#### Scenario: cut ID
- GIVEN that pane, with `clm view spec --format ansi` cutting `a`'s ID as `al…`, the model's second component being `alongname`
- WHEN the person presses the line `al…`
- THEN the inspector is `alongname`'s

#### Scenario: converged
- GIVEN that pane, with `b` absent from `clm view flight --format json`'s `/nodes`
- WHEN the person presses `b`'s line
- THEN the inspector's work in flight reads `converged` and lists no requirement

#### Scenario: deselect
- GIVEN `a` selected
- WHEN the person presses `a`'s line, and in a second run presses `Close`
- THEN both times no inspector is drawn

#### Scenario: across tabs and a reload
- GIVEN `a` selected on the Spec tab
- WHEN the person opens the Flight tab, and the module is then reloaded
- THEN both times the Flight tab draws `a`'s inspector above the chart, and `a`'s line between `components` and `changes` is pressable

#### Scenario: press reruns nothing
- GIVEN `a` selected on the Spec tab, the pane having run `clm view spec --format ansi`, `clm view spec --format json` and `clm view flight --format json` this generation
- WHEN the person presses `b`'s line
- THEN the inspector is `b`'s, and no `clm` command has run again

#### Scenario: node gone
- GIVEN `a` selected, and a redraw whose `clm view spec --format json` no longer lists `a`
- WHEN the pane draws, and again once a later redraw lists `a`
- THEN the first draw holds no inspector, and the second holds `a`'s

#### Scenario: lines and model disagree
- GIVEN a Spec tab whose `clm view spec --format ansi` prints three component lines and whose `clm view spec --format json` lists four components
- WHEN the pane draws
- THEN it draws the three lines as text, with no pressable line and no inspector

#### Scenario: model fails
- GIVEN `a` selected, and a `clm view flight --format json` that exits 1 with `boom` on stderr
- WHEN the Spec tab draws
- THEN the inspector holds `a`, `Alpha` and `boom` where its work in flight would be, and when `clm view spec --format json` fails instead on the Flight tab, the inspector holds `a` and that run's stderr where the title and counts would be

#### Scenario: no presses on the desktop
- GIVEN `a` selected, and the pane on the desktop surface
- WHEN the pane draws the Spec tab
- THEN it draws no inspector and no pressable line

### Requirement: manifest-owner
`plugin-panel/.claude-plugin/plugin.json` SHALL name `Christopher Lunny` as its `author` and `MIT` as its `license`.

#### Scenario: the owner's full name
- GIVEN the repository
- WHEN a test reads `plugin-panel/.claude-plugin/plugin.json`
- THEN `author.name` is `Christopher Lunny` and `license` is `MIT`

#### Serves
- capability/install-and-release
