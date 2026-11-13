---
title: Health
status: active
owners: ["@clunny"]
depends_on: [model, check, orchestrator, vcs, ledger, effort]
paths: ["crates/clm-health/**"]
successor: ""
serves: [capability/know-health, architecture/health-advises]
---

## Contract
`clm health [--root <dir>] [--expiry <hours>]` prints one advisory report of the repository's health (`health-advises`): what `clm check`, `clm pending`, `clm size` and `clm status` each show a part of, the things nothing shows, and the signals that say a requirement or a component is fighting its implementation. It reads the working tree and git, prints one line per finding as `<path>: health/<signal>: <message>`, grouped by signal with one summary line per signal, and exits 0 whatever it finds; 1 when the graph or the settings can't be loaded, naming what failed; 2 when git fails. No rule of `clm check`, `clm gate` or `clm land` reads it. The signals:

| Signal | One line per |
|---|---|
| `check` | violation `clm_check::run` finds, as `<path>: health/check: <rule>: <message>`; no detectors run |
| `pending` | component with pending requirements, with how many are `unbuilt`, `behind` and `obsolete` |
| `oversized` | component over a size budget, with both counts (`oversized-is-a-state`) |
| `lapsed-claim` | `origin/impl/<component>` or `origin/propose/<slug>` whose tip is older than the expiry and not in `origin/main` |
| `stalled-change` | `applied` or `implementing` change outside the archive with no requirement current |
| `unanswered` | `question` or `human-task` item added to git more than `stale_days` days ago |
| `uncited-decision` | accepted decision whose name no spec, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, guide page, live change or other decision names in a code span |
| `unserved` | entry of `ARCHITECTURE.md` or `CAPABILITIES.md` that no rule's effective serves names (`traced-rules`) |
| `history-words` | line of a Contract or Design that narrates history instead of describing the system (`spec-is-current`) |
| `design-heavy` | component whose Design is longer than its Contract and Requirements together |
| `outlier-size` | component whose code or spec tokens per requirement exceed twice the median |
| `unexercised` | run of lines no test runs, from the `lcov` file the settings name (`lcov-coverage-signal`) |
| `workaround` | `workaround` backlog item, with the requirement it names (`workaround-flags`) |
| `exemption` | exemption a component holds, with its rule, how many of the component's files it names and the item that ends it (`exemptions-signal`) |
| `rewritten` | requirement that two or more archived changes modified |
| `churn` | requirement the committed history cites at three or more versions, counted from the markers each commit added (`churn-from-marker-history`) |
| `land-time` | step, among the up to five with the most total seconds over the last 50 timed lands (`ledger/landed-times`), as `<path>: health/land-time: <step>: <runs> runs, median <n>s, total <n>s, selected by <paths>`, `<path>` the path that most often selected it or `clm.toml` when none did; the summary line is `<m> of <n> timed lands over <t>s`, `<t>` being `clm_health::Settings`' `land_check_target_seconds`, which the caller sets from `[land] check_target_seconds` (`land-time-signal`) |
| `many-scenarios` | requirement with more scenarios than the `scenarios` setting |
| `many-exceptions` | requirement whose text holds more exception clauses than the `exceptions` setting |
| `reconsider` | requirement whose score reaches the `reconsider` setting, with its breakdown and whether `backlog/reconsider-<component>-<slug>.md` exists |

A requirement's *score* is defined by `reconsider-score`. Rewrites and counts alone never file anything: the report shows them, and a build's flag is what starts the count, so a long archive doesn't fill the human's queue. `clm rebase` files a `question` item for a requirement of the branch's component whose score reaches the threshold (`sync/file-reconsider-item`). Churn is reported and never scored: it is the same moves `rewritten` counts, seen from the code, so scoring both would count each built rewrite twice, and the score is computed without git, which `clm rebase` can't afford to read the whole history for (`churn-from-marker-history`).

`clm workaround <component>/<slug> --title <title>`, the situation on stdin, files a `workaround` backlog item naming the requirement at its current version (`model/parse-backlog`) and prints its path (`workaround-command`). A build session runs it when a requirement is hard to build in one of the ways the build prompt names (`orchestrator.run/prompt-workaround`). A flag is evidence, never a block: it changes no state the queue, the gate or `clm check` reads. It exits 3 when a capability the requirement serves has `[effort] workaround_flag` unsettled flags or more, telling the session to stop and ask whether to redesign it (`redesign-before-workaround`).

