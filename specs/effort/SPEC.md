---
title: Effort
status: active
owners: ["@clunny"]
depends_on: [model, check, check.coverage, ledger, config, vcs]
paths: ["crates/clm-effort/**"]
successor: ""
serves: [capability/review-capability-cost, architecture/effort-accounted]
---

## Contract
The work each land spent, read from Claude Code's transcripts on this machine and kept as `effort` records in the ledger (`effort-accounted`). Of a transcript line it reads the keys `sessionId`, `gitBranch`, `cwd`, `timestamp`, `message.id`, `message.model` and, of `message.usage`, `input_tokens`, `output_tokens`, `cache_read_input_tokens` and `cache_creation_input_tokens` (`transcripts`). Crate `clm-effort` offers, under its default `git` feature:
- `sessions(transcripts, counted, pinned, branch)`: the work in the transcripts under `transcripts` whose entries ran in one of the `counted` directories, each given with a `Checkout` (`Unlinked` for one that is no linked worktree, `Branch(<branch>)` for a linked worktree with that branch checked out, `Detached` for one with none), as one `SessionWork` (`session`, `branch`, `models`, a `Usage` per model, `active_seconds`) per session and branch, optionally only those on `branch`, with the warnings the reading gave. A subagent's entries count for the branch checked out in the worktree it ran in, so a build run as a subagent is recorded against its `impl/<component>` branch, under the session `<session>/<agent>`, `<agent>` its file's name without `.jsonl`; once recorded so, `pinned` (the branch of each such `<session>/<agent>` the fold holds a record of) keeps them on that branch after the worktree is removed or checks out another (`transcripts`).
- `unrecorded(root, options, env)`: an `Unrecorded`, the `effort` bodies `record` would append (`bodies`) and the reading's `warnings`, without appending them, for `clm land` to append with its `landed` record. It reads `pinned` from the fold, as `record` does.
- `record(root, options, env)`: `clm effort record`, returning a `Recorded`, the lines the command prints (`lines`) and its `warnings`. `options` is an `Options`: `branch`, `transcripts` and `cwds`, the command's `--branch`, `--transcripts` and `--cwd`s. It reads the transcripts under `options.transcripts`, or by default `$CLAUDE_CONFIG_DIR/projects`, else `$HOME/.claude/projects`, from the injected environment, an empty variable counting as unset, and appends one `effort` record per session and branch whose counts exceed what the fold holds for its `id`, as one commit (`ledger/append-all`).
- `unrecorded`, `record`, `decide`, `history` and `capabilities` fail with an `Error`, which displays as the message the commands print after their prefix, and whose `exit_code()` is 1 for a refusal (no `specs/SPEC.md`, specs that don't load, a slug `CAPABILITIES.md` or `ARCHITECTURE.md` doesn't hold) and 2 when git, the ledger or reading the files fails.

`clm effort record [--branch <b>] [--transcripts <dir>] [--cwd <dir>]... [--root <dir>]` prints `effort <branch> <session> <work tokens> tokens <active>s` for each record it appends, then `recorded <n> sessions on <m> branches`, or `nothing new to record`, each warning on standard error as `warning: <text>`. It exits 0, 1 when it refuses (no CLM repository), and 2 when git or the ledger fails, each with `clm effort: <message>` on standard error. `clm land` appends what `unrecorded` gives for the branch it lands with its `landed` record (`land/records-effort`).

