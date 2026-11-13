---
title: Effort
status: active
owners: ["@clunny"]
depends_on: [model, check, ledger, config, vcs]
paths: ["crates/clm-effort/**"]
successor: ""
serves: [capability/review-capability-cost, architecture/effort-accounted]
---

## Contract
The work each land spent, read from Claude Code's transcripts on this machine and kept as `effort` records in the ledger (`effort-accounted`). Crate `clm-effort` offers, under its default `git` feature:
- `sessions(transcripts, counted, branch)`: the work in the transcripts under `transcripts` whose entries ran in one of the `counted` directories, as one `SessionWork` (`session`, `branch`, `models`, `active_seconds`) per session and branch, optionally only those on `branch`, with the warnings the reading gave.
- `unrecorded(root, options, env)`: the `effort` bodies `record` would append, and the reading's warnings, without appending them, for `clm land` to append with its `landed` record.
- `record(root, options, env)`: `clm effort record`, reading the transcripts under `options.transcripts`, or by default `$CLAUDE_CONFIG_DIR/projects`, else `$HOME/.claude/projects`, from the injected environment, and appending one `effort` record per session and branch whose counts exceed what the fold holds for its `id`, in one `append_all` (`ledger/append-all`).

`clm effort record [--branch <b>] [--transcripts <dir>] [--cwd <dir>]... [--root <dir>]` prints `effort <branch> <session> <work tokens> tokens <active>s` for each record it appends, then `recorded <n> sessions on <m> branches`, or `nothing new to record`. It exits 0, 1 when it refuses (no CLM repository), and 2 when git or the ledger fails. `clm land` appends what `unrecorded` gives for the branch it lands with its `landed` record (`land`).