The library, for `sync` and the `clm` crate: `report(root, settings, expiry)` returns what `clm health` prints as `Report { sections }`, each section a signal with its lines; `signals(graph, items, settings)` returns each requirement's `RequirementSignals { id, workarounds, rewritten, scenarios, exceptions, score }` from a loaded graph, its backlog items (the map of slugs to items) and the settings the score compares with (`reconsider-score`), with no git; `reconsider(graph, items, settings, component)` returns the requirements of one component whose score reaches the threshold, each with the text of the item `clm rebase` files; `workaround(root, requirement, title, body)` writes a flag; `crowded(root, requirement, threshold)` gives each capability the requirement serves whose unsettled flags number `threshold` or more, with the count; `churn(root)` returns `None` when `HEAD` names no commit, and otherwise, for each requirement ID the commits reachable from `HEAD` ever cited with a version, whether the graph holds it or not, its distinct versions as `Churn { id, versions }`, reading git and nothing else. `Settings { reconsider, scenarios, exceptions, stale_days, lcov, .. }`, whose last field holds the size budgets (`Settings::component_size`), is this crate's own struct, which the `clm` crate fills from `config`'s `health` and `component_size` (the budgets as `clm_check::Budgets`), so `clm-health` doesn't depend on `clm-config`, as `clm-check` takes its detectors.

## Design
Crate `clm-health`, dispatched from `crates/clm` as `clm health` and `clm workaround`; this component's build edits `crates/clm/src/main.rs`, `Cargo.toml` and `Cargo.lock` for them, as the root's Design says. It reads a checkout: the graph through `clm-model`'s `load`, the markers, coverage, pending list, sizes and `run` through `clm-check`, the claim refs through `clm-orchestrator`'s `claim_refs`, the ledger's fold through `clm-ledger` and the unsettled flags through `clm-effort`'s `unsettled` (for `clm workaround`), and git through `clm-git`. It never builds for wasm. `--root` defaults as every `clm` command's does (`clm/root-from-cwd`). `--expiry` defaults to 4 hours, as `clm claim` does, so the two agree on which claims are lapsed.

**Output.** Sections print in the order of the Contract's table. Each starts with a summary line, `health/<signal>: <n> <noun>` (`health/check: 0 violations`, `health/reconsider: 2 requirements`), followed by its lines; a section with nothing prints only its summary, so a reader sees that it ran. A signal that can't run prints its summary with the reason instead of a count: `health/unexercised: no lcov file configured`, `health/unexercised: <path> not found`, `health/lapsed-claim: no origin`. A token is four bytes, rounded up, as `check/node-size` counts.