The review pages, `clm capabilities`, `clm architecture` and their pages, built on the pure part, which builds without the `git` feature for the Worker (`effort-builds-for-wasm`); its result types (`Report`, `Cost`, `Constraint`, `Enforcing`, `Spend`, `Flag`, `Overlap`, `Decided`) are `Serialize` and `Deserialize`, so the Worker sends them as JSON:
- `report(inputs)`: a `Report` (`capabilities`, ranked by dollars, `unattributed`, `architecture_only`) from an `Inputs` (`graph`, `fold`, an optional `History`, `flag_text`, the workaround items' text by slug), one `Cost` per capability: `id`, `slug`, `version`, `statement`, `why`, `rules`, the components (`.nodes`, a name `component-words` keeps); `spend` (a `Spend`: `dollars`, `work_tokens`, `cache_read_tokens`, `active_seconds`), `changes`, `approvals`, `flags_settled`, `burdens`; `commits`, `questions_answered`, `blocking` (items and days) and `size_tokens`, `None` without history; `flags` (each a `Flag`: `slug`, `requirement`, `title`, `text`, `settled`, `added`), `overlap` (an `Overlap`: `only`, `shared`), `decisions` (each a `Decided`: `at`, `writer`, `value`, `decision`), `flagged` and `text`, its entry's Markdown (`entry-text`).
- `architecture(inputs, current)`: the architecture review (`architecture-report`) from the same `Inputs` and `current`, the IDs of the requirements that are current, or `None` when the caller can't tell, ranked by dollars: one `Constraint` per architecture entry, `id`, `slug`, `version`, `statement`, `enforced_by` (each an `Enforcing`: `rule`, and `current`, `None` without `current`), `untested` (its `#### Untested` text, or `None`), `rules`, the components (`.nodes`), `spend`, `changes`, `approvals`, `flags_settled`, `burdens`, `flags`, `overlap`, `decisions`, `flagged` and `text`, each as a `Cost` has it, with no field only history gives.
- `unsettled(graph, fold, entry)`: the slugs of the workaround flags open and unsettled on a capability or an architecture entry, named by its ID, `capability/<slug>` or `architecture/<slug>` (`unsettled-flags`).
- `flag_text(graph, read)`: the workaround items' text by slug, each its backlog file's body after the frontmatter, trimmed, `read` giving a file's text by its path, so `capabilities` and the Worker read flags alike.
- `capabilities_of(graph, id)`: the capabilities rule `id` serves that `CAPABILITIES.md` holds (`attribution`), which `clm workaround` counts against (`check.health`).
- `dollars(model, usage)`: a usage's estimated cost (`prices`).
- `list_text(report)` and `page_text(cost)`: the lines `clm capabilities` and `clm capabilities <slug>` print; `architecture_text(constraints)` and `constraint_text(constraint)`: the lines `clm architecture` and `clm architecture <slug>` print.
- Under `git`: `history(root)`, a `History` (`commits` by component, `items`, each an `ItemLife`: `slug`, `kind`, `nodes`, the `components` its file held, `blocks`, and the commit times `added` and `deleted`; `sizes` by component, `now`) read from git and the working tree (`history`); `capabilities(root)`, the `Report` for the working tree, with its history and its workaround items' text; and `decide(root, slug, value, decision, env)`, `clm capabilities decide`, which appends a `decided` record for `capability/<slug>` settling the flags `unsettled` gives for it (`decide-command`).

`clm capabilities [<slug>] [--format text|json] [--root <dir>]` prints the list (`capabilities-list`) or one capability's page (`capability-page`), or as JSON the `Report` or the `Cost` (`capabilities-json`). It exits 0; 1 for an unknown slug, printing `no capability <slug>` on standard output, or when it refuses: with no `specs/SPEC.md`, printing `not a CLM repository: <root> has no specs/SPEC.md` on standard error, or when the specs don't load, printing `clm capabilities: the specs don't load: <path>: <message>` there; 2 when git or the ledger fails, printing `clm capabilities: <message>` there. `clm capabilities decide <slug> --value high|medium|low --decision keep|cut|redesign|accept [--root <dir>]` exits 0, 1 for an unknown slug (on standard output) or a refusal, 2 when git or the ledger fails, a refusal or failure printed on standard error after `clm capabilities decide: `, and 2 for a slug before `decide`, printed on standard error after `clm capabilities: `.

`clm architecture [<slug>] [--format text|json] [--root <dir>]` prints the architecture list (`architecture-list`) or one entry's page (`architecture-page`), or as JSON the list of `Constraint`s or one `Constraint` (`architecture-json`), exiting as `clm capabilities` does, with `no architectural constraint <slug>` for an unknown slug, and every refusal and failure, outside a CLM repository too, printed after `clm architecture: `. `clm architecture decide <slug> --value high|medium|low --decision keep|cut|redesign|accept [--root <dir>]` appends a decision on one (`architecture-decide-command`), exiting as `clm capabilities decide` does, after `clm architecture decide: `, or `clm architecture: ` for a slug before `decide`; it is offered only through `clm architecture`'s call.

Under `git`, `clm-effort` offers each of `clm effort`, `clm capabilities` and `clm architecture` as a call, `effort_command`, `capabilities_command` and `architecture_command` (`commands-in-crate`, `commands-live-in-their-crates`): each takes the command's arguments as the command line gives them, the repository root, the directory it runs in, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. Their arguments are `EffortArgs`, `CapabilitiesArgs` and `ArchitectureArgs`, which `clm`'s parser reads, each with `root()`, the `--root` the command line gives, `decide`'s own first, from which `clm` finds the root it passes. `clm` routes the command to the call and adds nothing to what it writes.

## Design
**An `effort` record's `id`** is `ledger`'s `effort_id(session, branch)`, the first 32 hex digits of SHA-256 over `<session>\n<branch>`. The fold keeps, for each `id`, the record with the latest `at`, whatever the writer (`ledger/effort-records`). A record holds the larger of what a run read and what the fold holds (`record-command`), so a run from another clone with more of the session's transcript supersedes the earlier count, and a run that sees less of it (a worktree since removed, a `--cwd` not given again) never shrinks it.

**Which transcripts count** (`transcripts`).
- *Counted directories:* the repository's main worktree, every worktree `git worktree list --porcelain` names (`.clm/worktrees/` and `.claude/worktrees/` among them, and `../clm-wt/` when they are there), and each `--cwd <dir>` given.
- *Project directories read:* those directly under the transcripts directory whose name starts with a counted directory's path with every character but an ASCII letter or digit replaced by `-` (Claude Code's naming), so a session's `subagents/` files are read with it. A name that starts with the encoding of `/x/clm` also starts with `/x/clm-wt`'s, so the entries' own `cwd` decides.
- *Entries counted:* a `cwd` under a counted directory is compared by path component (`Path::starts_with`), so `/x/clm-wt` isn't under `/x/clm`.
- *Branch:* Claude Code writes the launching session's branch into every entry of a subagent's file, so for a file under `subagents/` the branch is read from where the entries ran: the linked worktree their `cwd` is under, as `git worktree list --porcelain` names its branch when the record is made. `clm land` runs in the build's worktree with the build's branch checked out, which is when a build's work is recorded (`land/records-effort`). The main worktree is never read this way. A subagent that worked on two branches in one worktree has all of that work counted for the one checked out when the record is made; no build does that.
- *A subagent's branch is pinned once recorded.* Since a record never shrinks, an entry that moves from one record to another is counted twice, and a build's worktree is removed after every build (and a harness without worktrees of its own has `clm-loop` reuse `.claude/worktrees/agent-<name>` for another branch), so the worktree can't be asked again later. The work a subagent did in a worktree with a branch is therefore grouped under the session `<sessionId>/<agent>`, `<agent>` the subagent file's name without `.jsonl` (Claude Code names each launch's file afresh), and the `effort` record made for it is the pin: a later run reading a file whose `<sessionId>/<agent>` the fold holds a record of counts all of the file's entries for that record's branch, whatever the worktree now holds. No ledger field is added: the `session` string carries the pin, and nothing reads an `effort` record's `session` but its `id` and `clm ledger show`. When two or more records hold one `<sessionId>/<agent>` (clones that saw the worktree on different branches), the one whose branch sorts first, by byte order, pins it, whatever their `at`. An `at` can't decide it: the fold keeps only each `id`'s latest record (`ledger/effort-records`), so a re-record of one branch moves its `at` past the other's, and compaction drops the earlier records, so nothing in the ledger says which branch was recorded first. The branches the records hold never change when either is re-recorded, so the pin stays where it is, and every clone folds to the same pin. A pin is an `effort` record whose `session` holds a `/`, which no Claude Code `sessionId` does. A `--cwd` inside a detached linked worktree is that worktree's, so it is detached too. With `branch` given, a pinned file's lines are read whether or not they name the branch or its worktree, since a pinned subagent's worktree may be gone. A pinned entry still counts only when its `cwd` is under a counted directory: a removed worktree outside the main one counts for nothing, which shrinks no record.
- *Until it is pinned:* an entry of a subagent file in a linked worktree that is detached counts for nothing, since the worktree's branch is not known yet and a record made now would count it twice once the build claims its branch; the entry is counted when the worktree has a branch, or for its own `gitBranch` once the worktree is removed. An entry whose `cwd` is under no linked worktree (a subagent run in the main worktree, or one whose unpinned worktree is no longer there) counts for its own `gitBranch` under its `sessionId`, joining the session's own work.
- *Accepted:* a subagent never recorded while its worktree had a branch, whose worktree path is then reused for another branch, counts for the new branch, once. An `effort` record that holds a subagent's work under a bare `<sessionId>` pins nothing, so a later run counts that work for its `gitBranch` too. Two clones that recorded one subagent for different branches each hold a record that never shrinks, so its work is counted on both, as far as each clone saw it; and a clone that records it for a third branch before it has fetched the pin moves the pin when that branch sorts first, counting the work there too.
- *Tokens:* Claude Code writes one line per content block, each repeating the response's `usage`, and a line written while the response streams may carry a count still growing, so each `message.id` counts once, each count the largest of its lines. Work tokens are input, output and cache writes; cache reads are kept apart.
- *Time:* an entry's `timestamp` gives only its active time. The session and branch it counts for come from its other keys, so an entry whose `timestamp` can't be read loses its time and keeps its tokens.
- *The branch filter:* with `branch` given, a line that holds neither the branch's name nor the path of a directory that has it checked out can't be its entry, so it isn't read, which spares parsing every line on each `clm land`. The skipped-lines warning says the record may undercount because lines couldn't be read; with a branch the record is that branch's alone, so a broken line that can't be its entry says nothing about it and isn't counted, and a broken line that holds the branch's name is. The comparison is of the line's raw text, so a name or path JSON would write otherwise (one holding `"`, `\` or a control character, or a path that isn't UTF-8) can't be looked for, and every line it could concern is read. A line that isn't UTF-8 is searched as bytes, the same way.