The review pages, `clm capabilities`, `clm architecture` and their pages, built on the pure part, which builds without the `git` feature for the Worker (`effort-builds-for-wasm`); its result types (`Report`, `Cost`, `Constraint`, `Enforcing`, `Spend`, `Flag`, `Overlap`, `Decided`) are `Serialize` and `Deserialize`, so the Worker sends them as JSON:
- `report(inputs)`: a `Report` (`capabilities`, ranked by dollars, `unattributed`, `architecture_only`) from an `Inputs` (`graph`, `fold`, an optional `History`, `flag_text`, the workaround items' text by slug), one `Cost` per capability: `id`, `slug`, `version`, `statement`, `why`, `rules`, the components (`.nodes`, a name `component-words` keeps); `spend` (a `Spend`: `dollars`, `work_tokens`, `cache_read_tokens`, `active_seconds`), `changes`, `approvals`, `flags_settled`, `burdens`; `commits`, `questions_answered`, `blocking` (items and days) and `size_tokens`, `None` without history; `flags` (each a `Flag`: `slug`, `requirement`, `title`, `text`, `settled`, `added`), `overlap` (an `Overlap`: `only`, `shared`), `decisions` (each a `Decided`: `at`, `writer`, `value`, `decision`), `flagged` and `text`, its entry's Markdown (`entry-text`).
- `architecture(inputs, current)`: the architecture review (`architecture-report`) from the same `Inputs` and `current`, the IDs of the requirements that are current, or `None` when the caller can't tell, ranked by dollars: one `Constraint` per architecture entry, `id`, `slug`, `version`, `statement`, `enforced_by` (each an `Enforcing`: `rule`, and `current`, `None` without `current`), `untested` (its `#### Untested` text, or `None`), `rules`, the components (`.nodes`), `spend`, `changes`, `approvals`, `flags_settled`, `burdens`, `flags`, `overlap`, `decisions`, `flagged` and `text`, each as a `Cost` has it, with no field only history gives.
- `unsettled(graph, fold, entry)`: the slugs of the workaround flags open and unsettled on a capability or an architecture entry, named by its ID, `capability/<slug>` or `architecture/<slug>` (`unsettled-flags`).
- `flag_text(graph, read)`: the workaround items' text by slug, each its backlog file's body after the frontmatter, trimmed, `read` giving a file's text by its path, so `capabilities` and the Worker read flags alike.
- `dollars(model, usage)`: a usage's estimated cost (`prices`).
- `list_text(report)` and `page_text(cost)`: the lines `clm capabilities` and `clm capabilities <slug>` print; `architecture_text(constraints)` and `constraint_text(constraint)`: the lines `clm architecture` and `clm architecture <slug>` print.
- Under `git`: `history(root)`, a `History` (`commits` by component, `items`, each an `ItemLife`, `sizes` by component, `now`) read from git and the working tree, and `capabilities(root)`, the `Report` for the working tree, with its history and its workaround items' text; and `architecture_review(root)`, the `architecture` list for the working tree, with its workaround items' text and the requirements current in it, as `clm coverage` gives them.

`capabilities_of(graph, id)` gives the capabilities rule `id` serves that `CAPABILITIES.md` holds, which `clm workaround` counts against (`check.health`). Under `git`, `decide(root, slug, value, decision, env)` is `clm capabilities decide`: it appends a `decided` record for `capability/<slug>` settling the flags `unsettled` gives for it (`decide-command`); the command exits 0, 1 for an unknown slug or a refusal, 2 when git or the ledger fails or for a slug before `decide`. `decide_architecture(root, slug, value, decision, env)` is `clm architecture decide`, the same for `architecture/<slug>` (`architecture-decide-command`).

`clm capabilities [<slug>] [--format text|json] [--root <dir>]` prints the list (`capabilities-list`) or one capability's page (`capability-page`), or as JSON the `Report` or the `Cost` (`capabilities-json`). It exits 0; 1 for an unknown slug, printing `no capability <slug>`, or when the specs don't load; 2 when git fails.

`clm architecture [<slug>] [--format text|json] [--root <dir>]` prints the architecture list (`architecture-list`) or one entry's page (`architecture-page`), or as JSON the list of `Constraint`s or one `Constraint` (`architecture-json`), exiting as `clm capabilities` does, with `no architectural constraint <slug>` for an unknown slug. `clm architecture decide <slug> --value high|medium|low --decision keep|cut|redesign|accept [--root <dir>]` appends a decision on one (`architecture-decide-command`).

## Design
**An `effort` record's `id`** is `ledger`'s `effort_id(session, branch)`, the first 32 hex digits of SHA-256 over `<session>\n<branch>`. The fold keeps, for each `id`, the record with the latest `at`, whatever the writer (`ledger/effort-records`). A record holds the larger of what a run read and what the fold holds (`record-command`), so a run from another clone with more of the session's transcript supersedes the earlier count, and a run that sees less of it (a worktree since removed, a `--cwd` not given again) never shrinks it.

**Which transcripts count** (`transcripts`).
- *Counted directories:* the repository's main worktree, every worktree `git worktree list --porcelain` names (`.clm/worktrees/` and `.claude/worktrees/` among them, and `../clm-wt/` when they are there), and each `--cwd <dir>` given.
- *Project directories read:* those directly under the transcripts directory whose name starts with a counted directory's path with every character but an ASCII letter or digit replaced by `-` (Claude Code's naming), so a session's `subagents/` files are read with it. A name that starts with the encoding of `/x/clm` also starts with `/x/clm-wt`'s, so the entries' own `cwd` decides.
- *Entries counted:* a `cwd` under a counted directory is compared by path component (`Path::starts_with`), so `/x/clm-wt` isn't under `/x/clm`.
- *Tokens:* Claude Code writes one line per content block, each repeating the response's `usage`, so each `message.id` counts once; where the lines differ, each count is the largest of them: `input_tokens`, `output_tokens`, `cache_read_input_tokens` and `cache_creation_input_tokens`, 0 when missing. Work tokens are input, output and cache writes; cache reads are kept apart.
- Relative `--cwd` and `--transcripts` paths are taken from the directory `clm` runs in.

