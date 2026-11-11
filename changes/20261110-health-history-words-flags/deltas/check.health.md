---
component: check.health
base: { design: "52ee94efb82adb1c" }
---

## MODIFIED Design
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


## MODIFIED Requirements
### Requirement: spec-bloat-signals <!-- base: f92bab085b0fed01 -->
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