**Attribution** (for the capabilities' cost, `attribution`; `docs/decisions/effort-records.md`). A component's cost goes to capabilities by its rules: each rule weighs 1, split over the capabilities in its effective serves (`model/serves`) that `CAPABILITIES.md` holds; a rule that serves none of them puts its weight on an `architecture only` line. A capability's *rules* are those whose effective serves names it, and its *components* are theirs. Approvals count whole, not split.

**Architecture's cost** (`architecture-report`; `docs/decisions/architecture-review.md`) is a second split of the same records, never added to the capabilities': a component's share reaches its rules as above, and each rule an entry's `#### Enforced by` names gives its part equally to the entries that name it. So the architecture list adds up to what the enforcing rules cost, not to the total, and an entry holding only `#### Untested` costs nothing, since nothing is built for it. A rule a live change adds, which `#### Enforced by` may name before it lands (`model/human-layers`), isn't yet one of the rules its component's share is split over, so it gives no part, and a component with no rules gives none. An entry's *rules* are its enforcing rules, and its *components* theirs; its approvals, changes and burdens are counted from them as a capability's are. Its flags are the workaround items on its enforcing rules, and its overlap counts the enforcing rules no other entry names. Whether each enforcing rule is current comes from the caller, since the pure part reads no markers: `clm` passes what `clm coverage` gives, and the Worker what its marker index gives. It is a function of its own beside `report`, rather than a field of `Inputs` and `Report`, so the capabilities' report, its JSON and its callers are untouched.

**Prices** live in `prices.rs`: one row per model family, a pattern and four rates in USD per million tokens for input, output, cache reads and cache writes, most specific first, checked against the list prices on the date the file states, and a fallback of its own, the constant `SONNET`, at the Sonnet 4 rates, for a model ID no row matches. Every cache write is priced at the 5-minute rate, and Haiku 5.5 at its rate for prompts of 100K tokens or fewer. The panel keeps a copy of these rows for its runs (`plugin-panel/src/agents.ts`, `plugin.panel`), so a price change changes both; this one prices the records. `plugin.panel`'s test reads `prices.rs` as text, matching each row as `("<pattern>", [<rates>])` and the fallback as `const SONNET: Rates = [<rates>]`, so a build of `effort` keeps each row in that shape and the fallback under that name; it can't edit that test, which is `plugin.panel`'s.

**History** is what only git knows: the commits that touched each component's `SPEC.md` or its `paths`, the life of each `question`, `human-task` and `workaround` item (when its file was added and deleted, its `kind` as added, and every `components` and `blocks` entry its file held while it lived), read from one patch log over `backlog/` whose patches hold their whole files, and each component's size. An item is a capability's when its `blocks` names one of the capability's rules or its `components` one of its components. The Worker has no history, so its page leaves those numbers out.

**"Can we live without it?"** is computed, never written by a model (`deterministic-enforcement`): how many of the capability's rules serve no other capability, and which other capabilities the rest serve, with counts.

**A flag is settled per capability, by slug.** A decision lists in its record's `settles` the slugs open on its capability when it is made (`unsettled-flags`), so a flag filed later under another slug, or open on another capability its rule also serves, stays unsettled there.

## Requirements

### Requirement: transcripts
`sessions(transcripts, counted, pinned, branch)`, `counted` giving each counted directory and, for a linked worktree, the branch checked out in it or that it has none, and `pinned` giving a branch for each `<session>/<agent>`, SHALL read every JSONL file under the project directories of `transcripts` whose names start with a counted directory's path with every character but an ASCII letter or digit replaced by `-`, recursively, the files in the order of their paths and each file's lines in order, and count an entry when it is a JSON object whose `sessionId`, `gitBranch` and `cwd` are strings and whose `cwd` is under a counted directory by path component. An entry read from a file `<agent>.jsonl` under a `subagents/` directory SHALL count, under the session `<sessionId>/<agent>`, for the branch `pinned` gives that session when it gives one; otherwise, when its `cwd` is under a linked worktree of the repository, the deepest counted directory that holds the `cwd` deciding which worktree that is, it SHALL count under `<sessionId>/<agent>` for the branch checked out there, whatever its `gitBranch`, since a subagent's entries carry the branch of the session that launched it, and SHALL count for nothing when that worktree has none. Every other entry SHALL count under its `sessionId` for its `gitBranch`. It SHALL count each `message.id` once within the session and branch it counts for, under the `message.model` of the first of its lines read there, each of `message.usage`'s `input_tokens`, `output_tokens`, `cache_read_input_tokens` and `cache_creation_input_tokens` being the largest any of its lines holds, a count that is missing or isn't written as a non-negative integer being 0; SHALL add the counts of an entry whose `message.id` isn't a string each time it reads one; SHALL count no tokens for an entry without both `message.model` and `message.usage`; and SHALL count nothing for `<synthetic>`. It SHALL read a `timestamp` that is an RFC 3339 date-time with upper-case `T` and `Z`, `YYYY-MM-DDTHH:MM:SS` with a four-digit year and every other field two digits, optionally `.` and one or more digits of a fraction of a second, then `Z` or an offset `+HH:MM` or `-HH:MM`, as the instant it names, the fraction cut to the millisecond; a field with fewer or more digits, a sign before the year, a `.` with no digits after it, and a value out of range (a month outside 01-12, a day the month doesn't have, an hour outside 00-23, a minute or second outside 00-59, an offset beyond 23:59) are not of that form. An entry whose `timestamp` is missing or not of that form SHALL add no time, and SHALL count for nothing when it lacks `message.model` or `message.usage` as well. It SHALL group the work by the session and the branch it counts for, and give as active seconds the gaps of 600 seconds or less between the group's timestamps taken in time order, whatever order its lines are in, summed and the sum rounded down to whole seconds. Without `branch` it SHALL read every line. With `branch` it SHALL read every line of every file when the branch's name holds `"`, `\` or a character below U+0020, which JSON escapes; otherwise every line of a subagent's file whose name without `.jsonl` is the part after the last `/` of a key of `pinned`, every line of any other subagent's file when the path of a counted directory given with that branch checked out (a linked worktree, or a `--cwd` inside one) isn't UTF-8 or holds `"`, `\` or a character below U+0020, and else only a line whose bytes hold, case-sensitively and anywhere in the line, the bytes of the branch's name or, in a subagent's file, of the path of such a directory; a line it doesn't read counts for nothing. Of the lines it reads, it SHALL skip one that isn't JSON, a line that isn't UTF-8 counting as one, warning once `skipped <n> transcript lines that aren't JSON` with their count; a blank line, a JSON value that isn't an object, an entry whose `sessionId`, `gitBranch` or `cwd` isn't a string, and a file it can't read SHALL count for nothing and add nothing to the warning. It SHALL warn `no transcripts at <dir>` when the directory isn't there or can't be read.

#### Scenario: repeated message
- GIVEN two lines of one session with one `message.id` and usage `{input 10, output 5}`
- WHEN `sessions` reads them
- THEN the session has input 10 and output 5

#### Scenario: how counts are read
- GIVEN, all at one time, two lines of one `message.id` with `input_tokens` 10 on both and `output_tokens` 12 and then 850, and a line with no `message.id` whose `input_tokens` is 5 and whose `output_tokens` is `"7"`, written twice; and 60 seconds later an entry with no `message`
- WHEN `sessions` reads them
- THEN the session has input 20 and output 850, and its active seconds are 60

#### Scenario: timestamps
- GIVEN, each entry a response of its own with `input_tokens` 1 unless said otherwise: session `s1`'s entries written in this order with the timestamps `2026-10-30T00:02:00Z`, `2026-10-30T00:00:00.000Z`, `2026-10-30T02:01:00.5+02:00`, `yesterday`, `2026-10-30T0:03:00Z`, `+2026-10-30T00:04:00Z`, `2026-10-30T00:05:00.Z` and `2026-10-30T00:05:75Z`; session `s2`'s one entry, with no `message`, whose timestamp is `yesterday`; and session `s3`'s entries at `2027-01-01T00:01:00Z`, `2026-13-01T00:00:00Z` and `2026-12-31T24:02:00Z`
- WHEN `sessions` reads them
- THEN `s1` has input 8 and active seconds 120, `s2` gives no `SessionWork`, and `s3` has input 3 and active seconds 0

#### Scenario: lines that count nothing
- GIVEN one session's own file holding, besides one entry on `impl/a`, a blank line, the line `{not json`, a line cut off inside a UTF-8 character, the line `[1]`, an entry without `gitBranch`, the line `{"gitBranch": "impl/b"` cut off there, and a line holding `"impl/b"` cut off inside a UTF-8 character; and beside it a file `y.jsonl` that can't be read
- WHEN `sessions` reads them, and again with `branch` `impl/b`
- THEN the first reading counts only the one entry and warns once, `skipped 4 transcript lines that aren't JSON`; the second counts nothing and warns once, `skipped 2 transcript lines that aren't JSON`, the two cut-off lines that hold `impl/b`

#### Scenario: a subagent in a build's worktree
- GIVEN `<session>/subagents/agent-x.jsonl` whose entries carry `gitBranch` `main` and a `cwd` under a linked worktree in which `impl/a` is checked out, and nothing pinned
- WHEN `sessions` reads them
- THEN the work counts for `impl/a` under the session `<session>/agent-x`, and none of it for `main`

#### Scenario: a subagent whose worktree has no branch
- GIVEN the same file, the worktree's `HEAD` detached, and nothing pinned
- WHEN `sessions` reads it
- THEN it counts nothing

#### Scenario: a subagent whose worktree is gone
- GIVEN the same file, its worktree removed, and `pinned` giving `impl/a` for `<session>/agent-x`
- WHEN `sessions` reads it
- THEN the work counts for `impl/a` under `<session>/agent-x`, and none of it for `main`

#### Scenario: a subagent's worktree reused for another branch
- GIVEN the same file, `impl/b` now checked out in its worktree, and `pinned` giving `impl/a` for `<session>/agent-x`
- WHEN `sessions` reads it
- THEN the work counts for `impl/a`, and none of it for `impl/b`

#### Scenario: a pinned subagent under a branch filter
- GIVEN the same file, its worktree removed, and `pinned` giving `impl/a` for `<session>/agent-x`
- WHEN `sessions` reads it with `branch` `impl/a`
- THEN the work counts for `impl/a` under `<session>/agent-x`

#### Scenario: an unpinned subagent whose worktree is gone
- GIVEN the same file, its worktree removed, and nothing pinned
- WHEN `sessions` reads it
- THEN the work counts for `main` under `<session>`, the entries' `gitBranch` and `sessionId`

#### Scenario: the session's own entries in a worktree
- GIVEN a session's own file, outside any `subagents/` directory, whose entries carry `gitBranch` `spec/x` and a `cwd` under a linked worktree in which `impl/a` is checked out
- WHEN `sessions` reads them
- THEN the work counts for `spec/x`

#### Scenario: split by branch
- GIVEN one `sessionId` with entries on `impl/a` and on `impl/b`
- WHEN `sessions` reads them
- THEN it gives two `SessionWork`, one per branch

#### Scenario: subagent file
- GIVEN `<session>/subagents/agent-x.jsonl` with the same `sessionId` and branch, its entries' `cwd` under no linked worktree
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
`clm effort record [--branch <b>] [--transcripts <dir>] [--cwd <dir>]... [--root <dir>]` SHALL read the sessions of the repository's worktrees and each `--cwd`, from `--transcripts` or else `$CLAUDE_CONFIG_DIR/projects` or `$HOME/.claude/projects`, a variable set empty counting as unset, with each `<session>/<agent>` the fold holds an `effort` record of pinned to that record's branch, the branch that sorts first by byte order deciding among two or more, whatever their `at` (`unrecorded` reading with the same pins), and append, as one commit on the writer's ledger branch (`ledger/append-all`), one `effort` record per session and branch (only `<b>`'s with `--branch`) holding, per model and count and for the active seconds, the larger of what it read and of the record the fold holds for the same session and branch (`ledger/effort-records`), when that differs from the folded record. A relative `--cwd`, `--transcripts` or `--root` SHALL be taken from the directory the command runs in. It SHALL print `effort <branch> <session> <work tokens> tokens <active>s` per record and then `recorded <n> sessions on <m> branches`, or `nothing new to record`, and exit 0; 1 when it refuses, outside a CLM repository; 2 when git or the ledger fails. Each warning the reading gives SHALL go to standard error as `warning: <text>`, and with neither variable set and no `--transcripts` it SHALL warn `no transcripts: neither CLAUDE_CONFIG_DIR nor HOME is set`. A refusal or failure SHALL print `clm effort: <message>` on standard error and nothing on standard output, outside a CLM repository `clm effort: not a CLM repository: <root> has no specs/SPEC.md`.

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

#### Scenario: a build's worktree removed after its record
- GIVEN a run that recorded a subagent's entries, whose `gitBranch` is `main`, for `impl/a`, the branch of the linked worktree they ran in, and the worktree then removed
- WHEN `clm effort record` runs again
- THEN it prints `nothing new to record`, and the fold holds no `effort` record of the session on `main`

#### Scenario: a pin re-recorded
- GIVEN a subagent's `<session>/<agent>` recorded for `impl/a` at t1 and, by another clone, for `impl/b` at t2, then `impl/a`'s record re-recorded with larger counts at t3 after t2, its worktree, under the main worktree, since removed, and one more line in its transcript
- WHEN `clm effort record` runs
- THEN it appends one record, for `impl/a`, and the fold's record for `impl/b` is unchanged

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
- WHEN `clm-effort`'s call for `clm effort` runs with the arguments `record --transcripts <dir> --branch impl/a`
- THEN it prints the record's line and `recorded 1 sessions on 1 branches` and exits 0

#### Scenario: relative paths
- GIVEN, under the directory the command runs in, a CLM repository `repo`, the transcripts in `projects` and a session of `impl/a` run in `elsewhere` as well as in `repo`
- WHEN `clm effort record --branch impl/a --cwd ./elsewhere --transcripts ./projects --root repo` runs there
- THEN it prints `effort impl/a s1 14 tokens 0s` and `recorded 1 sessions on 1 branches`, counting the session's work in both directories

#### Scenario: warnings and failures
- GIVEN a CLM repository
- WHEN `clm effort record --transcripts ./missing` runs; then `clm effort record` with `HOME` and `CLAUDE_CONFIG_DIR` both set empty; then `clm effort record` once `specs/SPEC.md` is removed
- THEN the first prints `warning: no transcripts at <cwd>/missing` on standard error, `<cwd>` the directory it runs in, and `nothing new to record` on standard output and exits 0; the second prints `warning: no transcripts: neither CLAUDE_CONFIG_DIR nor HOME is set` on standard error and `nothing new to record` on standard output and exits 0; and the third prints `clm effort: not a CLM repository: <root> has no specs/SPEC.md` on standard error and nothing on standard output and exits 1

### Requirement: attribution
`report` SHALL attribute each `effort` record the fold holds by its branch: `impl/<component>` to that component when the graph holds it; `spec/<id>` equally to the components the deltas of change `<id>`, live or archived, name; any other branch, and the share of a component the graph doesn't hold or that has no rules, to `unattributed`. It SHALL split a component's share equally over its rules and each rule's part equally over the capabilities in its effective serves that `CAPABILITIES.md` holds, putting the part of a rule that serves none of them on `architecture_only`, and `capabilities_of(graph, id)` SHALL give those capabilities for rule `id`. It SHALL count an `approved` record once for each capability served by a rule of its branch's components, `changes` as the changes, live or archived and whatever their status, whose deltas name one of a capability's components, and `burdens` as their declared burdens, each once. It SHALL rank the capabilities by dollars, highest first, ties in `CAPABILITIES.md`'s order.

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

#### Scenario: a capability that isn't there
- GIVEN component `n` whose one rule `x` serves only `capability/gone`, which `CAPABILITIES.md` doesn't hold, and an `effort` worth $2 on `impl/n`
- WHEN `report` runs
- THEN `architecture_only` is $2, and `capabilities_of` gives nothing for `n/x`

### Requirement: prices
`dollars(model, usage)` SHALL be the sum over input, output, cache reads and cache writes of the tokens times the rate, in USD per million tokens, of the first model family of `effort`'s price table, most specific first, whose pattern the lowercased model ID contains, divided by one million. When no pattern matches it SHALL use its fallback rates, the Sonnet 4 rates: $3 for input, $15 for output, $0.30 for cache reads and $3.75 for cache writes per million tokens, not the `sonnet-5` family's.

#### Scenario: unknown model
- GIVEN the model IDs `some-new-model` and `claude-sonnet-4-6`, which no pattern matches
- WHEN `dollars` prices a million input tokens on each
- THEN each gives $3.00, the fallback's input rate

### Requirement: history
`history(root)` SHALL give, for each component, the commits reachable from `HEAD` that touched its `SPEC.md` or a path its `paths` match; for each `question`, `human-task` and `workaround` item, each life of its file under `backlog/` (the commit time it was added, the commit time it was deleted, if it was, its `kind` as added, and every `components` and `blocks` entry its file held while it lived); each component's spec and code tokens as `clm size` counts them; and the time now. A file whose frontmatter doesn't read when it is added, or whose `kind` is then none of those three, SHALL start no life, whatever it is later edited to, and an unborn `HEAD` SHALL give no commits and no lives. `report` SHALL give a capability, with history, `commits` as the distinct commits of its components; `questions_answered` as the lives, ended by a deletion, of `question` and `human-task` items whose `blocks` held one of its rules or whose `components` held one of its components; `blocking` as the lives of `question` and `human-task` items whose `blocks` held one of its rules, and the days they were open in all: the seconds from each life's adding commit to its deleting commit, or to now while it lives, a life that ends before it starts counting 0, summed and taken in whole days rounded down; `size_tokens` by the rule share; and each flag's `added`, the UTC date, `YYYY-MM-DD`, of the commit that added the latest life of a `workaround` item of its slug, or `None` when no commit added one. Without history each is `None`.

#### Scenario: commits and items
- GIVEN a repository with commits on two components' paths, and a `question` item added and later deleted
- WHEN `history` runs
- THEN each component names its own commits, and the item has its added and deleted times

#### Scenario: odd history
- GIVEN capability `p` served by rule `a/x`; a `question` item blocking `a/x` added and deleted a day and a half apart; a `question` item blocking `a/x` whose deleting commit's time is a day before its adding commit's; an item added as an `idea` blocking `a/x` and later edited to be a `question`; and a `workaround` item on `a/x` added by a commit at 1970-01-08T00:00:00Z
- WHEN `report` runs with `history`
- THEN `p`'s `blocking` is 2 items and 1 day, and its flag's `added` is `1970-01-08`; and in a repository whose `HEAD` is unborn, `history` gives no commits and no lives

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
`clm capabilities` SHALL print `capabilities <n> · <k> ⚑ · costs are estimates: a component's cost is split by its rules' share`, with `k` the flagged ones; then one row per capability in the report's order, `⚑ ` or two spaces, the slug padded to the longest, and `  ≈$<d>  <tokens> tok  <c> commits  <a> reviews  <w> workarounds  ` then its latest decision, `<decision> <value> <YYYY-MM-DD>`, or `never decided`; then `unattributed` and `architecture only`, each padded to the width of the rows' `⚑ <slug>` or of `architecture only`, whichever is wider, followed by `  ≈$<d>  <tokens> tok`. Tokens are work tokens and SHALL read as `850`, `85k` or `1.3M`, and dollars to the cent under $10 and to the tenth above, as the panel formats them; `<w>` counts unsettled flags; and a number only history gives SHALL read `-` without it. In a directory that holds no `specs/SPEC.md`, `clm capabilities`, with a slug or without, SHALL print nothing on standard output and exit 1 with a line on standard error that starts `not a CLM repository`. When the specs don't load it SHALL print nothing on standard output and exit 1 with `clm capabilities: the specs don't load: <path>: <message>` on standard error, naming the first violation loading reports; when git or the ledger fails, it SHALL exit 2 with `clm capabilities: <message>` there.

#### Scenario: the list
- GIVEN a report of two capabilities, one flagged, without history
- WHEN `list_text` runs
- THEN its first line is `capabilities 2 · 1 ⚑ · costs are estimates: a component's cost is split by its rules' share`, the flagged row starts with `⚑ ` and shows `- commits`, and the last two lines are `unattributed` and `architecture only`

#### Scenario: outside a CLM repository
- GIVEN a directory with a `CAPABILITIES.md` holding capability `p` and no `specs/SPEC.md`
- WHEN `clm capabilities` runs there, and `clm capabilities p`
- THEN each exits 1, prints nothing on standard output, and its standard error starts `not a CLM repository`

#### Scenario: specs that don't load, and git failing
- GIVEN a git repository whose `specs/SPEC.md` has `status: bogus`, and a directory holding a `specs/SPEC.md` that loads but no git repository
- WHEN `clm capabilities` runs in each
- THEN in the first it exits 1 with `clm capabilities: the specs don't load: specs/SPEC.md: ` and the model's message on standard error, and in the second it exits 2 with a line on standard error that starts `clm capabilities: `, each printing nothing on standard output

### Requirement: capability-page
`clm capabilities <slug>` SHALL print the capability's page: `<slug>`, followed by ` ⚑` when it is flagged; its statement; `<n> rules · components <a>, <b>` (or `0 rules · no components`); a blank line; then these sections, each heading unindented and its body indented two spaces: `why we need it`; `what it costs (an estimate: a component's cost is split by its rules' share)`, with `≈$<d> · <tokens> tokens · <h>h active · <cache> cache reads`, `<c> commits · <ch> changes`, `your attention: <a> approvals · <q> questions answered · <f> flags settled`, `blocking: <i> items held its rules up for <days> days`, `size: <r> rules · <t> tokens` and `declared burdens: <list>` or `none`; `workarounds (<open> open, <u> unsettled)`, one line per flag, `⚑ ` on an unsettled one, then `<slug> <requirement> <title>` and ` (added <date>)` when known, its text indented four; `can we live without it?`, `<only> of its <n> rules serve only it; the rest also serve <cap> (<k>), …` or `all its rules serve only it`; `rules`, one ID a line; `decisions`, `<YYYY-MM-DD> <decision> (value <v>) by <writer>` a line, or `never decided`. Its last line SHALL be `decide: clm capabilities decide <slug> --value high|medium|low --decision keep|cut|redesign|accept`. An unknown slug SHALL exit 1 with `no capability <slug>` on standard output.

#### Scenario: the page
- GIVEN a capability with a flag, two rules and no decision, without history
- WHEN `page_text` runs
- THEN its sections come in order, `- commits` shows, the unsettled flag starts with `⚑ `, and the last line is the `decide:` line

#### Scenario: unknown slug
- GIVEN a repository without capability `nope`
- WHEN `clm capabilities nope` runs
- THEN it prints `no capability nope` and exits 1

#### Scenario: a capability with no rules
- GIVEN a capability no rule serves
- WHEN `clm capabilities <slug>` runs
- THEN its third line is `0 rules · no components`

### Requirement: decide-command
`clm capabilities decide <slug> --value high|medium|low --decision keep|cut|redesign|accept [--root <dir>]` SHALL append to the writer's ledger branch (`ledger/append`) a `decided` record for `capability/<slug>` with that value and decision, whose `settles` is what `unsettled` gives for the capability when it runs, print `decided capability/<slug> <decision> (value <v>), settles <n> flags` and exit 0. It SHALL exit 1, printing `no capability <slug>` on standard output, when `CAPABILITIES.md` holds no such capability, and 1 when it refuses (no CLM repository, the specs don't load), and 2 when git or the ledger fails, a refusal or failure printing `clm capabilities decide: <message>` on standard error and nothing on standard output. `decide` as `clm capabilities`'s first positional argument SHALL always be this subcommand, with `--root` before or after it, so a capability slugged `decide` has no page there; a slug before `decide` SHALL exit 2, appending nothing, with ``clm capabilities: `decide` takes the slug after it: clm capabilities decide <slug> (not <parent>)`` on standard error, `<slug>` as written and `<parent>` the slug given before `decide`, whether or not the directory is a CLM repository.

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