**Attribution** (for the capabilities' cost, `attribution`; `docs/decisions/effort-records.md`). A component's cost goes to capabilities by its rules: each rule weighs 1, split over the capabilities in its effective serves (`model/serves`); a rule that serves only architectural constraints puts its weight on an `architecture only` line. A capability's *rules* are those whose effective serves names it, and its *components* are theirs. Approvals count whole, not split.

**Architecture's cost** (`architecture-report`; `docs/decisions/architecture-review.md`) is a second split of the same records, never added to the capabilities': a component's share reaches its rules as above, and each rule an entry's `#### Enforced by` names gives its part equally to the entries that name it. So the architecture list adds up to what the enforcing rules cost, not to the total, and an entry holding only `#### Untested` costs nothing, since nothing is built for it. An entry's *rules* are its enforcing rules, and its *components* theirs; its approvals, changes and burdens are counted from them as a capability's are. Its flags are the workaround items on its enforcing rules, and its overlap counts the enforcing rules no other entry names. Whether each enforcing rule is current comes from the caller, since the pure part reads no markers: `clm` passes what `clm coverage` gives, and the Worker what its marker index gives. It is a function of its own beside `report`, rather than a field of `Inputs` and `Report`, so the capabilities' report, its JSON and its callers are untouched.

**Prices** live in `prices.rs`, one row per model family in USD per million tokens for input, output, cache reads and cache writes, checked against the list prices on the date the file states. A model ID matches the first row whose pattern its lowercased ID contains; an ID no row matches is priced as the `sonnet` row. Every cache write is priced at the 5-minute rate, and Haiku 5.5 at its rate for prompts of 100K tokens or fewer. The panel keeps a copy of these rows for its runs (`plugin-panel/src/agents.ts`, `plugin.panel`), so a price change changes both; this one prices the records.

**History** is what only git knows: the commits that touched each component's `SPEC.md` or its `paths`, the life of each `question`, `human-task` and `workaround` item (when its file was added and deleted, its `kind` as added, and every `components` and `blocks` entry its file held while it lived), read from one patch log over `backlog/` whose patches hold their whole files, and each component's size. An item is a capability's when its `blocks` names one of the capability's rules or its `components` one of its components. The Worker has no history, so its page leaves those numbers out.

**"Can we live without it?"** is computed, never written by a model (`deterministic-enforcement`): how many of the capability's rules serve no other capability, and which other capabilities the rest serve, with counts.

**A flag is settled per capability, by slug** A decision lists in its record's `settles` the slugs open on its capability when it is made (`unsettled-flags`), so a flag filed later under another slug, or open on another capability its rule also serves, stays unsettled there.

## Requirements

### Requirement: transcripts
`sessions(transcripts, counted, branch)` SHALL read every JSONL file under the project directories of `transcripts` whose names start with a counted directory's encoded path, recursively, and count an entry when it is a JSON object with `sessionId`, `gitBranch`, a `timestamp` and a `cwd` under a counted directory by path component. It SHALL count each `message.id`'s `message.usage` once, under `message.model`, counting nothing for `<synthetic>`, group the work by `sessionId` and `gitBranch`, and give as active seconds the sum of the gaps between the group's consecutive timestamps that are 600 seconds or less. It SHALL skip a line that isn't JSON, warning once with the count, and warn `no transcripts at <dir>` when the directory isn't there.

#### Scenario: repeated message
- GIVEN two lines of one session with one `message.id` and usage `{input 10, output 5}`
- WHEN `sessions` reads them
- THEN the session has input 10 and output 5

#### Scenario: split by branch
- GIVEN one `sessionId` with entries on `impl/a` and on `impl/b`
- WHEN `sessions` reads them
- THEN it gives two `SessionWork`, one per branch

#### Scenario: subagent file
- GIVEN `<session>/subagents/agent-x.jsonl` with the same `sessionId` and branch
- WHEN `sessions` reads it
- THEN its work joins the session's

#### Scenario: synthetic model
- GIVEN an entry whose `message.model` is `<synthetic>`
- WHEN `sessions` reads it
- THEN it counts nothing

#### Scenario: long gaps
- GIVEN a session's entries at 0, 60, 120 and 1000 seconds
- WHEN `sessions` reads them
- THEN its active seconds are 120

#### Scenario: another directory
- GIVEN an entry whose `cwd` is `/x/clm-wt`, with `/x/clm` the counted directory
- WHEN `sessions` reads it
- THEN it counts nothing

#### Scenario: no transcripts
- GIVEN a transcripts directory that isn't there
- WHEN `sessions` runs
- THEN it gives no sessions and warns `no transcripts at <dir>`

### Requirement: record-command
`clm effort record [--branch <b>] [--transcripts <dir>] [--cwd <dir>]... [--root <dir>]` SHALL read the sessions of the repository's worktrees and each `--cwd`, from `--transcripts` or else `$CLAUDE_CONFIG_DIR/projects` or `$HOME/.claude/projects`, and append, in one `append_all`, one `effort` record per session and branch (only `<b>`'s with `--branch`) holding, per model and count and for the active seconds, the larger of what it read and of the record the fold holds for its `effort_id`, when that differs from the folded record. It SHALL print `effort <branch> <session> <work tokens> tokens <active>s` per record and then `recorded <n> sessions on <m> branches`, or `nothing new to record`, and exit 0; 1 when it refuses, outside a CLM repository; 2 when git or the ledger fails.

