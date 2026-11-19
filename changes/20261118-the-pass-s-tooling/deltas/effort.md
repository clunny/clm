---
component: effort
base: { contract: "f336d9beb5f04676", design: "db332d8fe1a53e61" }
---

## MODIFIED Contract
The work each land spent, read from Claude Code's transcripts on this machine and kept as `effort` records in the ledger (`effort-accounted`). Crate `clm-effort` offers, under its default `git` feature:
- `sessions(transcripts, counted, branch)`: the work in the transcripts under `transcripts` whose entries ran in one of the `counted` directories, as one `SessionWork` (`session`, `branch`, `models`, `active_seconds`) per session and branch, optionally only those on `branch`, with the warnings the reading gave. A subagent's entries count for the branch checked out in the worktree it ran in, so a build run as a subagent is recorded against its `impl/<component>` branch (`transcripts`).
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

`clm-effort` offers each of `clm effort`, `clm capabilities` and `clm architecture` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the directory it runs in, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## MODIFIED Design
**An `effort` record's `id`** is `ledger`'s `effort_id(session, branch)`, the first 32 hex digits of SHA-256 over `<session>\n<branch>`. The fold keeps, for each `id`, the record with the latest `at`, whatever the writer (`ledger/effort-records`). A record holds the larger of what a run read and what the fold holds (`record-command`), so a run from another clone with more of the session's transcript supersedes the earlier count, and a run that sees less of it (a worktree since removed, a `--cwd` not given again) never shrinks it.

**Which transcripts count** (`transcripts`).
- *Counted directories:* the repository's main worktree, every worktree `git worktree list --porcelain` names (`.clm/worktrees/` and `.claude/worktrees/` among them, and `../clm-wt/` when they are there), and each `--cwd <dir>` given.
- *Project directories read:* those directly under the transcripts directory whose name starts with a counted directory's path with every character but an ASCII letter or digit replaced by `-` (Claude Code's naming), so a session's `subagents/` files are read with it. A name that starts with the encoding of `/x/clm` also starts with `/x/clm-wt`'s, so the entries' own `cwd` decides.
- *Entries counted:* a `cwd` under a counted directory is compared by path component (`Path::starts_with`), so `/x/clm-wt` isn't under `/x/clm`.
- *Branch:* Claude Code writes the launching session's branch into every entry of a subagent's file, so for a file under `subagents/` the branch is read from where the entries ran: the linked worktree their `cwd` is under, as `git worktree list --porcelain` names its branch when the record is made. `clm land` runs in the build's worktree with the build's branch checked out, which is when a build's work is recorded (`land/records-effort`). The main worktree is never read this way, and a worktree that is detached or gone leaves the entry's own `gitBranch`. A subagent that worked on two branches in one worktree has all of that work counted for the one checked out when the record is made; no build does that.
- *Tokens:* Claude Code writes one line per content block, each repeating the response's `usage`, so each `message.id` counts once; where the lines differ, each count is the largest of them: `input_tokens`, `output_tokens`, `cache_read_input_tokens` and `cache_creation_input_tokens`, 0 when missing. Work tokens are input, output and cache writes; cache reads are kept apart.
- Relative `--cwd` and `--transcripts` paths are taken from the directory `clm` runs in.