#### Scenario: a slug before decide
- GIVEN a CLM repository with capabilities `p` and `q`
- WHEN `clm capabilities p decide q --value high --decision keep` runs; then the same once `specs/SPEC.md` is removed; then, there, `clm capabilities decide q --value high --decision keep`
- THEN the first two exit 2 with ``clm capabilities: `decide` takes the slug after it: clm capabilities decide <slug> (not p)`` on standard error and append nothing, and the third exits 1 with a line on standard error that starts `clm capabilities decide: not a CLM repository` and nothing on standard output

### Requirement: capabilities-json
`clm capabilities --format json` SHALL print the `Report` as JSON, and `clm capabilities <slug> --format json` the capability's `Cost`, each of which deserialises back to its type.

#### Scenario: JSON
- GIVEN a CLM repository with capabilities
- WHEN `clm capabilities --format json` runs
- THEN its output deserialises to a `Report`

### Requirement: effort-builds-for-wasm
`clm-effort` SHALL build for `wasm32-unknown-unknown` with its default features off, holding `report`, `architecture`, `unsettled`, `flag_text`, `capabilities_of`, `dollars`, `list_text`, `page_text`, `architecture_text` and `constraint_text`, and without `clm-git` or `clm-check`.

#### Scenario: wasm build
- GIVEN the workspace
- WHEN `cargo build -p clm-effort --no-default-features --target wasm32-unknown-unknown` runs
- THEN it succeeds, and its dependency tree holds neither `clm-git` nor `clm-check`