#### Scenario: one commit
- GIVEN two sessions on `impl/a` in the transcripts and a clone with a bare `origin`
- WHEN `clm effort record` runs
- THEN the writer's ledger branch has one new commit holding two `effort` lines with their `effort_id`s, and it prints both and `recorded 2 sessions on 1 branches`

#### Scenario: rerun
- GIVEN that run
- WHEN `clm effort record` runs again
- THEN it prints `nothing new to record` and the ledger branch is unchanged

#### Scenario: more work
- GIVEN that run, then one more transcript line in one of the sessions
- WHEN `clm effort record` runs
- THEN it appends one record, which the fold holds

#### Scenario: less seen
- GIVEN a run that recorded a session's entries in the repository and in a `--cwd` directory
- WHEN `clm effort record` runs again without that `--cwd`
- THEN it prints `nothing new to record`

#### Scenario: branch filter
- GIVEN sessions on `impl/a` and `impl/b`
- WHEN `clm effort record --branch impl/a` runs
- THEN it records only `impl/a`'s

#### Scenario: linked worktree
- GIVEN a session whose `cwd` is a linked worktree of the repository
- WHEN `clm effort record` runs without `--cwd`
- THEN that session is recorded

#### Scenario: the command
- GIVEN a CLM repository and a transcript on `impl/a`
- WHEN the `clm` binary runs `effort record --transcripts <dir> --branch impl/a`
- THEN it prints the record's line and `recorded 1 sessions on 1 branches` and exits 0

### Requirement: attribution
`report` SHALL attribute each `effort` record the fold holds by its branch: `impl/<component>` to that component when the graph holds it; `spec/<id>` equally to the components the deltas of change `<id>`, live or archived, name; any other branch, and the share of a component the graph doesn't hold or that has no rules, to `unattributed`. It SHALL split a component's share equally over its rules and each rule's part equally over the capabilities in its effective serves, putting the part of a rule that serves no capability on `architecture_only`. It SHALL count an `approved` record once for each capability served by a rule of its branch's components, `changes` as the changes, live or archived and whatever their status, whose deltas name one of a capability's components, and `burdens` as their declared burdens, each once. It SHALL rank the capabilities by dollars, highest first, ties in `CAPABILITIES.md`'s order.

#### Scenario: a node's cost by its rules
- GIVEN component `a` with rule `x` serving `capability/p` and rule `y` serving `p` and `q`, and an `effort` worth $4 on `impl/a`
- WHEN `report` runs
- THEN `p` costs $3 and `q` $1

#### Scenario: a spec branch
- GIVEN change `c` with deltas on `a` and `b`, and an `effort` on `spec/c`
- WHEN `report` runs
- THEN half its cost goes to `a`'s rules and half to `b`'s

#### Scenario: unattributed
- GIVEN `effort` records on `main`, `propose/x` and `impl/gone`
- WHEN `report` runs
- THEN their cost is `unattributed`

#### Scenario: approvals count whole
- GIVEN one `approved` record on `spec/c`
- WHEN `report` runs
- THEN each capability that `a`'s and `b`'s rules serve has 1 approval

### Requirement: prices
`dollars(model, usage)` SHALL be the sum over input, output, cache reads and cache writes of the tokens times the rate of the first row of `prices.rs` whose pattern the lowercased model ID contains, divided by one million, using the `sonnet` row when none matches.

