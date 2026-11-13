---
component: effort
base: { contract: "ae2169a3f4ff9c1f" }
---

## MODIFIED Contract
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

## ADDED Requirements
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