### Requirement: architecture-report
`architecture(inputs, current)` SHALL give one `Constraint` per entry of `ARCHITECTURE.md`, ranked by dollars, highest first, ties in the file's order. Its `enforced_by` SHALL list the rules its `#### Enforced by` names, in block order, each `current` true when `current` holds the rule's ID, false when it doesn't, and `None` when `current` is `None`; its `untested` SHALL be its `#### Untested` text; its `statement` its text before the first `####` heading. Its spend SHALL be, for each `effort` record, the share `attribution` gives each component, split equally over the component's rules, with each rule's part split equally over the entries whose `#### Enforced by` names it; a rule an entry names that isn't yet one of its component's rules, such as one a live change adds, SHALL give no part, whether or not the component holds other rules. It SHALL count an `approved` record once when a component of its branch holds one of the entry's enforcing rules, a rule it doesn't hold yet counting for none, `changes` as the changes, live or archived and whatever their status, whose deltas name one of those rules' components, `burdens` as their declared burdens, each once, `overlap.only` as its enforcing rules that no other entry names, and `overlap.shared` as the other entries that name the rest, with how many, in the file's order. A rule the entry names that isn't current is pending, whether it is `unbuilt`, `behind` or obsolete. An entry without `#### Enforced by` SHALL have no rules, cost nothing, and have an `overlap` of 0 only and nothing shared. `report` SHALL be unchanged by it.