#### Scenario: unknown model
- GIVEN a model ID no row matches
- WHEN `dollars` prices a million input tokens
- THEN it gives the `sonnet` row's input rate

### Requirement: history
`history(root)` SHALL give, for each component, the commits reachable from `HEAD` that touched its `SPEC.md` or a path its `paths` match; for each `question`, `human-task` and `workaround` item, each life of its file under `backlog/` (the commit time it was added, the commit time it was deleted, if it was, its `kind` as added, and every `components` and `blocks` entry its file held while it lived), read from one patch log; each component's spec and code tokens as `clm size` counts them; and the time now. `report` SHALL give a capability, with history, `commits` as the distinct commits of its components, `questions_answered` as its deleted `question` and `human` items, `blocking` as its `question` and `human` items whose `blocks` name one of its rules and the days they were open in all, `size_tokens` by the rule share, and each flag's `added` date; without history each is `None`.

#### Scenario: commits and items
- GIVEN a repository with commits on two components' paths, and a `question` item added and later deleted
- WHEN `history` runs
- THEN each component names its own commits, and the item has its added and deleted times

### Requirement: unsettled-flags
A `workaround` item SHALL be open on each capability its requirement's effective serves names and on each architecture entry whose `#### Enforced by` names its requirement, and settled there when a `decided` record for that entry lists its slug in `settles`. `unsettled` SHALL give the slugs open and unsettled on a capability or an architecture entry, in slug order, a `Cost` or a `Constraint` is `flagged` when there is one, and its `flags_settled` counts the distinct slugs its decisions' `settles` list.

#### Scenario: a flag on two capabilities
- GIVEN a `workaround` item on `a/y`, which serves `p` and `q`, and a `decided` `accept` for `p` that settles it
- WHEN `report` runs
- THEN `p` isn't flagged and `q` is

#### Scenario: a flag on an architecture entry
- GIVEN a `workaround` item on `a/y`, which serves `p` and which entry `e`'s `#### Enforced by` names, and a `decided` `accept` for `architecture/e` that settles it
- WHEN `report` and `architecture` run
- THEN `e` isn't flagged and `p` is

### Requirement: capabilities-list
`clm capabilities` SHALL print `capabilities <n> · <k> ⚑ · costs are estimates: a component's cost is split by its rules' share`, with `k` the flagged ones; then one row per capability in the report's order, `⚑ ` or two spaces, the slug padded to the longest, and `  ≈$<d>  <tokens> tok  <c> commits  <a> reviews  <w> workarounds  ` then its latest decision, `<decision> <value> <YYYY-MM-DD>`, or `never decided`; then `unattributed` and `architecture only`, each padded to the width of the rows' `⚑ <slug>` or of `architecture only`, whichever is wider, followed by `  ≈$<d>  <tokens> tok`. Tokens are work tokens and SHALL read as `850`, `85k` or `1.3M`, and dollars to the cent under $10 and to the tenth above, as the panel formats them; `<w>` counts unsettled flags; and a number only history gives SHALL read `-` without it.

#### Scenario: the list
- GIVEN a report of two capabilities, one flagged, without history
- WHEN `list_text` runs
- THEN its first line is `capabilities 2 · 1 ⚑ · costs are estimates: a component's cost is split by its rules' share`, the flagged row starts with `⚑ ` and shows `- commits`, and the last two lines are `unattributed` and `architecture only`

### Requirement: capability-page
`clm capabilities <slug>` SHALL print the capability's page: `<slug>`, followed by ` ⚑` when it is flagged; its statement; `<n> rules · components <a>, <b>` (or `0 rules · no components`); a blank line; then these sections, each heading unindented and its body indented two spaces: `why we need it`; `what it costs (an estimate: a component's cost is split by its rules' share)`, with `≈$<d> · <tokens> tokens · <h>h active · <cache> cache reads`, `<c> commits · <ch> changes`, `your attention: <a> approvals · <q> questions answered · <f> flags settled`, `blocking: <i> items held its rules up for <days> days`, `size: <r> rules · <t> tokens` and `declared burdens: <list>` or `none`; `workarounds (<open> open, <u> unsettled)`, one line per flag, `⚑ ` on an unsettled one, then `<slug> <requirement> <title>` and ` (added <date>)` when known, its text indented four; `can we live without it?`, `<only> of its <n> rules serve only it; the rest also serve <cap> (<k>), …` or `all its rules serve only it`; `rules`, one ID a line; `decisions`, `<YYYY-MM-DD> <decision> (value <v>) by <writer>` a line, or `never decided`. Its last line SHALL be `decide: clm capabilities decide <slug> --value high|medium|low --decision keep|cut|redesign|accept`. An unknown slug SHALL exit 1 with `no capability <slug>`.