**The signals.**
- `check`: `clm_check::run(root)` with the markers scanned at the root; each violation as `<path>: health/check: <rule>: <message>`. The command never runs detectors: they need a toolchain, and `clm check` is where they fail.
- `pending`: `clm_check::pending` over the same markers; `specs/<component path>/SPEC.md: health/pending: <n> pending (<a> unbuilt, <b> behind, <c> obsolete)`.
- `oversized`: `clm_check::size` with `config`'s `component_size` passed in with the settings; `specs/<component path>/SPEC.md: health/oversized: spec <n>/<budget>, code <n>/<budget>`.
- `lapsed-claim`: `clm-orchestrator`'s `claim_refs(root)` for the component claims and `propose_refs(root)` for the backlog claims, on the refs as fetched, without fetching (the caller fetched, or sees the last fetch, as `clm status` does), each ref carrying its tip time and whether it is merged: `impl/<component>: health/lapsed-claim: last pushed <h>h ago` for a component claim, and `propose/<slug>: ...` for a backlog claim. Whether there is an `origin` is judged first, through `clm-git`, by whether `refs/remotes/origin/main` exists; without it the summary says `no origin` and nothing is listed.
- `stalled-change`: `clm_check::change_progress`; `changes/<id>/change.md: health/stalled-change: 0/<total> requirements current` when `current` is 0 and `total` above 0.
- `unanswered`: for each `question` or `human-task` item, the commit that added its file, `git log --diff-filter=A --format=%ct -- backlog/<slug>.md` through `clm-git`, oldest when several; `backlog/<slug>.md: health/unanswered: <kind> open <d> days`. An untracked item has no commit and is skipped. `now` is passed in, so tests are deterministic.
- `uncited-decision`: the decisions `clm-model` loads, `status: accepted` only (a superseded one is history), each cited as `uncited-decisions` says. A missing `CAPABILITIES.md`, `CLAUDE.md` or `docs/guide/` is simply nothing to search.
- `unserved`: each entry of the human layers, architecture first, each layer's in file order, that no rule's effective serves (`Graph::effective_serves`, a `#### Serves` block or its component's `serves`) names. It is advisory: an entry may be proven end to end by its own tests alone.
- `history-words`: each line of a component's Contract or Design, outside fenced code blocks, that holds a date `20\d\d-\d\d-\d\d` or, case-insensitively and at word boundaries, one of the history phrases below: `specs/<component path>/SPEC.md:<line>: health/history-words: <the date or phrase>`, once per line, naming the first match. A scenario may legitimately give a date, which is why requirements and scenarios are never read. The phrase list is a constant of the crate, changed by a code PR; it sits in a fenced block so this component's own text isn't flagged for it. A phrase goes on the list only when it narrates a change wherever it appears: "no longer" stays off it, since it as often states a present condition (a local branch that no longer contains its remote tip, a tree the session no longer has), and so does "used to", since it is as often a passive that describes the present system (a ref that is used to hold the claim, a file that may be used to override a setting); the matcher reads no grammar, so a phrase that is history only in some of its forms is dropped rather than parsed (`yagni`), and a signal that advises costs reader time and invites wrong cuts each time it misfires (`health-advises`).

```
as before, today's, amended, predates, formerly, previously, was removed, is gone
```

- `design-heavy`: a component with no requirements and no Contract is skipped.
- `outlier-size`: with `clm_check::size`'s counts, each component's code tokens and spec tokens divided by its number of requirements; the median of each ratio is the middle value in sorted order, the lower of the two for an even count. `specs/<component path>/SPEC.md: health/outlier-size: <code|spec> <r> tokens per requirement, median <m>`, one line per count. Fewer than three such components gives no median and the summary says so.
- `unexercised`: the lcov file is the setting joined to the repository root. `SF:<path>` starts a file's record, `DA:<line>,<count>` gives a line's execution count, `end_of_record` ends it; other lines (`FN`, `FNDA`, `BRDA`, `LH`, `LF`) are skipped. A path is taken relative to the repository root when it is absolute under it, else as written. A run is written `<start>` alone for one line.
- `workaround`: `backlog/<slug>.md: health/workaround: <component>/<slug> <title>` per `workaround` item.
- `rewritten`: the archived changes (`changes/archive/*/deltas/<component>.md`) whose `## MODIFIED Requirements` holds `### Requirement: <slug>`, read with `clm-model`'s delta parsing; `specs/<component path>/SPEC.md: health/rewritten: <component>/<slug> modified by <n> archived changes`. A delta that fails to parse counts nothing, as it adds nothing for `blocks`.
- `churn`: `churn(root)` runs, through `clm-git`, `git -c diff.noprefix=false -c diff.mnemonicPrefix=false -c core.quotePath=false log -p --format= --no-color --no-renames --no-ext-diff --no-textconv` over the commits reachable from `HEAD`, the `-c` settings and the `--no-` options pinning the patch's shape against the machine's git configuration, and reads the output as patches: a `+++ ` line that directly follows a `--- ` line is a header, and names, as `+++ b/<path>`, the file the added lines after it belong to (`+++ /dev/null`, a deleted file, names none, and a path git quotes, one holding a control character or `"`, is skipped: no code file is named so); every other line starting with `+` is an added line, and its text after the `+` is handed to `clm-check`'s `markers_in(path, text)` with the header's path, so a marker counts exactly where `clm coverage` would count it, Markdown never does, and a marker without a version adds nothing; a removed line changes nothing, so a version once cited stays counted. The history isn't filtered by path: the scan costs the same with or without one (`clm health` and the test that runs it on this checkout pay for the whole history) and a path filter loses a file's history before a rename. A merge commit shows no patch, so a marker its resolution alone added isn't seen; the working tree's uncommitted markers aren't either. Before the log, `git rev-parse --verify --quiet HEAD` says whether there is a history: exit 1 means `HEAD` names no commit and `churn` returns `None`, any other failure is git failing, exit 2 as the Contract says; a repository with commits returns `Some` of every cited ID, the graph's and others', each `Churn { id, versions }` with `versions` the distinct 8-hex-digit versions without their `@`, sorted, and the entries sorted by `id`. The section: `health/churn: no history` with nothing listed for `None`; otherwise the summary counts `requirements`, and the lines, in the graph's order as `rewritten` lists them, are `specs/<component path>/SPEC.md: health/churn: <component>/<slug> cited at <n> versions` for each requirement the graph holds whose versions number 3 or more; an ID the graph doesn't hold is skipped, since its requirement isn't in the spec and `check/dangling-markers` covers a marker that outlives it.
- `many-scenarios`: `specs/<component path>/SPEC.md: health/many-scenarios: <component>/<slug> <n> scenarios`.
- `many-exceptions`: the requirement's text above its first `####` block, case-insensitively and at word boundaries (so `except` never matches `exception`), is searched for the crate's exception phrases, a constant like the history words, in a fenced block for the same reason: `specs/<component path>/SPEC.md: health/many-exceptions: <component>/<slug> <n> exception clauses`.

```
unless, except, but not, other than, only when, in which case, whichever, so also
```

- `reconsider`: the score (`reconsider-score`) is `f + r + s + e` with `f` the flags, `r` the archived rewrites, `s` one when the scenarios exceed `scenarios` and `e` one when the exception clauses exceed `exceptions`. `specs/<component path>/SPEC.md: health/reconsider: <component>/<slug> score <s> (workarounds <f>, rewritten <r>, scenarios <0|1>, exceptions <0|1>); filed as backlog/reconsider-<component>-<slug>.md`.

**Loading.** The graph can't be loaded only when `<root>/specs/SPEC.md` is missing, which exits 1; the violations `clm-model` reports for a tree it does load, a stale workaround flag among them, are findings under `check`, as `clm_check::run` returns them. The settings can't be loaded when `config`'s `load` rejects them, which the `clm` crate reports before calling the library, exit 1.

**Workaround flags** (`workaround-flags`, `workaround-command`). The file's `<n>` is the smallest positive integer whose file doesn't exist and whose slug no `decided` record settles (a settled slug stays settled, `effort/unsettled-flags`, so a flag filed after its requirement's earlier flags were deleted never inherits their settlement; a ledger that can't be read settles none), so flags on one requirement never overwrite each other and one build can file several. The command counts with `effort`'s `unsettled`, over the ledger's fold, so a flag a review decision settled doesn't count (`effort/unsettled-flags`). It commits nothing: the session commits the file with its work, as it does a `question` item. The model rejects a flag whose requirement isn't in the graph or whose version isn't current (`model/parse-backlog`), so the spec change that drops or rewrites the requirement deletes the flags, or records the new version when the workaround applies, and `clm check` tells it to.

**Reconsider items** (`sync/file-reconsider-item`). `reconsider(graph, items, settings, component)` scores the component's requirements from `signals` and returns, for each whose score reaches `settings.reconsider`, its ID and the item's text: `title: "Reconsider <component>/<slug>"`, `kind: question`, `components: [<component>]`, and a body as `reconsider-score` says. `clm rebase` writes it when `backlog/reconsider-<component>-<slug>.md` doesn't exist. The slug uses the component ID as written, dots included.

## Requirements

### Requirement: health-report
`clm health` SHALL print, for each signal in the Contract's table in that order, a summary line `health/<signal>: <n> <noun>` followed by one line per finding as `<path>: health/<signal>: <message>`, and SHALL exit 0 whatever it finds. It SHALL exit 1, naming what failed, when `specs/SPEC.md` is missing or the settings can't be loaded, and 2 when git fails; violations the model reports for a tree it loads SHALL be findings under `check`. It SHALL NOT run detectors.

#### Scenario: a clean repository
- GIVEN this repository's own checkout
- WHEN `clm health` runs
- THEN it prints a line starting `health/check:` and a line starting `health/reconsider:`, and exits 0

#### Scenario: findings don't fail it
- GIVEN a repository with a marker citing a requirement that doesn't exist, and a detector `bad` (`["sh", "-c", "touch ran; exit 3"]`) in its `clm.toml`
- WHEN `clm health` runs
- THEN it prints a line containing `health/check: check/dangling-markers:`, exits 0, and no file `ran` exists

#### Scenario: settings can't be loaded
- GIVEN a repository whose `clm.toml` sets `reconsider = 0` under `[health]`
- WHEN `clm health` runs
- THEN it exits 1 naming `clm.toml` and `health`

#### Needs
- config/health-setting

### Requirement: state-signals
`clm health` SHALL report each component with pending requirements under `pending` with its `unbuilt`, `behind` and `obsolete` counts; each oversized component under `oversized` with both counts against their budgets; each claim ref, `origin/impl/<component>` or `origin/propose/<slug>`, whose tip is older than the expiry and not an ancestor of `origin/main` under `lapsed-claim`; each `applied` or `implementing` change outside the archive with no requirement current under `stalled-change`; and each `question` or `human-task` item whose file was added to git more than `stale_days` days ago under `unanswered`, skipping an untracked item.

#### Scenario: pending and oversized
- GIVEN component `a` with requirement `a/x` nothing cites, and component `b` whose tracked code is 8000 bytes against a code default of 1000 tokens
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/pending: 1 pending (1 unbuilt, 0 behind, 0 obsolete)` and `specs/b/SPEC.md: health/oversized: spec <n>/12000, code 2000/1000`

#### Scenario: lapsed claim
- GIVEN `origin/impl/a` whose tip was committed 10 hours ago and isn't in `origin/main`, and `origin/impl/b` committed 1 hour ago
- WHEN `clm health --expiry 4` runs
- THEN it prints `impl/a: health/lapsed-claim: last pushed 10h ago` and no line for `impl/b`

#### Scenario: stalled change
- GIVEN an `applied` change `20260101-c` whose delta adds `a/x` and `a/y`, neither cited, and an `applied` change `20260102-d` whose added requirement is current
- WHEN `clm health` runs
- THEN it prints `changes/20260101-c/change.md: health/stalled-change: 0/2 requirements current` and no `stalled-change` line for `20260102-d`

#### Scenario: unanswered question
- GIVEN `backlog/q.md` of kind `question` added by a commit 45 days ago, `backlog/h.md` of kind `human-task` added 3 days ago, and an untracked `backlog/u.md` of kind `question`
- WHEN `clm health` runs with the default `stale_days`
- THEN it prints `backlog/q.md: health/unanswered: question open 45 days` and no `unanswered` line for `h` or `u`

#### Needs
- model/parse-backlog

### Requirement: uncited-decisions
`clm health` SHALL report each accepted decision whose name appears, in a code span outside fenced code blocks, in no component's `SPEC.md`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `AGENTS.md`, page under `docs/guide/`, file of a change outside the archive, or other decision, as `docs/decisions/<name>.md: health/uncited-decision: nothing names it`. A superseded decision SHALL NOT be reported.

#### Scenario: cited and uncited
- GIVEN decisions `kept` (named as `` `kept` `` in `specs/a/SPEC.md`), `lone` (named nowhere) and `old` (`status: superseded`, named nowhere)
- WHEN `clm health` runs
- THEN it prints `docs/decisions/lone.md: health/uncited-decision: nothing names it` and no `uncited-decision` line for `kept` or `old`

#### Scenario: named only in a fence
- GIVEN decision `fenced`, whose only mention is `` `fenced` `` inside a ```` ``` ```` block of `specs/a/SPEC.md`
- WHEN `clm health` runs
- THEN it prints an `uncited-decision` line for `fenced`

### Requirement: unserved-entries
`clm health` SHALL report each entry no rule's effective serves names as `<file>: health/unserved: <id> served by no rule`, `<file>` being `ARCHITECTURE.md` or `CAPABILITIES.md`.

#### Scenario: one served, one not
- GIVEN capability entries `c` and `d`, and component `a` with `serves: [capability/c]`
- WHEN `clm health` runs
- THEN it prints `CAPABILITIES.md: health/unserved: capability/d served by no rule` and no `unserved` line for `c`

#### Scenario: served by a rule's block
- GIVEN capability entry `c`, and component `a` without `serves` whose rule `a/x` has a `#### Serves` block naming `capability/c`
- WHEN `clm health` runs
- THEN it prints no `unserved` line

#### Serves
- architecture/traced-rules
- capability/know-health

### Requirement: spec-bloat-signals
`clm health` SHALL report, under `history-words`, each line of a component's Contract or Design outside fenced code blocks that holds a date `20\d\d-\d\d-\d\d` or one of the crate's history phrases, with the line number and the first match; under `design-heavy`, each component whose Design has more tokens than its Contract and Requirements together; and under `outlier-size`, each component whose code tokens or spec tokens per requirement exceed twice the median across the components with at least one requirement, when there are at least three such components. Requirement and scenario text SHALL NOT be read for `history-words`.

#### Scenario: history in a design
- GIVEN component `a` whose Design line 14 says `The old path, removed 2026-10-01, no longer runs`, and whose requirement `a/x` has a scenario mentioning `2026-10-01`
- WHEN `clm health` runs
- THEN it prints exactly one `history-words` line for `a`, `specs/a/SPEC.md:14: health/history-words: 2026-10-01`

#### Scenario: a present condition
- GIVEN component `a` whose Design says `A local branch that no longer contains its remote tip is kept`, and no other line of its Contract or Design holds a date or a history phrase
- WHEN `clm health` runs
- THEN it prints no `history-words` line for `a`

#### Scenario: a passive used to
- GIVEN component `a` whose Design says `The claim ref is used to hold the session's place`, and no other line of its Contract or Design holds a date or a history phrase
- WHEN `clm health` runs
- THEN it prints no `history-words` line for `a`

#### Scenario: design heavier than the rest
- GIVEN component `a` with a 2000-byte Design, a 200-byte Contract and one 300-byte requirement
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/design-heavy: design 500 tokens, contract and requirements 125`

#### Scenario: outlier
- GIVEN components `a`, `b` and `c` with one requirement each and 1000, 1200 and 1100 bytes of code, and component `d` with one requirement and 8000 bytes of code, the four `SPEC.md` files of equal size
- WHEN `clm health` runs
- THEN it prints an `outlier-size` line for `d` naming `code` and `median 275` (the lower middle of 250, 275, 300 and 2000), and none for `a`, `b` or `c`

#### Serves
- capability/know-health
- architecture/slim-components
- architecture/spec-is-current

### Requirement: unexercised-code
When the settings set `lcov` and the file exists, `clm health` SHALL read it as lcov (`SF:`, `DA:<line>,<count>`, `end_of_record`) and report, for each file some component's `paths` own, each run of consecutive lines with count 0 as `<path>:<start>-<end>: health/unexercised: <n> lines no test runs`, the summary counting lines. With `lcov` unset it SHALL print `health/unexercised: no lcov file configured`, and with the file missing `health/unexercised: <path> not found`, and in both cases report nothing else for the signal and still exit 0.

#### Scenario: a run of unexercised lines
- GIVEN `lcov = "cov.info"` whose record for `crates/a/src/lib.rs` (owned by component `a`) has `DA:3,1`, `DA:4,0`, `DA:5,0`, `DA:6,2` and `DA:9,0`, and a record for `vendor/x.rs` (owned by no component) with `DA:1,0`
- WHEN `clm health` runs
- THEN it prints `crates/a/src/lib.rs:4-5: health/unexercised: 2 lines no test runs` and `crates/a/src/lib.rs:9: health/unexercised: 1 lines no test runs`, its summary says `3 lines`, and no line names `vendor/x.rs`

#### Scenario: no file
- GIVEN `lcov = "cov.info"` and no such file
- WHEN `clm health` runs
- THEN it prints `health/unexercised: cov.info not found` and exits 0

#### Serves
- capability/know-health
- architecture/proven-by-tests

#### Needs
- config/health-setting

### Requirement: requirement-signals
`clm health` SHALL report each `workaround` backlog item under `workaround` with the requirement it names and its title; each requirement that two or more archived changes modified under `rewritten` with the count; each requirement with more scenarios than `scenarios` under `many-scenarios`; and each requirement whose text above its first `####` block holds more of the crate's exception phrases than `exceptions` under `many-exceptions`.

#### Scenario: workaround and rewrites
- GIVEN `backlog/workaround-a-x-1.md` naming `a/x` with title `Faked the clock`, and archived changes `20260101-p` and `20260102-q` whose deltas for `a` both list `x` under `## MODIFIED Requirements`, and archived change `20260103-r` whose delta modifies `a/y` only
- WHEN `clm health` runs
- THEN it prints `backlog/workaround-a-x-1.md: health/workaround: a/x Faked the clock` and `specs/a/SPEC.md: health/rewritten: a/x modified by 2 archived changes`, and no `rewritten` line for `a/y`

#### Scenario: scenarios and exceptions
- GIVEN `scenarios = 2` and `exceptions = 1` under `[health]`, requirement `a/x` with three scenarios, and requirement `a/y` whose text says `... unless the file is missing, except on Windows` and whose only scenario mentions `unless` twice
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/many-scenarios: a/x 3 scenarios` and `specs/a/SPEC.md: health/many-exceptions: a/y 2 exception clauses`

#### Serves
- capability/know-health
- architecture/redesign-before-workaround

#### Needs
- model/parse-backlog
- config/health-setting

### Requirement: reconsider-score
A requirement's score SHALL be zero when no `workaround` item names it, and otherwise the number of `workaround` items naming it, plus the number of archived changes that modified it, plus one when its scenarios exceed `scenarios` and one when its exception clauses exceed `exceptions`. `clm health` SHALL report each requirement whose score reaches `reconsider` under `reconsider` with the score, its breakdown, and `filed as backlog/reconsider-<component>-<slug>.md` when that file exists or `not filed` otherwise. The library's `reconsider(graph, items, settings, component)` SHALL return those of one component's requirements with the text of the item `clm rebase` files, whose body names the score, each flag, each rewriting change, the counts, the three ways out, and that deleting the item without a spec change brings it back on the component's next build, comparing it with no other item.

#### Scenario: at the threshold
- GIVEN the default `reconsider` of 3, requirement `a/x` with two `workaround` items and one archived change that modified it, and `a/y` with one workaround item
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/reconsider: a/x score 3 (workarounds 2, rewritten 1, scenarios 0, exceptions 0); not filed` and no `reconsider` line for `a/y`

#### Scenario: rewrites without a flag
- GIVEN requirement `a/z` with no `workaround` item, modified by five archived changes, with eight scenarios
- WHEN `clm health` runs
- THEN it prints a `rewritten` line and a `many-scenarios` line for `a/z` and no `reconsider` line for it

#### Scenario: already filed
- GIVEN the same, with `backlog/reconsider-a-x.md` present
- WHEN `clm health` runs
- THEN the `reconsider` line for `a/x` ends `filed as backlog/reconsider-a-x.md`

#### Scenario: library returns the item
- GIVEN the same graph and items, with no reconsider file
- WHEN `reconsider` is called for component `a`
- THEN it returns one entry, for `a/x`, whose text has `title: "Reconsider a/x"`, `kind: question`, `components: [a]`, names both workaround items' paths and the archived change, says to drop, change or redesign, says deleting it without a spec change brings it back on the component's next build, and mentions no split item

#### Serves
- capability/know-health
- architecture/redesign-before-workaround

#### Needs
- model/parse-backlog
- config/health-setting
- model/component-keys

### Requirement: churn-signal
`clm health` SHALL report, under `churn`, each requirement of the graph whose ID the commits reachable from `HEAD` cite at three or more distinct versions, counting the versioned `req:` markers in the added lines of every commit's patch for the files `markers_in` reads, whatever their path, as `specs/<component path>/SPEC.md: health/churn: <component>/<slug> cited at <n> versions`. It SHALL NOT count a marker without a version, a marker in a file `markers_in` doesn't read, or a marker no commit holds, and SHALL skip an ID no component holds. The library's `churn(root)` SHALL return every cited ID, held by a component or not, with its distinct versions, and `None` when `HEAD` names no commit.

#### Scenario: re-cited across commits and a rename
- GIVEN component `a` with requirements `x` and `y`, and a history in which `tests/old.rs` cites `req:a/x@11111111`, then `req:a/x@22222222`, then is renamed to `tests/new.rs` where it cites `req:a/x@33333333`, and `tests/y.rs` cites `req:a/y@aaaaaaaa`, then `req:a/y@bbbbbbbb`
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/churn: a/x cited at 3 versions` and no `churn` line for `a/y`

#### Scenario: what doesn't count
- GIVEN component `a` with requirement `z`, a history whose commits cite `req:a/z@cccccccc` and then `req:a/z@dddddddd` in `tests/z.rs`, `req:a/z@eeeeeeee` in `notes.md` only, and `req:a/z` without a version in `tests/z.rs`, and cite `a/gone`, which no component holds, at four versions in `tests/gone.rs`; and an uncommitted edit of `tests/z.rs` that cites `req:a/z@ffffffff`
- WHEN `clm health` runs, and `churn` is called for the same root
- THEN it prints no `churn` line for `a/z` or `a/gone`, and `churn` returns `a/z` with the two versions `cccccccc` and `dddddddd` and `a/gone` with four

### Requirement: workaround-command
`clm workaround <component>/<slug> --title <title>` SHALL read the body from stdin and write `backlog/workaround-<component>-<slug>-<n>.md`, `<n>` the smallest positive integer whose file doesn't exist and whose slug no `decided` record's `settles` lists, with `title`, `kind: workaround`, `components: [<component>]` (`model/component-keys`), `requirement: <component>/<slug>@<version>` at the requirement's current version, and the body, and print its path. It SHALL refuse, exit 1 and write nothing, for a requirement the graph doesn't hold, naming it. It SHALL NOT commit. After writing the flag, when some capability its requirement serves has at least `workaround_flag` unsettled flags (`config/effort-setting`, `effort/unsettled-flags`), this one included, it SHALL print to stderr `clm workaround: warning: <n> unsettled workarounds on capability/<slug>: stop and ask whether to redesign it (redesign-before-workaround)` for each such capability, and exit 3, the flag written. When the settings or the ledger can't be read it SHALL warn `clm workaround: warning: the workarounds weren't counted: <why>`, and exit 0.

#### Scenario: third flag
- GIVEN `workaround_flag` at its default, 3, and two unsettled flags on a requirement serving `capability/p`
- WHEN `clm workaround` flags it a third time
- THEN it writes the flag, prints the warning naming `3 unsettled workarounds on capability/p`, and exits 3

#### Scenario: second flag counted
- GIVEN one unsettled flag on a requirement serving `capability/p`
- WHEN `clm workaround` flags it again
- THEN it exits 0 and prints no warning

#### Scenario: settled flags not counted
- GIVEN two flags on a requirement serving `capability/p`, settled by a `decided` record for `p`
- WHEN `clm workaround` flags it a third time
- THEN it exits 0

#### Scenario: first flag
- GIVEN component `a` with requirement `a/x` at version `3fa2c9d1`
- WHEN `clm workaround a/x --title "Faked the clock"` runs with `The scenario needs a fixed date, so the test fakes the clock.` on stdin
- THEN it prints `backlog/workaround-a-x-1.md`, and that file has `kind: workaround`, `components: [a]`, `requirement: a/x@3fa2c9d1`, `title: "Faked the clock"` and the body

#### Scenario: second flag
- GIVEN the same, with `backlog/workaround-a-x-1.md` present
- WHEN `clm workaround a/x --title "Workaround"` runs
- THEN it prints `backlog/workaround-a-x-2.md` and `workaround-a-x-1.md` is unchanged

#### Scenario: no such requirement
- GIVEN component `a` with no requirement `ghost`
- WHEN `clm workaround a/ghost --title "x"` runs
- THEN it exits 1 naming `a/ghost` and writes no file

#### Serves
- capability/know-health
- architecture/redesign-before-workaround

#### Needs
- model/parse-backlog
- model/component-keys

### Requirement: says-component
The text `clm health` and `clm workaround` print, and the reconsider items `reconsider` writes, SHALL say component and components, never the old words, as `component-words` defines them: the summary noun of the `pending`, `oversized`, `design-heavy` and `outlier-size` sections is `components` (`health/pending: <n> components`), `outlier-size`'s reason without a median is `no median: fewer than 3 components with requirements`, and a reconsider item says deleting it without a spec change brings it back on the component's next build. The rules a `check` line names keep their spelling, and a workaround flag's file name keeps its form, `workaround-<component>-<slug>-<n>.md`. A test reads the string literals in `crates/clm-health/src/` and fails naming the file, the line and the word for each old word `component-words` doesn't keep.

#### Scenario: a pending component
- GIVEN component `a` with an uncited requirement
- WHEN `clm health` runs
- THEN its `pending` summary line is `health/pending: 1 components`, followed by the line for `specs/a/SPEC.md`

#### Serves
- architecture/plain-terms

### Requirement: component-size-field
The field of `Settings` that holds the size budgets SHALL be `component_size`, named as the `config` table the `clm` crate fills it from (`config/component-size-setting`), so the crate's own name for the budgets says component as the setting does (`plain-terms`). No field, variable or doc comment under `crates/clm-health/src/` SHALL keep the field's old name, which `clm/retired-terms`'s list holds once `clm/node-names-retired` is built.

#### Scenario: the budgets by their name
- GIVEN a `Settings` whose `component_size` gives a spec-token budget of 1
- WHEN `report` runs on a repository with a component whose `SPEC.md` is longer than 4 bytes
- THEN its `oversized` section names that component

#### Serves
- architecture/plain-terms

### Requirement: land-time-signal
`clm health` SHALL report a `land-time` signal, between `churn` and the signal after it, over the last 50 timed lands `ledger`'s `Fold::timed_lands(50)` gives: one line for each of the up to five steps, by name, with the most total seconds across them, most first and ties by name, naming its runs, its median and total seconds and up to three paths that most often selected it, most first and ties by path, and the summary line `<m> of <n> timed lands over <t>s`, `<t>` being `Settings`' `land_check_target_seconds`. With no timed land it SHALL print no finding line, and its summary SHALL count 0 lands.

#### Scenario: slow steps
- GIVEN a ledger with three timed lands over a target of 60 seconds, each with a step `web` of 100 seconds selected by `web/src/main.ts` and a step `fmt` of 1 second, and one under it with only `fmt`
- WHEN `clm health` runs
- THEN its `land-time` lines name `web` first, with 3 runs, a median of 100 and a total of 300 seconds and `web/src/main.ts`, then `fmt` with 4 runs, the summary is `3 of 4 timed lands over 60s`, and it exits 0

#### Scenario: no timings
- GIVEN a ledger with no timed `landed` record
- WHEN `clm health` runs
- THEN it prints no `health/land-time` finding line, and the signal's summary counts 0 lands

#### Needs
- ledger/landed-times

### Requirement: exemptions-signal
`clm health` SHALL report, under `exemption`, each exemption a component holds (`model/exemptions`), in component ID then written order, as `specs/<component path>/SPEC.md: health/exemption: <component>: <rule> until backlog/<slug>: <n> files`, `<n>` the number of tracked files the component owns (`check.coverage/owner`) that its `files` match, followed by `; item gone` when `backlog/<slug>.md` doesn't exist and `; matches nothing` when `<n>` is 0, so an exemption whose work is done, or whose files are gone, shows as one to remove (`temporary-exemptions`). The summary line SHALL count the exemptions, as `health/exemption: <n> exemptions`.

#### Scenario: exemption being paid down
- GIVEN component `a` owning `tests/a/x.py` and `tests/a/y.py`, with an exemption from `undeclared-imports` for `tests/a/**` until `backlog/r`, and `backlog/r.md`
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/exemption: a: undeclared-imports until backlog/r: 2 files`

#### Scenario: work done
- GIVEN the same exemption, with no `backlog/r.md` and no file under `tests/a/`
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/exemption: a: undeclared-imports until backlog/r: 0 files; item gone; matches nothing`

#### Serves
- architecture/exemptions-end
- architecture/health-advises

#### Needs
- model/exemptions