#### Scenario: a rule two entries name
- GIVEN component `a` with rules `x` and `y`, entry `e` enforced by `a/x` and `a/y`, entry `f` enforced by `a/x`, and an `effort` worth $4 on `impl/a`
- WHEN `architecture` runs
- THEN `e` costs $3 and `f` $1, and `e`'s overlap is 1 only and `f` (1)

#### Scenario: is it held
- GIVEN entry `e` enforced by `a/x`, current, and `a/y`, which isn't, and entry `g` holding only `#### Untested` with the text `A judgement.`
- WHEN `architecture` runs with `current` holding `a/x`, and again with `current` `None`
- THEN the first gives `e`'s `a/x` as current and `a/y` as not, and `g` no rules, no cost and the untested text `A judgement.`; the second gives each `current` as `None`

#### Scenario: a rule not built yet
- GIVEN component `a` with rules `x` and `y`, component `n` with no rules, a live change whose deltas add `a/new` and `n/new`, entry `h` enforced by `a/new` and `n/new`, an `effort` worth $4 on each of `impl/a` and `impl/n`, and an `approved` record on `impl/a`
- WHEN `architecture` runs
- THEN `h`'s dollars, work tokens, cache reads and active seconds are each 0, and its approvals 0

#### Needs
- model/human-layers