**Attribution** (for the capabilities' cost, `attribution`; `docs/decisions/effort-records.md`). A component's cost goes to capabilities by its rules: each rule weighs 1, split over the capabilities in its effective serves (`model/serves`); a rule that serves only architectural constraints puts its weight on an `architecture only` line. A capability's *rules* are those whose effective serves names it, and its *components* are theirs. Approvals count whole, not split.

**Architecture's cost** (`architecture-report`; `docs/decisions/architecture-review.md`) is a second split of the same records, never added to the capabilities': a component's share reaches its rules as above, and each rule an entry's `#### Enforced by` names gives its part equally to the entries that name it. So the architecture list adds up to what the enforcing rules cost, not to the total, and an entry holding only `#### Untested` costs nothing, since nothing is built for it. An entry's *rules* are its enforcing rules, and its *components* theirs; its approvals, changes and burdens are counted from them as a capability's are. Its flags are the workaround items on its enforcing rules, and its overlap counts the enforcing rules no other entry names. Whether each enforcing rule is current comes from the caller, since the pure part reads no markers: `clm` passes what `clm coverage` gives, and the Worker what its marker index gives. It is a function of its own beside `report`, rather than a field of `Inputs` and `Report`, so the capabilities' report, its JSON and its callers are untouched.

**Prices** live in `prices.rs`, one row per model family in USD per million tokens for input, output, cache reads and cache writes, checked against the list prices on the date the file states. A model ID matches the first row whose pattern its lowercased ID contains; an ID no row matches is priced as the `sonnet` row. Every cache write is priced at the 5-minute rate, and Haiku 5.5 at its rate for prompts of 100K tokens or fewer. The panel keeps a copy of these rows for its runs (`plugin-panel/src/agents.ts`, `plugin.panel`), so a price change changes both; this one prices the records.

**History** is what only git knows: the commits that touched each component's `SPEC.md` or its `paths`, the life of each `question`, `human-task` and `workaround` item (when its file was added and deleted, its `kind` as added, and every `components` and `blocks` entry its file held while it lived), read from one patch log over `backlog/` whose patches hold their whole files, and each component's size. An item is a capability's when its `blocks` names one of the capability's rules or its `components` one of its components. The Worker has no history, so its page leaves those numbers out.

**"Can we live without it?"** is computed, never written by a model (`deterministic-enforcement`): how many of the capability's rules serve no other capability, and which other capabilities the rest serve, with counts.

**A flag is settled per capability, by slug** A decision lists in its record's `settles` the slugs open on its capability when it is made (`unsettled-flags`), so a flag filed later under another slug, or open on another capability its rule also serves, stays unsettled there.

## MODIFIED Requirements

### Requirement: transcripts <!-- base: 8de9737c217b541b -->
`sessions(transcripts, counted, branch)`, `counted` giving each counted directory with the branch checked out in it when it is a linked worktree that has one, SHALL read every JSONL file under the project directories of `transcripts` whose names start with a counted directory's encoded path, recursively, and count an entry when it is a JSON object with `sessionId`, `gitBranch`, a `timestamp` and a `cwd` under a counted directory by path component. An entry read from a file under a `subagents/` directory whose `cwd` is under a linked worktree of the repository with a branch checked out SHALL count for that branch, whatever its `gitBranch`, since a subagent's entries carry the branch of the session that launched it; every other entry SHALL count for its `gitBranch`. It SHALL count each `message.id`'s `message.usage` once, under `message.model`, counting nothing for `<synthetic>`, group the work by `sessionId` and the branch it counts for, and give as active seconds the sum of the gaps between the group's consecutive timestamps that are 600 seconds or less. It SHALL skip a line that isn't JSON, warning once with the count, and warn `no transcripts at <dir>` when the directory isn't there.

#### Scenario: repeated message
- GIVEN two lines of one session with one `message.id` and usage `{input 10, output 5}`
- WHEN `sessions` reads them
- THEN the session has input 10 and output 5

#### Scenario: a subagent in a build's worktree
- GIVEN `<session>/subagents/agent-x.jsonl` whose entries carry `gitBranch` `main` and a `cwd` under a linked worktree in which `impl/a` is checked out
- WHEN `sessions` reads them
- THEN the session's work counts for `impl/a`, and none of it for `main`

#### Scenario: a subagent whose worktree has no branch
- GIVEN the same file, the worktree's `HEAD` detached
- WHEN `sessions` reads it
- THEN the work counts for `main`, the entries' `gitBranch`

#### Scenario: the session's own entries in a worktree
- GIVEN a session's own file, outside any `subagents/` directory, whose entries carry `gitBranch` `spec/x` and a `cwd` under a linked worktree in which `impl/a` is checked out
- WHEN `sessions` reads them
- THEN the work counts for `spec/x`

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