#### Scenario: the page
- GIVEN a capability with a flag, two rules and no decision, without history
- WHEN `page_text` runs
- THEN its sections come in order, `- commits` shows, the unsettled flag starts with `⚑ `, and the last line is the `decide:` line

#### Scenario: unknown slug
- GIVEN a repository without capability `nope`
- WHEN `clm capabilities nope` runs
- THEN it prints `no capability nope` and exits 1

### Requirement: decide-command
`clm capabilities decide <slug> --value high|medium|low --decision keep|cut|redesign|accept [--root <dir>]` SHALL append, through `append`, a `decided` record for `capability/<slug>` with that value and decision, whose `settles` is what `unsettled` gives for the capability when it runs, print `decided capability/<slug> <decision> (value <v>), settles <n> flags` and exit 0. It SHALL exit 1, printing `no capability <slug>`, when `CAPABILITIES.md` holds no such capability, and 1 when it refuses (no CLM repository, the specs don't load), and 2 when git or the ledger fails. `decide` as `clm capabilities`'s first positional argument SHALL always be this subcommand, with `--root` before or after it, so a capability slugged `decide` has no page there; a slug before `decide` SHALL exit 2.

#### Scenario: a decision
- GIVEN a CLM repository with capability `p` and a clone with a bare `origin`
- WHEN `clm capabilities decide p --value high --decision keep` runs
- THEN it prints `decided capability/p keep (value high), settles 0 flags`, exits 0, and the fold holds the `decided` record

#### Scenario: open flags settled
- GIVEN two `workaround` items on a rule serving `q`
- WHEN `clm capabilities decide q --value low --decision accept` runs
- THEN the record's `settles` names both, and a `workaround` item filed afterwards is unsettled on `q`

#### Scenario: unknown slug
- GIVEN a repository without capability `nope`
- WHEN `clm capabilities decide nope --value high --decision keep` runs
- THEN it prints `no capability nope`, appends nothing and exits 1

### Requirement: capabilities-json
`clm capabilities --format json` SHALL print the `Report` as JSON, and `clm capabilities <slug> --format json` the capability's `Cost`, each of which deserialises back to its type.

#### Scenario: JSON
- GIVEN a CLM repository with capabilities
- WHEN `clm capabilities --format json` runs
- THEN its output deserialises to a `Report`

### Requirement: effort-builds-for-wasm
`clm-effort` SHALL build for `wasm32-unknown-unknown` with its default features off, holding `report`, `architecture`, `unsettled`, `dollars`, `list_text`, `page_text`, `architecture_text` and `constraint_text`, and without `clm-git` or `clm-check`.

#### Scenario: wasm build
- GIVEN the workspace
- WHEN `cargo build -p clm-effort --no-default-features --target wasm32-unknown-unknown` runs
- THEN it succeeds, and its dependency tree holds neither `clm-git` nor `clm-check`

### Requirement: architecture-report
`architecture(inputs, current)` SHALL give one `Constraint` per entry of `ARCHITECTURE.md`, ranked by dollars, highest first, ties in the file's order. Its `enforced_by` SHALL list the rules its `#### Enforced by` names, in block order, each `current` true when `current` holds the rule's ID, false when it doesn't, and `None` when `current` is `None`; its `untested` SHALL be its `#### Untested` text; its `statement` its text before the first `####` heading. Its spend SHALL be, for each `effort` record, the share `attribution` gives each component, split equally over the component's rules, with each rule's part split equally over the entries whose `#### Enforced by` names it. It SHALL count an `approved` record once when a component of its branch holds one of the entry's enforcing rules, `changes` as the changes, live or archived and whatever their status, whose deltas name one of those rules' components, `burdens` as their declared burdens, each once, `overlap.only` as its enforcing rules that no other entry names, and `overlap.shared` as the other entries that name the rest, with how many, in the file's order. A rule the entry names that isn't current is pending, whether it is `unbuilt`, `behind` or obsolete. An entry without `#### Enforced by` SHALL have no rules, cost nothing, and have an `overlap` of 0 only and nothing shared. `report` SHALL be unchanged by it.

#### Scenario: a rule two entries name
- GIVEN component `a` with rules `x` and `y`, entry `e` enforced by `a/x` and `a/y`, entry `f` enforced by `a/x`, and an `effort` worth $4 on `impl/a`
- WHEN `architecture` runs
- THEN `e` costs $3 and `f` $1, and `e`'s overlap is 1 only and `f` (1)

#### Scenario: is it held
- GIVEN entry `e` enforced by `a/x`, current, and `a/y`, which isn't, and entry `g` holding only `#### Untested` with the text `A judgement.`
- WHEN `architecture` runs with `current` holding `a/x`, and again with `current` `None`
- THEN the first gives `e`'s `a/x` as current and `a/y` as not, and `g` no rules, no cost and the untested text `A judgement.`; the second gives each `current` as `None`

#### Needs
- model/human-layers

### Requirement: architecture-list
`clm architecture` SHALL print `architecture <n> · <h> held · <k> ⚑ · costs are estimates: an entry's cost is its enforcing rules' share, counted apart from the capabilities'`, with `h` the entries that name enforcing rules all of which are current and `k` the flagged ones; then one row per entry in the report's order, `⚑ ` or two spaces, the slug padded to the longest, then its proof, `held <c>/<n>` for `n` enforcing rules of which `c` are current, followed by ` + untested` when it also holds `#### Untested`, or `untested` when it holds only that, padded to the widest, then `  ≈$<d>  <tokens> tok  <a> reviews  <w> workarounds  ` and its latest decision, `<decision> <value> <YYYY-MM-DD>`, or `never decided`, each number formatted as `capabilities-list` formats it.

#### Scenario: the list
- GIVEN entry `e`, flagged, enforced by two rules of which one is current, and entry `g` holding only `#### Untested`
- WHEN `architecture_text` runs
- THEN its first line is `architecture 2 · 0 held · 1 ⚑ · costs are estimates: an entry's cost is its enforcing rules' share, counted apart from the capabilities'`, `e`'s row starts with `⚑ ` and shows `held 1/2`, and `g`'s shows `untested`

#### Scenario: help says architectural constraint
- WHEN `clm architecture --help` and `clm architecture decide --help` run
- THEN each calls an entry an architectural constraint, and neither holds the word `architecture` followed by the word `requirement`, in any case

### Requirement: architecture-page
`clm architecture <slug>` SHALL print the entry's page: `<slug>`, followed by ` ⚑` when it is flagged; its statement; `<n> enforcing rules · components <a>, <b>` (or `0 enforcing rules · no components`); a blank line; then these sections, laid out as `capability-page` lays its out: `is it held?`, one line per enforcing rule, `current <rule>`, or `pending <rule>` for one that isn't current, then `untested: <text>` when it holds `#### Untested`; `what it costs (an estimate: its enforcing rules' share, counted apart from the capabilities')`, with `≈$<d> · <tokens> tokens · <h>h active · <cache> cache reads`, `<ch> changes`, `your attention: <a> approvals · <f> flags settled` and `declared burdens: <list>` or `none`; `workarounds (<open> open, <u> unsettled)` as on a capability's page; `can we live without it?`, `<only> of its <n> enforcing rules enforce only it; the rest also enforce <entry> (<k>), …`, `all its enforcing rules enforce only it` or `no rule enforces it`; `decisions`, as on a capability's page. Its last line SHALL be `decide: clm architecture decide <slug> --value high|medium|low --decision keep|cut|redesign|accept`. An unknown slug SHALL exit 1 with `no architectural constraint <slug>`.

#### Scenario: the page
- GIVEN entry `e` enforced by `a/x`, current, and `a/y`, pending, holding `#### Untested` text `Partly judgement.`, with a flag and no decision
- WHEN `constraint_text` runs
- THEN its sections come in order, `is it held?` lists `current a/x`, `pending a/y` and `untested: Partly judgement.`, the unsettled flag starts with `⚑ `, and the last line is the `decide:` line

#### Scenario: unknown slug
- GIVEN a repository without architectural constraint `nope`
- WHEN `clm architecture nope` runs
- THEN it prints `no architectural constraint nope` and exits 1

### Requirement: architecture-decide-command
`clm architecture decide <slug> --value high|medium|low --decision keep|cut|redesign|accept [--root <dir>]` SHALL append, through `append`, a `decided` record for `architecture/<slug>` with that value and decision, whose `settles` is what `unsettled` gives for the entry when it runs, print `decided architecture/<slug> <decision> (value <v>), settles <n> flags` and exit 0. It SHALL exit 1, printing `no architectural constraint <slug>`, when `ARCHITECTURE.md` holds no such entry, 1 when it refuses (no CLM repository, the specs don't load), and 2 when git or the ledger fails. `decide` as `clm architecture`'s first positional argument SHALL always be this subcommand, with `--root` before or after it; a slug before `decide` SHALL exit 2.

#### Scenario: a decision
- GIVEN a CLM repository with architectural constraint `e`, enforced by a rule with one `workaround` item, and a clone with a bare `origin`
- WHEN `clm architecture decide e --value medium --decision redesign` runs
- THEN it prints `decided architecture/e redesign (value medium), settles 1 flags`, exits 0, and the fold holds the `decided` record

#### Scenario: unknown slug
- GIVEN a repository without architectural constraint `nope`
- WHEN `clm architecture decide nope --value high --decision keep` runs
- THEN it prints `no architectural constraint nope`, appends nothing and exits 1

#### Needs
- ledger/decision-records

### Requirement: architecture-json
`clm architecture --format json` SHALL print the `architecture` list as JSON, and `clm architecture <slug> --format json` the entry's `Constraint`, each of which deserialises back to its type.

#### Scenario: JSON
- GIVEN a CLM repository with architectural constraints
- WHEN `clm architecture --format json` runs
- THEN its output deserialises to a list of `Constraint`, one per entry

### Requirement: says-component
The text `clm capabilities` and `clm architecture` print SHALL say component and components, never the old words, as `component-words` defines them, as `capabilities-list`, `capability-page` and `architecture-page` quote it. The report's fields are data the `web` app reads and keep their names (`component-words`). A test reads the string literals in `crates/clm-effort/src/` and fails naming the file, the line and the word for each old word `component-words` doesn't keep.

#### Scenario: a capability with no rules
- GIVEN a capability no rule serves
- WHEN `clm capabilities <slug>` runs
- THEN its third line is `0 rules · no components`

#### Serves
- architecture/plain-terms

### Requirement: entry-text
`report` SHALL give each `Cost`, and `architecture` each `Constraint`, a `text`: its entry's Markdown in `CAPABILITIES.md` or `ARCHITECTURE.md` from the line after its `### ` heading to the next heading of level 3 or above or the end of the file, with leading and trailing blank lines removed, its `####` headings and blocks included. The text pages `clm capabilities <slug>` and `clm architecture <slug>` SHALL be unchanged by it, and `clm capabilities --format json` SHALL carry it.

#### Scenario: a capability's whole entry
- GIVEN `CAPABILITIES.md` holding `### Capability: x`, its statement with `` `clm land` `` in it, a `#### Why` block and a `#### Scenario: s` block, then `### Capability: y`
- WHEN `report` runs
- THEN `x`'s `text` starts with its statement, holds `` `clm land` ``, `#### Why` and `#### Scenario: s`, and holds nothing of `y`

#### Scenario: a constraint's whole entry
- GIVEN `ARCHITECTURE.md` holding entry `e` with `#### Enforced by` naming `a/x` and an `#### Untested` block
- WHEN `architecture` runs
- THEN `e`'s `text` holds its statement and both blocks

#### Serves
- capability/follow-from-browser
- capability/review-capability-cost
