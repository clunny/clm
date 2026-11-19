---
component: check.health
base: { frontmatter: "324bd48cb5417cc5", contract: "cf2912a2e0f286f2", design: "a86046105d49c25c" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [model, check, check.coverage, orchestrator, vcs, ledger, effort, gate]
```

## MODIFIED Contract
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
| `outside-contract` | code name in a requirement's statement that the component's Contract doesn't hold (`requirements-state-what-the-contract-shows`) |
| `test-placement` | requirement whose statement says where a test sits |
| `design-exit` | line of a Design that states an exit status |
| `uncited-test` | test in a file a component owns that holds no marker |

A requirement's *score* is defined by `reconsider-score`. Rewrites and counts alone never file anything: the report shows them, and a build's flag is what starts the count, so a long archive doesn't fill the human's queue. `clm rebase` files a `question` item for a requirement of the branch's component whose score reaches the threshold (`sync/file-reconsider-item`). Churn is reported and never scored: it is the same moves `rewritten` counts, seen from the code, so scoring both would count each built rewrite twice, and the score is computed without git, which `clm rebase` can't afford to read the whole history for (`churn-from-marker-history`).

`clm workaround <component>/<slug> --title <title>`, the situation on stdin, files a `workaround` backlog item naming the requirement at its current version (`model/parse-backlog`) and prints its path (`workaround-command`). A build session runs it when a requirement is hard to build in one of the ways the build prompt names (`orchestrator.run/prompt-workaround`). A flag is evidence, never a block: it changes no state the queue, the gate or `clm check` reads. It exits 3 when a capability the requirement serves has `[effort] workaround_flag` unsettled flags or more, telling the session to stop and ask whether to redesign it (`redesign-before-workaround`).

The library, for `sync` and the `clm` crate: `report(root, settings, expiry)` returns what `clm health` prints as `Report { sections }`, each section a signal with its lines; `signals(graph, items, settings)` returns each requirement's `RequirementSignals { id, workarounds, rewritten, scenarios, exceptions, score }` from a loaded graph, its backlog items (the map of slugs to items) and the settings the score compares with (`reconsider-score`), with no git; `reconsider(graph, items, settings, component)` returns the requirements of one component whose score reaches the threshold, each with the text of the item `clm rebase` files; `workaround(root, requirement, title, body)` writes a flag; `crowded(root, requirement, threshold)` gives each capability the requirement serves whose unsettled flags number `threshold` or more, with the count; `churn(root)` returns `None` when `HEAD` names no commit, and otherwise, for each requirement ID the commits reachable from `HEAD` ever cited with a version, whether the graph holds it or not, its distinct versions as `Churn { id, versions }`, reading git and nothing else. `Settings { reconsider, scenarios, exceptions, stale_days, lcov, .. }`, whose last field holds the size budgets (`Settings::component_size`), is this crate's own struct, which the `clm` crate fills from `config`'s `health` and `component_size` (the budgets as `clm_check::Budgets`), so `clm-health` doesn't depend on `clm-config`, as `clm-check` takes its detectors.

`clm-health` offers each of `clm health` and `clm workaround` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root and the text on standard input, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## MODIFIED Design
Crate `clm-health`, dispatched from `crates/clm` as `clm health` and `clm workaround`; this component's build edits `crates/clm/src/main.rs`, `Cargo.toml` and `Cargo.lock` for them, as the root's Design says. It reads a checkout: the graph through `clm-model`'s `load`, the sizes and `run` through `clm-check`, the markers, coverage, pending list and `marker_spans` from `clm-coverage` (`check-coverage-split`), the claim refs through `clm-orchestrator`'s `claim_refs`, the ledger's fold through `clm-ledger` and the unsettled flags through `clm-effort`'s `unsettled` (for `clm workaround`), and git through `clm-git`. It never builds for wasm. `--root` defaults as every `clm` command's does (`clm/root-from-cwd`). `--expiry` defaults to 4 hours, as `clm claim` does, so the two agree on which claims are lapsed.

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
- `churn`: `churn(root)` runs, through `clm-git`, `git -c diff.noprefix=false -c diff.mnemonicPrefix=false -c core.quotePath=false log -p --format= --no-color --no-renames --no-ext-diff --no-textconv` over the commits reachable from `HEAD`, the `-c` settings and the `--no-` options pinning the patch's shape against the machine's git configuration, and reads the output as patches: a `+++ ` line that directly follows a `--- ` line is a header, and names, as `+++ b/<path>`, the file the added lines after it belong to (`+++ /dev/null`, a deleted file, names none, and a path git quotes, one holding a control character or `"`, is skipped: no code file is named so); every other line starting with `+` is an added line, and its text after the `+` is handed to `clm-coverage`'s `marker_spans`, for every file but Markdown, so a marker counts whether or not it sits in a test, since a patch line shows no test around it (`churn-signal`), Markdown never does, and a marker without a version adds nothing; a removed line changes nothing, so a version once cited stays counted. The history isn't filtered by path: the scan costs the same with or without one (`clm health` and the test that runs it on this checkout pay for the whole history) and a path filter loses a file's history before a rename. A merge commit shows no patch, so a marker its resolution alone added isn't seen; the working tree's uncommitted markers aren't either. Before the log, `git rev-parse --verify --quiet HEAD` says whether there is a history: exit 1 means `HEAD` names no commit and `churn` returns `None`, any other failure is git failing, exit 2 as the Contract says; a repository with commits returns `Some` of every cited ID, the graph's and others', each `Churn { id, versions }` with `versions` the distinct 8-hex-digit versions without their `@`, sorted, and the entries sorted by `id`. The section: `health/churn: no history` with nothing listed for `None`; otherwise the summary counts `requirements`, and the lines, in the graph's order as `rewritten` lists them, are `specs/<component path>/SPEC.md: health/churn: <component>/<slug> cited at <n> versions` for each requirement the graph holds whose versions number 3 or more; an ID the graph doesn't hold is skipped, since its requirement isn't in the spec and `check/dangling-markers` covers a marker that outlives it.
- `many-scenarios`: `specs/<component path>/SPEC.md: health/many-scenarios: <component>/<slug> <n> scenarios`.
- `many-exceptions`: the requirement's text above its first `####` block, case-insensitively and at word boundaries (so `except` never matches `exception`), is searched for the crate's exception phrases, a constant like the history words, in a fenced block for the same reason: `specs/<component path>/SPEC.md: health/many-exceptions: <component>/<slug> <n> exception clauses`.

```
unless, except, but not, other than, only when, in which case, whichever, so also
```

- `reconsider`: the score (`reconsider-score`) is `f + r + s + e` with `f` the flags, `r` the archived rewrites, `s` one when the scenarios exceed `scenarios` and `e` one when the exception clauses exceed `exceptions`. `specs/<component path>/SPEC.md: health/reconsider: <component>/<slug> score <s> (workarounds <f>, rewritten <r>, scenarios <0|1>, exceptions <0|1>); filed as backlog/reconsider-<component>-<slug>.md`.

- `outside-contract`, `test-placement`, `design-exit` and `uncited-test` (`rule-signals`) count what a program can see of the rule for a requirement. Each is a heuristic and none is a fault by itself: a helper's unit test rightly cites nothing, and a Contract may be what is missing a name. A *code name* is read from a code span of the statement, outside fenced blocks: the span with one trailing parenthesised list taken off is identifiers joined by `::`, and the span held the list, or holds `::`, or holds a lowercase letter directly followed by an uppercase one, so `foreign_files(repo, base, head, component)`, `clm_gate::fingerprint_of` and `RequirementSignals` are code names and `clm.toml`, `human_burdens`, `origin/main` and `Settings` are not. The Contract holds a name when its text contains it. The sentences and the placement phrases are `clm-gate`'s `stating_sentences` and `places_a_test`, so the signal and the gate's count of a cut agree, and this component depends on `gate` for them alone. The exit pattern is `exit`, `exits` or `exited`, then optionally `with`, then optionally `status` or `code`, then a number, at word boundaries and without regard to case. A Contract is never read for it: a dependant's build receives the Contract and not the requirements, so a Contract keeps its summary of exits.

**Loading.** The graph can't be loaded only when `<root>/specs/SPEC.md` is missing, which exits 1; the violations `clm-model` reports for a tree it does load, a stale workaround flag among them, are findings under `check`, as `clm_check::run` returns them. The settings can't be loaded when `config`'s `load` rejects them, which the `clm` crate reports before calling the library, exit 1.

**Workaround flags** (`workaround-flags`, `workaround-command`). The file's `<n>` is the smallest positive integer whose file doesn't exist and whose slug no `decided` record settles (a settled slug stays settled, `effort/unsettled-flags`, so a flag filed after its requirement's earlier flags were deleted never inherits their settlement; a ledger that can't be read settles none), so flags on one requirement never overwrite each other and one build can file several. The command counts with `effort`'s `unsettled`, over the ledger's fold, so a flag a review decision settled doesn't count (`effort/unsettled-flags`). It commits nothing: the session commits the file with its work, as it does a `question` item. The model rejects a flag whose requirement isn't in the graph or whose version isn't current (`model/parse-backlog`), so the spec change that drops or rewrites the requirement deletes the flags, or records the new version when the workaround applies, and `clm check` tells it to.

**Reconsider items** (`sync/file-reconsider-item`). `reconsider(graph, items, settings, component)` scores the component's requirements from `signals` and returns, for each whose score reaches `settings.reconsider`, its ID and the item's text: `title: "Reconsider <component>/<slug>"`, `kind: question`, `components: [<component>]`, and a body as `reconsider-score` says. `clm rebase` writes it when `backlog/reconsider-<component>-<slug>.md` doesn't exist. The slug uses the component ID as written, dots included.

## ADDED Requirements

### Requirement: rule-signals
After `reconsider`, `clm health` SHALL report four signals of the rule for a requirement (`requirements-state-what-the-contract-shows`), each advisory:
- under `outside-contract`, for each requirement, each distinct code name in its statement, the text above its first `####` heading outside fenced blocks, that the component's Contract doesn't contain, as `specs/<component path>/SPEC.md: health/outside-contract: <component>/<slug> names <name>`, the summary counting `names`. A code name is the text of a code span with one trailing parenthesised list taken off, when that text is identifiers joined by `::` and the span held such a list, or holds `::`, or holds a lowercase letter directly followed by an uppercase one;
- under `test-placement`, each requirement whose statement holds a sentence `gate`'s `places_a_test` accepts, as `specs/<component path>/SPEC.md: health/test-placement: <component>/<slug> <phrase>`, the phrase the first found, the summary counting `requirements`;
- under `design-exit`, each line of a component's Design outside fenced blocks that holds `exit`, `exits` or `exited`, then optionally `with`, then optionally `status` or `code`, then a number, without regard to case, as `specs/<component path>/SPEC.md:<line>: health/design-exit: <the match>`, once per line, the summary counting `lines`. A Contract SHALL NOT be read for it;
- under `uncited-test`, each test `check.coverage`'s `uncited_tests` returns for the checkout's files in a file some component owns (`check.coverage/owner`), as `<file>:<line>: health/uncited-test: <name>`, the summary counting `tests`.

#### Scenario: a name the Contract doesn't hold
- GIVEN component `a` whose Contract names `select(graph)`, and whose requirement `a/x` says its command SHALL print what `select(graph)` returns, as `walk_up(path)` finds it, in the file `clm.toml` names
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/outside-contract: a/x names walk_up` and no `outside-contract` line naming `select` or `clm.toml`

#### Scenario: a requirement that places a test
- GIVEN requirement `a/y` whose statement ends `Every test of a's requirements SHALL sit in a's own files.`
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/test-placement: a/y SHALL sit`

#### Scenario: an exit in a Design
- GIVEN component `a` whose Design line 12 says `A missing file exits 1 with the path`, and whose Contract says `it exits 2 when git fails`
- WHEN `clm health` runs
- THEN it prints exactly one `design-exit` line, `specs/a/SPEC.md:12: health/design-exit: exits 1`

#### Scenario: a test that cites nothing
- GIVEN component `a` owning `crates/a/tests/t.rs`, which holds a test citing `a/x` and, at line 9, a test `helper_rounds_up` with no marker, and `vendor/v.rs`, owned by no component, holding a test with no marker
- WHEN `clm health` runs
- THEN it prints `crates/a/tests/t.rs:9: health/uncited-test: helper_rounds_up`, its summary is `health/uncited-test: 1 tests`, and no line names `vendor/v.rs`

#### Scenario: nothing found
- GIVEN a repository with none of these
- WHEN `clm health` runs
- THEN it prints `health/outside-contract: 0 names`, `health/test-placement: 0 requirements`, `health/design-exit: 0 lines` and `health/uncited-test: 0 tests`, and exits 0

#### Serves
- capability/know-health
- architecture/slim-components

#### Needs
- gate/cuts
- check.coverage/test-lists
