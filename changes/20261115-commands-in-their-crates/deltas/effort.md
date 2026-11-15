---
component: effort
base: { contract: "64b5dfda95ffd905" }
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

`clm-effort` offers each of `clm effort`, `clm capabilities` and `clm architecture` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the directory it runs in, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## ADDED Requirements

### Requirement: commands-in-crate
`clm-effort` SHALL offer each of `clm effort`, `clm capabilities` and `clm architecture` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the directory it runs in, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `effort`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `effort`'s own files and call `clm-effort` itself, this call where the requirement states what a command prints or how it exits: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `effort`'s own files
- WHEN it calls `clm-effort`'s call with the arguments of `clm effort` and two buffers
- THEN the buffers hold what `clm effort` prints to standard output and to standard error there, and the call returns the status `clm effort` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm effort` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