### Requirement: architecture-list
`clm architecture` SHALL print `architecture <n> · <h> held · <k> ⚑ · costs are estimates: an entry's cost is its enforcing rules' share, counted apart from the capabilities'`, with `h` the entries that name enforcing rules all of which are current and `k` the flagged ones; then one row per entry in the report's order, `⚑ ` or two spaces, the slug padded to the longest, then its proof, `held <c>/<n>` for `n` enforcing rules of which `c` are current, followed by ` + untested` when it also holds `#### Untested`, or `untested` when it holds only that, padded to the widest, then `  ≈$<d>  <tokens> tok  <a> reviews  <w> workarounds  ` and its latest decision, `<decision> <value> <YYYY-MM-DD>`, or `never decided`, each number formatted as `capabilities-list` formats it. A rule whose currency the caller couldn't tell SHALL count as not current. `clm architecture` SHALL print nothing on standard output and exit 1, with a line on standard error starting `clm architecture: `, in a directory that holds no `specs/SPEC.md` (`clm architecture: not a CLM repository: <root> has no specs/SPEC.md`) or when the specs don't load, and exit 2 so when git or the ledger fails.

#### Scenario: the list
- GIVEN entry `e`, flagged, enforced by two rules of which one is current, and entry `g` holding only `#### Untested`
- WHEN `architecture_text` runs
- THEN its first line is `architecture 2 · 0 held · 1 ⚑ · costs are estimates: an entry's cost is its enforcing rules' share, counted apart from the capabilities'`, `e`'s row starts with `⚑ ` and shows `held 1/2`, and `g`'s shows `untested`

#### Scenario: currency unknown in the list
- GIVEN entry `e` enforced by `a/x` and `a/y`, from `architecture` run with `current` `None`
- WHEN `architecture_text` runs
- THEN its first line counts `0 held`, and `e`'s row shows `held 0/2`

#### Scenario: help says architectural constraint
- WHEN `clm architecture --help` and `clm architecture decide --help` run
- THEN each calls an entry an architectural constraint, and neither holds the word `architecture` followed by the word `requirement`, in any case

#### Scenario: refusals and failures
- GIVEN a directory with no `specs/SPEC.md`, a git repository whose `specs/SPEC.md` has `status: bogus`, and a directory holding a `specs/SPEC.md` that loads but no git repository
- WHEN `clm architecture` runs in each
- THEN the first exits 1 with `clm architecture: not a CLM repository: <root> has no specs/SPEC.md`, the second exits 1 with `clm architecture: the specs don't load: specs/SPEC.md: ` and the model's message, and the third exits 2 with a line starting `clm architecture: `, each on standard error and with nothing on standard output

