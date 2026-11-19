---
component: effort
base: { design: "519463e2e0a7af99" }
---

## MODIFIED Design
**An `effort` record's `id`** is `ledger`'s `effort_id(session, branch)`, the first 32 hex digits of SHA-256 over `<session>\n<branch>`. The fold keeps, for each `id`, the record with the latest `at`, whatever the writer (`ledger/effort-records`). A record holds the larger of what a run read and what the fold holds (`record-command`), so a run from another clone with more of the session's transcript supersedes the earlier count, and a run that sees less of it (a worktree since removed, a `--cwd` not given again) never shrinks it.

**Which transcripts count** (`transcripts`).
- *Counted directories:* the repository's main worktree, every worktree `git worktree list --porcelain` names (`.clm/worktrees/` and `.claude/worktrees/` among them, and `../clm-wt/` when they are there), and each `--cwd <dir>` given.
- *Project directories read:* those directly under the transcripts directory whose name starts with a counted directory's path with every character but an ASCII letter or digit replaced by `-` (Claude Code's naming), so a session's `subagents/` files are read with it. A name that starts with the encoding of `/x/clm` also starts with `/x/clm-wt`'s, so the entries' own `cwd` decides.
- *Entries counted:* a `cwd` under a counted directory is compared by path component (`Path::starts_with`), so `/x/clm-wt` isn't under `/x/clm`.
- *Branch:* Claude Code writes the launching session's branch into every entry of a subagent's file, so for a file under `subagents/` the branch is read from where the entries ran: the linked worktree their `cwd` is under, as `git worktree list --porcelain` names its branch when the record is made. `clm land` runs in the build's worktree with the build's branch checked out, which is when a build's work is recorded (`land/records-effort`). The main worktree is never read this way. A subagent that worked on two branches in one worktree has all of that work counted for the one checked out when the record is made; no build does that.
- *A subagent's branch is pinned once recorded.* Since a record never shrinks, an entry that moves from one record to another is counted twice, and a build's worktree is removed after every build (and a harness without worktrees of its own has `clm-loop` reuse `.claude/worktrees/agent-<name>` for another branch), so the worktree can't be asked again later. The work a subagent did in a worktree with a branch is therefore grouped under the session `<sessionId>/<agent>`, `<agent>` the subagent file's name without `.jsonl` (Claude Code names each launch's file afresh), and the `effort` record made for it is the pin: a later run reading a file whose `<sessionId>/<agent>` the fold holds a record of counts all of the file's entries for that record's branch, whatever the worktree now holds. No ledger field is added: the `session` string carries the pin, and nothing reads an `effort` record's `session` but its `id` and `clm ledger show`. When two or more records hold one `<sessionId>/<agent>` (clones that saw the worktree on different branches), the one whose branch sorts first, by byte order, pins it, whatever their `at`. An `at` can't decide it: the fold keeps only each `id`'s latest record (`ledger/effort-records`), so a re-record of one branch moves its `at` past the other's, and compaction drops the earlier records, so nothing in the ledger says which branch was recorded first. The branches the records hold never change when either is re-recorded, so the pin stays where it is, and every clone folds to the same pin. A pin is an `effort` record whose `session` holds a `/`, which no Claude Code `sessionId` does. A `--cwd` inside a detached linked worktree is that worktree's, so it is detached too. With `branch` given, a pinned file's lines are read whether or not they name the branch or its worktree, since a pinned subagent's worktree may be gone. A pinned entry still counts only when its `cwd` is under a counted directory: a removed worktree outside the main one counts for nothing, which shrinks no record.
- *Until it is pinned:* an entry of a subagent file in a linked worktree that is detached counts for nothing, since the worktree's branch is not known yet and a record made now would count it twice once the build claims its branch; the entry is counted when the worktree has a branch, or for its own `gitBranch` once the worktree is gone. An entry whose `cwd` is under no linked worktree (a subagent run in the main worktree, or one whose unpinned worktree is gone) counts for its own `gitBranch` under its `sessionId`, joining the session's own work.
- *Accepted:* a subagent never recorded while its worktree had a branch, whose worktree path is then reused for another branch, counts for the new branch, once. An `effort` record that holds a subagent's work under a bare `<sessionId>` pins nothing, so a later run counts that work for its `gitBranch` too. Two clones that recorded one subagent for different branches each hold a record that never shrinks, so its work is counted on both, as far as each clone saw it; and a clone that records it for a third branch before it has fetched the pin moves the pin when that branch sorts first, counting the work there too.
- *Tokens:* Claude Code writes one line per content block, each repeating the response's `usage`, so each `message.id` counts once; where the lines differ, each count is the largest of them: `input_tokens`, `output_tokens`, `cache_read_input_tokens` and `cache_creation_input_tokens`, 0 when missing. Work tokens are input, output and cache writes; cache reads are kept apart.
- Relative `--cwd` and `--transcripts` paths are taken from the directory `clm` runs in.

**Attribution** (for the capabilities' cost, `attribution`; `docs/decisions/effort-records.md`). A component's cost goes to capabilities by its rules: each rule weighs 1, split over the capabilities in its effective serves (`model/serves`); a rule that serves only architectural constraints puts its weight on an `architecture only` line. A capability's *rules* are those whose effective serves names it, and its *components* are theirs. Approvals count whole, not split.

**Architecture's cost** (`architecture-report`; `docs/decisions/architecture-review.md`) is a second split of the same records, never added to the capabilities': a component's share reaches its rules as above, and each rule an entry's `#### Enforced by` names gives its part equally to the entries that name it. So the architecture list adds up to what the enforcing rules cost, not to the total, and an entry holding only `#### Untested` costs nothing, since nothing is built for it. An entry's *rules* are its enforcing rules, and its *components* theirs; its approvals, changes and burdens are counted from them as a capability's are. Its flags are the workaround items on its enforcing rules, and its overlap counts the enforcing rules no other entry names. Whether each enforcing rule is current comes from the caller, since the pure part reads no markers: `clm` passes what `clm coverage` gives, and the Worker what its marker index gives. It is a function of its own beside `report`, rather than a field of `Inputs` and `Report`, so the capabilities' report, its JSON and its callers are untouched.

**Prices** live in `prices.rs`, one row per model family in USD per million tokens for input, output, cache reads and cache writes, checked against the list prices on the date the file states. A model ID matches the first row whose pattern its lowercased ID contains; an ID no row matches is priced as the `sonnet` row. Every cache write is priced at the 5-minute rate, and Haiku 5.5 at its rate for prompts of 100K tokens or fewer. The panel keeps a copy of these rows for its runs (`plugin-panel/src/agents.ts`, `plugin.panel`), so a price change changes both; this one prices the records.

**History** is what only git knows: the commits that touched each component's `SPEC.md` or its `paths`, the life of each `question`, `human-task` and `workaround` item (when its file was added and deleted, its `kind` as added, and every `components` and `blocks` entry its file held while it lived), read from one patch log over `backlog/` whose patches hold their whole files, and each component's size. An item is a capability's when its `blocks` names one of the capability's rules or its `components` one of its components. The Worker has no history, so its page leaves those numbers out.

**"Can we live without it?"** is computed, never written by a model (`deterministic-enforcement`): how many of the capability's rules serve no other capability, and which other capabilities the rest serve, with counts.

**A flag is settled per capability, by slug** A decision lists in its record's `settles` the slugs open on its capability when it is made (`unsettled-flags`), so a flag filed later under another slug, or open on another capability its rule also serves, stays unsettled there.

## MODIFIED Requirements

### Requirement: record-command <!-- base: 05b07a1db0663704 -->
`clm effort record [--branch <b>] [--transcripts <dir>] [--cwd <dir>]... [--root <dir>]` SHALL read the sessions of the repository's worktrees and each `--cwd`, from `--transcripts` or else `$CLAUDE_CONFIG_DIR/projects` or `$HOME/.claude/projects`, with each `<session>/<agent>` the fold holds an `effort` record of pinned to that record's branch, the branch that sorts first by byte order deciding among two or more, whatever their `at` (`unrecorded` reading with the same pins), and append, in one `append_all`, one `effort` record per session and branch (only `<b>`'s with `--branch`) holding, per model and count and for the active seconds, the larger of what it read and of the record the fold holds for its `effort_id`, when that differs from the folded record. It SHALL print `effort <branch> <session> <work tokens> tokens <active>s` per record and then `recorded <n> sessions on <m> branches`, or `nothing new to record`, and exit 0; 1 when it refuses, outside a CLM repository; 2 when git or the ledger fails.

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
- WHEN the `clm` binary runs `effort record --transcripts <dir> --branch impl/a`
- THEN it prints the record's line and `recorded 1 sessions on 1 branches` and exits 0