### Requirement: architecture-page
`clm architecture <slug>` SHALL print the entry's page: `<slug>`, followed by ` ⚑` when it is flagged; its statement; `<n> enforcing rules · components <a>, <b>` (or `0 enforcing rules · no components`); a blank line; then these sections, laid out as `capability-page` lays its out: `is it held?`, one line per enforcing rule, `current <rule>`, `pending <rule>` for one that isn't current, or `unknown <rule>` for one whose currency the caller couldn't tell, then `untested: <text>` when it holds `#### Untested`; `what it costs (an estimate: its enforcing rules' share, counted apart from the capabilities')`, with `≈$<d> · <tokens> tokens · <h>h active · <cache> cache reads`, `<ch> changes`, `your attention: <a> approvals · <f> flags settled` and `declared burdens: <list>` or `none`; `workarounds (<open> open, <u> unsettled)` as on a capability's page; `can we live without it?`, `<only> of its <n> enforcing rules enforce only it; the rest also enforce <entry> (<k>), …`, `all its enforcing rules enforce only it` or `no rule enforces it`; `decisions`, as on a capability's page. Its last line SHALL be `decide: clm architecture decide <slug> --value high|medium|low --decision keep|cut|redesign|accept`. An unknown slug SHALL exit 1 with `no architectural constraint <slug>` on standard output.

#### Scenario: the page
- GIVEN entry `e` enforced by `a/x`, current, and `a/y`, pending, holding `#### Untested` text `Partly judgement.`, with a flag and no decision
- WHEN `constraint_text` runs
- THEN its sections come in order, `is it held?` lists `current a/x`, `pending a/y` and `untested: Partly judgement.`, the unsettled flag starts with `⚑ `, and the last line is the `decide:` line

#### Scenario: currency unknown
- GIVEN entry `e` enforced by `a/x` and `a/y`, from `architecture` run with `current` `None`
- WHEN `constraint_text` runs
- THEN `is it held?` lists `unknown a/x` and `unknown a/y`

#### Scenario: unknown slug
- GIVEN a repository without architectural constraint `nope`
- WHEN `clm architecture nope` runs
- THEN it prints `no architectural constraint nope` and exits 1

### Requirement: architecture-decide-command
`clm architecture decide <slug> --value high|medium|low --decision keep|cut|redesign|accept [--root <dir>]` SHALL append to the writer's ledger branch (`ledger/append`) a `decided` record for `architecture/<slug>` with that value and decision, whose `settles` is what `unsettled` gives for the entry when it runs, print `decided architecture/<slug> <decision> (value <v>), settles <n> flags` and exit 0. It SHALL exit 1, printing `no architectural constraint <slug>` on standard output, when `ARCHITECTURE.md` holds no such entry, 1 when it refuses (no CLM repository, the specs don't load), and 2 when git or the ledger fails, a refusal or failure printing `clm architecture decide: <message>` on standard error and nothing on standard output. `decide` as `clm architecture`'s first positional argument SHALL always be this subcommand, with `--root` before or after it; a slug before `decide` SHALL exit 2, appending nothing, with ``clm architecture: `decide` takes the slug after it: clm architecture decide <slug> (not <parent>)`` on standard error, `<slug>` as written and `<parent>` the slug given before `decide`, whether or not the directory is a CLM repository.

#### Scenario: a decision
- GIVEN a CLM repository with architectural constraint `e`, enforced by a rule with one `workaround` item, and a clone with a bare `origin`
- WHEN `clm architecture decide e --value medium --decision redesign` runs
- THEN it prints `decided architecture/e redesign (value medium), settles 1 flags`, exits 0, and the fold holds the `decided` record

#### Scenario: unknown slug
- GIVEN a repository without architectural constraint `nope`
- WHEN `clm architecture decide nope --value high --decision keep` runs
- THEN it prints `no architectural constraint nope`, appends nothing and exits 1

#### Scenario: a constraint's slug before decide
- GIVEN a CLM repository with architectural constraint `e`
- WHEN `clm architecture e decide x --value high --decision keep` runs; then the same once `specs/SPEC.md` is removed; then, there, `clm architecture decide e --value high --decision keep`
- THEN the first two exit 2 with ``clm architecture: `decide` takes the slug after it: clm architecture decide <slug> (not e)`` on standard error and append nothing, and the third exits 1 with a line on standard error that starts `clm architecture decide: not a CLM repository` and nothing on standard output

#### Needs
- ledger/decision-records

### Requirement: architecture-json
`clm architecture --format json` SHALL print the `architecture` list as JSON, and `clm architecture <slug> --format json` the entry's `Constraint`, each of which deserialises back to its type.

#### Scenario: JSON
- GIVEN a CLM repository with architectural constraints
- WHEN `clm architecture --format json` runs
- THEN its output deserialises to a list of `Constraint`, one per entry

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

### Requirement: commands-in-crate
`clm-effort` SHALL offer each of `clm effort`, `clm capabilities` and `clm architecture` as a call of its own that takes the command's arguments as the command line gives them, the repository root, which the command's `--root` overrides, the directory it runs in, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and nothing to the output stream and return 2, and for `--help`, given after the command or after one of its subcommands, it SHALL write that command's help to the output stream, nothing to the error stream, and return 0. The help's first line SHALL be the command's description, then an empty line, then its `Usage:` line naming the command in full:
- `clm effort`: `The work Claude Code sessions spent, recorded in the ledger`, then `Usage: clm effort <COMMAND>`;
- `clm effort record`: ``Read this machine's transcripts and append an `effort` record for each session and branch whose counts changed``, then a line starting `Usage: clm effort record `;
- `clm capabilities`: `What each capability costs, from the ledger's effort records and git: the list, or one capability's page`, then a line starting `Usage: clm capabilities `;
- `clm capabilities decide`: `Record a review decision on a capability, settling the workaround flags open on it`, then a line starting `Usage: clm capabilities decide `;
- `clm architecture`: `What each architectural constraint costs and whether its enforcing rules are built: the list, or one constraint's page`, then a line starting `Usage: clm architecture `;
- `clm architecture decide`: `Record a review decision on an architectural constraint, settling the workaround flags open on it`, then a line starting `Usage: clm architecture decide `.

#### Scenario: a command called in a test
- GIVEN a fixture repository
- WHEN `clm-effort`'s call for `clm effort` runs with that command's arguments and two buffers
- THEN the buffers hold what `clm effort` prints to standard output and to standard error there, and the call returns the status `clm effort` exits with

#### Scenario: arguments it can't parse
- GIVEN each of the three calls
- WHEN it is given an option its command doesn't have
- THEN it returns 2 with the usage error in the error buffer and nothing in the output buffer, and the caller's process goes on

#### Scenario: help
- GIVEN each of the three calls
- WHEN it is given `--help` for each of `clm effort`, `clm effort record`, `clm capabilities`, `clm capabilities decide`, `clm architecture` and `clm architecture decide`
- THEN it returns 0 with nothing in the error buffer, and its output buffer starts with that command's description, an empty line and its `Usage:` line, as stated above

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
