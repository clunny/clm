---
component: check.health
base: { contract: "275ca1129c79f6f2", design: "24c0c62fc8794ad1" }
---

## MODIFIED Contract
`clm health [--root <dir>] [--expiry <hours>]` prints one advisory report of the repository's health (`health-advises`): what `clm check`, `clm pending`, `clm size` and `clm status` each show a part of, the things nothing shows, and the signals that say a requirement or a component is fighting its implementation. It reads the working tree and git, prints to standard output one line per finding as `<path>: health/<signal>: <message>`, grouped by signal with one summary line per signal, and exits 0 whatever it finds; 1 when `specs/SPEC.md` is missing, the settings can't be loaded or standard output can't be written, and 2 when git fails, naming what failed on standard error (`health-report`). No rule of `clm check`, `clm gate` or `clm land` reads it. The signals:

| Signal | One line per |
|---|---|
| `check` | violation `clm_check::run` finds, as `<path>: health/check: <rule>: <message>`; no detectors run |
| `pending` | component with pending requirements, with how many are `unbuilt`, `behind` and `obsolete` |
| `oversized` | component over a size budget, with both counts (`oversized-is-a-state`) |
| `lapsed-claim` | `origin/impl/<component>` or `origin/propose/<slug>` whose tip is older than the expiry and not in `origin/main` |
| `stalled-change` | `applied` or `implementing` change outside the archive with requirements and none of them current |
| `unanswered` | `question` or `human-task` item added to git more than `stale_days` days ago |
| `uncited-decision` | accepted decision whose name no spec, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `AGENTS.md`, guide page, live change or other decision names in a code span |
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

`clm workaround <component>/<slug> --title <title> [--root <dir>]`, the situation on stdin, files a `workaround` backlog item naming the requirement at its current version (`model/parse-backlog`) and prints its path (`workaround-command`). A build session runs it when a requirement is hard to build in one of the ways the build prompt names (`orchestrator.run/prompt-workaround`). A flag is evidence, never a block: it changes no state the queue, the gate or `clm check` reads. It exits 3 when a capability the requirement serves has `[effort] workaround_flag` unsettled flags or more, telling the session to stop and ask whether to redesign it (`redesign-before-workaround`), and 1 when the requirement isn't in the graph, the body can't be read or the flag can't be written.

The library, each call taking a checkout or a loaded graph: `report(root, settings, expiry)` returns a `Report`, which prints through `Display` what `clm health` prints to standard output, or a `Failure` (`vcs/failure`, which the crate offers again as `Failure`), `Failure::Refused` when `specs/SPEC.md` is missing and `Failure::Git` when git fails; `signals(graph, items, settings)` returns each requirement's `RequirementSignals { id, workarounds, rewritten, scenarios, exceptions, score }` from a loaded graph, its backlog items (the map of slugs to items) and the settings the score compares with (`reconsider-score`), with no git; `reconsider(graph, items, settings, component)` returns, as `Reconsider { id, text }`, the requirements of one component whose score reaches the threshold, each with the text of the item `clm rebase` files, which `sync` calls; `workaround(root, requirement, title, body)` writes a flag and returns its path, or `Failure::Refused` for a requirement the graph doesn't hold or a flag it can't write; `churn(root)` returns `None` when `HEAD` names no commit, and otherwise, for each requirement ID the commits reachable from `HEAD` ever cited with a version, whether the graph holds it or not, its distinct versions as `Churn { id, versions }`, reading git and nothing else, or `Failure::Git` when git fails. `Settings { reconsider, scenarios, exceptions, stale_days, lcov, land_check_target_seconds, component_size }`, whose last field holds the size budgets (`Settings::component_size`), is this crate's own struct, which the `clm` crate fills from `config`'s `health`, `land` and `component_size` (the budgets as `clm_check::Budgets`), so `clm-health` doesn't depend on `clm-config`, as `clm-check` takes its detectors.

`clm-health` offers each of `clm health` and `clm workaround` as a call (`commands-in-crate`, `commands-live-in-their-crates`): `health_command(args, root, settings, env, out, err)` and `workaround_command(args, root, workaround_flag, stdin, env, out, err)`. Each takes the command's arguments as the command line gives them, the repository root the caller found, what the caller loaded for the command (for `clm health` its `Settings` or the `clm_check::Violation` that rejected them, for `clm workaround` its `[effort] workaround_flag` or why it couldn't be read), for `clm workaround` a reader of standard input, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. Their arguments are `HealthArgs { expiry, root }` and `WorkaroundArgs { requirement, title, root }`, which `crates/clm`'s parser reads. `crates/clm` routes the command to it and adds nothing to what it writes.

## MODIFIED Design
Crate `clm-health`, dispatched from `crates/clm` as `clm health` and `clm workaround`; this component's build edits `crates/clm/src/main.rs`, `Cargo.toml` and `Cargo.lock` for them, as the root's Design says. It reads a checkout: the graph through `clm-model`'s `load`, the sizes and `run` through `clm-check`, the markers, coverage, pending list, change progress and `marker_spans` from `clm-coverage` (`check-coverage-split`), the claim refs through `clm-orchestrator`'s `claim_refs`, the ledger's fold through `clm-ledger` and the unsettled flags through `clm-effort`'s `unsettled` (for `clm workaround`), and git through `clm-git`. It never builds for wasm. `--root` defaults as every `clm` command's does (`clm/root-from-cwd`). `--expiry` defaults to the hours `clm claim` takes, so the two agree on which claims are lapsed.

**Output.** Sections print in the order of the Contract's table, each led by its summary line (`health-report`); a section with nothing prints only its summary, so a reader sees that it ran, and a signal that can't run gives its reason in place of a count. A token is four bytes, rounded up, as `check/node-size` counts.

**The signals.**
- `check`: `clm_check::run(root)` with the markers scanned at the root. The command never runs detectors: they need a toolchain, and `clm check` is where they fail.
- `pending`: `clm_coverage::pending` over the same markers, counted by component; a component the graph no longer holds has no `SPEC.md` of its own, so its path is built from its ID.
- `oversized`: `clm_check::size` with the budgets `Settings` carries.
- `lapsed-claim`: `clm-orchestrator`'s `claim_refs(root)` for the component claims and `propose_refs(root)` for the backlog claims, on the refs as fetched, without fetching (the caller fetched, or sees the last fetch, as `clm status` does), each ref carrying its tip time and whether it is merged. Whether there is an `origin` is judged first, through `clm-git`, by whether `refs/remotes/origin/main` exists.
- `stalled-change`: `clm_coverage::change_progress`.
- `unanswered`: for each `question` or `human-task` item, `git log --diff-filter=A --format=%ct -- backlog/<slug>.md` through `clm-git`, the earliest when several. Without a commit no item was added to git and `git log` can't run, so no item is read. `now` is passed in, so tests are deterministic.
- `uncited-decision`: the decisions `clm-model` loads, `status: accepted` only (a superseded one is history), each cited as `uncited-decisions` says, its code spans read through `clm_check::code_spans`; the crate keeps no span reader of its own. The Markdown sources are found by walking their directories without following a symlinked directory, so a link to an ancestor can't loop.
- `unserved`: each entry of the human layers, architecture first, each layer's in file order, that no rule's effective serves (`Graph::effective_serves`, a `#### Serves` block or its component's `serves`) names. It is advisory: an entry may be proven end to end by its own tests alone.
- `history-words`: a scenario may legitimately give a date, which is why requirements and scenarios are never read; the phrase list sits in `spec-bloat-signals`' statement, which the signal therefore never reads. A phrase goes on the list only when it narrates a change wherever it appears: "no longer" stays off it, since it as often states a present condition (a local branch that no longer contains its remote tip, a tree the session no longer has), and so does "used to", since it is as often a passive that describes the present system (a ref that is used to hold the claim, a file that may be used to override a setting); the matcher reads no grammar, so a phrase that is history only in some of its forms is dropped rather than parsed (`yagni`), and a signal that advises costs reader time and invites wrong cuts each time it misfires (`health-advises`).
- `design-heavy`: the Requirements section's bytes are counted as the model stores a section's text.
- `outlier-size`: with `clm_check::size`'s counts.
- `unexercised`: the file is read whole as text; the repository root is compared with an `SF:` path both as given and canonicalised, so a path under a symlinked root still reads as relative.
- `rewritten`: the archived changes (`changes/archive/*/deltas/<component>.md`) whose `## MODIFIED Requirements` holds `### Requirement: <slug>`, read with `clm-model`'s delta parsing. A delta that fails to parse counts nothing, as it adds nothing for `blocks`.
- `churn`: `churn(root)` runs, through `clm-git`, `git -c diff.noprefix=false -c diff.srcPrefix=a/ -c diff.dstPrefix=b/ -c log.showRoot=true -c diff.mnemonicPrefix=false -c core.quotePath=false log -p --format= --no-color --no-renames --no-ext-diff --no-textconv` over the commits reachable from `HEAD`, the `-c` settings and the `--no-` options pinning the patch's shape against the machine's git configuration, and reads the output as patches: a `+++ ` line that directly follows a `--- ` line is a header, and names, as `+++ b/<path>`, the file the added lines after it belong to (`+++ /dev/null`, a deleted file, names none; git ends the header of a path holding a space with a tab, which is taken off; a path git quotes, `+++ "b/<path>"`, is unquoted as git quotes it, its C-style escapes and octal bytes read back); every other line starting with `+` is an added line, and its text after the `+` is handed to `clm-coverage`'s `marker_spans`. A removed line changes nothing, so a version once cited stays counted. The history isn't filtered by path: the scan costs the same with or without one (`clm health` and the test that runs it on this checkout pay for the whole history) and a path filter loses a file's history before a rename. A merge commit shows no patch, so a marker its resolution alone added isn't seen; the working tree's uncommitted markers aren't either. Before the log, `git rev-parse --verify --quiet HEAD` says whether there is a history: its status of 1 means `HEAD` names no commit and `churn` returns `None`, and any other failure is git failing (`health-report`). The entries are sorted by `id`, each `versions` sorted, and the section lists them in the graph's order, as `rewritten` does.
- `many-exceptions`: the exception phrases are a constant of the crate, these:

```
unless, except, but not, other than, only when, in which case, whichever, so also
```

- `reconsider`: the score (`reconsider-score`) is `f + r + s + e` with `f` the flags, `r` the archived rewrites, `s` one when the scenarios exceed `scenarios` and `e` one when the exception clauses exceed `exceptions`.

- `outside-contract`, `test-placement`, `design-exit` and `uncited-test` (`rule-signals`) count what a program can see of the rule for a requirement. Each is a heuristic and none is a fault by itself: a helper's unit test rightly cites nothing, and a Contract may be what is missing a name. A statement's code spans are read through `clm_check::code_spans`, as `uncited-decision`'s are. By `rule-signals`' reading, `foreign_files(repo, base, head, component)`, `clm_gate::fingerprint_of` and `RequirementSignals` are code names and `clm.toml`, `human_burdens`, `origin/main` and `Settings` are not. The sentences and the placement phrases are `clm-gate`'s `sentences` and `places_a_test`, so a sentence the signal reports is one the gate doesn't count as a cut when a branch drops it, and this component depends on `gate` for them alone; a statement's fenced lines, which the gate reads as part of the statement, are left out here as they are for `outside-contract`. A Contract is never read for `design-exit`: a dependant's build receives the Contract and not the requirements, so a Contract keeps its summary of exits.

**Loading.** The graph loads whenever `<root>/specs/SPEC.md` exists; the violations `clm-model` reports for a tree it loads, a stale workaround flag among them, are findings under `check`, as `clm_check::run` returns them. The settings are `config`'s, which the `clm` crate loads and hands to the call, or hands the violation that rejected them, which the call prints (`health-report`).

**Workaround flags** (`workaround-flags`, `workaround-command`). The file's `<n>` is the smallest positive integer whose file doesn't exist and whose slug no `decided` record settles (a settled slug stays settled, `effort/unsettled-flags`, so a flag filed after its requirement's earlier flags were deleted never inherits their settlement; a ledger that can't be read settles none), so flags on one requirement never overwrite each other and one build can file several. The file is created only when absent, so two builds flagging at once never overwrite a flag. The command counts with `effort`'s `unsettled`, over the ledger's fold, so a flag a review decision settled doesn't count (`effort/unsettled-flags`); it reads the ledger whether or not the requirement serves a capability, so a ledger that can't be read is always reported. It commits nothing: the session commits the file with its work, as it does a `question` item. The model rejects a flag whose requirement isn't in the graph or whose version isn't current (`model/parse-backlog`), so the spec change that drops or rewrites the requirement deletes the flags, or records the new version when the workaround applies, and `clm check` tells it to.

**Reconsider items** (`sync/file-reconsider-item`). `reconsider(graph, items, settings, component)` scores the component's requirements from `signals` and returns, for each whose score reaches `settings.reconsider`, its ID and the item's text: `title: "Reconsider <component>/<slug>"`, `kind: question`, `components: [<component>]`, and a body as `reconsider-score` says. `clm rebase` writes it when `backlog/reconsider-<component>-<slug>.md` doesn't exist. The slug uses the component ID as written, dots included.

## MODIFIED Requirements

### Requirement: health-report <!-- base: a5dd24b1f06ef8c3 -->
`clm health` SHALL print to standard output, for each signal in the Contract's table in that order, a summary line `health/<signal>: <summary>` followed by one line per finding as `<path>: health/<signal>: <message>`, and SHALL exit 0 whatever it finds. `<summary>` SHALL be `<n> <noun>`, `<n>` the number of the section's finding lines where the signal's requirement gives no other count or summary, and `<noun>` by signal: `violations` for `check`; `components` for the four signals `says-component` names; `claims` for `lapsed-claim`; `changes` for `stalled-change`; `items` for `unanswered`; `decisions` for `uncited-decision`; `entries` for `unserved`; `lines` for `history-words`, `unexercised` and `design-exit`; `flags` for `workaround`; `exemptions` for `exemption`; `names` for `outside-contract`; `tests` for `uncited-test`; and `requirements` for `rewritten`, `churn`, `many-scenarios`, `many-exceptions`, `reconsider` and `test-placement`. Violations the model reports for a tree it loads SHALL be findings under `check`, as `<path>: health/check: <rule>: <message>`. It SHALL NOT run detectors. It SHALL exit 1 when the caller passes a violation in place of the settings, printing `clm health: ` and the violation as `clm check` prints one, `<path>: <rule>: <message>`, before it reads the tree; 1 when `specs/SPEC.md` under the root is missing, printing `clm health: <root>: specs/SPEC.md not found`; and 2 when git fails, printing `clm health: ` and what failed; each on standard error, with nothing on standard output. When standard output can't be written it SHALL print `clm health: ` and why to standard error and exit 1. The library's `report(root, settings, expiry)` SHALL return the `Report` whose `Display` text is what `clm health` prints to standard output, `Failure::Refused` when `specs/SPEC.md` is missing and `Failure::Git` when git fails, each with the message the command prints after `clm health: `.

#### Scenario: a clean repository
- GIVEN this repository's own checkout
- WHEN `clm health` runs
- THEN it prints a line starting `health/check:` and a line starting `health/reconsider:`, and exits 0

#### Scenario: findings don't fail it
- GIVEN a repository with a marker citing a requirement that doesn't exist, and a detector `bad` (`["sh", "-c", "touch ran; exit 3"]`) in its `clm.toml`
- WHEN `clm health` runs
- THEN it prints a line containing `health/check: check/dangling-markers:`, exits 0, and no file `ran` exists

#### Scenario: settings can't be loaded
- GIVEN a directory with no `specs/SPEC.md`, and settings the caller couldn't load: a violation of `clm.toml` naming `health`, as the caller passes when `config` rejects `reconsider = 0` under `[health]`
- WHEN `clm health` runs
- THEN it exits 1 naming `clm.toml` and `health`

#### Scenario: every summary
- GIVEN a git repository with no commit, no remote and no file but a `specs/SPEC.md` the model loads without a violation, and settings that set no `lcov` and a land target of 120 seconds
- WHEN `clm health` runs
- THEN it prints exactly these lines, in this order, then the four summaries scenario `nothing found` shows, and exits 0: `health/check: 0 violations`, `health/pending: 0 components`, `health/oversized: 0 components`, `health/lapsed-claim: no origin`, `health/stalled-change: 0 changes`, `health/unanswered: 0 items`, `health/uncited-decision: 0 decisions`, `health/unserved: 0 entries`, `health/history-words: 0 lines`, `health/design-heavy: 0 components`, `health/outlier-size: no median: fewer than 3 components with requirements`, `health/unexercised: no lcov file configured`, `health/workaround: 0 flags`, `health/exemption: 0 exemptions`, `health/rewritten: 0 requirements`, `health/churn: no history`, `health/land-time: 0 of 0 timed lands over 120s`, `health/many-scenarios: 0 requirements`, `health/many-exceptions: 0 requirements`, `health/reconsider: 0 requirements`

#### Scenario: refused
- GIVEN in turn: a directory with no `specs/SPEC.md`; a directory holding `specs/SPEC.md` that isn't a git repository; and a repository whose report the output stream refuses to take
- WHEN `clm health` runs on each, and `report` is called on the first two
- THEN it exits 1 with `clm health: <root>: specs/SPEC.md not found` on standard error for the first; 2 for the second; and 1 with `clm health: ` and the stream's error on standard error for the third; printing nothing to standard output for the first two; and `report` returns `Failure::Refused` for the first and `Failure::Git` for the second

#### Needs
- config/health-setting

### Requirement: state-signals <!-- base: 1f6b7a1b1018d4aa -->
`clm health` SHALL report each component with pending requirements under `pending` as `specs/<component path>/SPEC.md: health/pending: <n> pending (<a> unbuilt, <b> behind, <c> obsolete)`, a component no longer in the spec whose markers are `obsolete` at `specs/<its ID, each dot made a slash>/SPEC.md`; each oversized component under `oversized` with both counts against their budgets, as `specs/<component path>/SPEC.md: health/oversized: spec <n>/<budget>, code <n>/<budget>`; each claim ref, `origin/impl/<component>` or `origin/propose/<slug>`, whose tip is older than the expiry, `--expiry` hours or 4 when it isn't given, and not an ancestor of `origin/main` under `lapsed-claim`, as `impl/<component>: health/lapsed-claim: last pushed <h>h ago` or `propose/<slug>: health/lapsed-claim: last pushed <h>h ago`, `<h>` the whole hours since its tip; each `applied` or `implementing` change outside the archive with one or more requirements and none of them current under `stalled-change`, as `changes/<id>/change.md: health/stalled-change: 0/<total> requirements current`; and each `question` or `human-task` item whose file the earliest commit reachable from `HEAD` that added it added more than `stale_days` days ago under `unanswered`, as `backlog/<slug>.md: health/unanswered: <kind> open <d> days`, `<d>` the whole days since that commit, skipping an item no commit added. When `refs/remotes/origin/main` doesn't exist, the `lapsed-claim` summary SHALL be `health/lapsed-claim: no origin` and it SHALL list no claim.

#### Scenario: pending and oversized
- GIVEN component `a` with requirement `a/x` nothing cites, and component `b` whose tracked code is 8000 bytes against a code default of 1000 tokens
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/pending: 1 pending (1 unbuilt, 0 behind, 0 obsolete)` and `specs/b/SPEC.md: health/oversized: spec <n>/12000, code 2000/1000`

#### Scenario: lapsed claim
- GIVEN `origin/impl/a` whose tip was committed 10 hours ago and isn't in `origin/main`, and `origin/impl/b` committed 1 hour ago
- WHEN `clm health --expiry 4` runs
- THEN it prints `impl/a: health/lapsed-claim: last pushed 10h ago` and no line for `impl/b`

#### Scenario: the default expiry
- GIVEN `origin/propose/q` whose tip was committed 5 hours ago and isn't in `origin/main`
- WHEN `clm health` runs with no `--expiry`
- THEN it prints `propose/q: health/lapsed-claim: last pushed 5h ago`

#### Scenario: stalled change
- GIVEN an `applied` change `20260101-c` whose delta adds `a/x` and `a/y`, neither cited, and an `applied` change `20260102-d` whose added requirement is current
- WHEN `clm health` runs
- THEN it prints `changes/20260101-c/change.md: health/stalled-change: 0/2 requirements current` and no `stalled-change` line for `20260102-d`

#### Scenario: a removed component and an empty change
- GIVEN an `applied` change `20260105-r` whose delta removes component `b.c`, which held requirement `x`, and adds or modifies no requirement, and a test citing `b.c/x`
- WHEN `clm health` runs
- THEN it prints `specs/b/c/SPEC.md: health/pending: 1 pending (0 unbuilt, 0 behind, 1 obsolete)` and no `stalled-change` line for `20260105-r`

#### Scenario: unanswered question
- GIVEN `backlog/q.md` of kind `question` added by a commit 45 days ago, then deleted and added again by a commit 2 days ago, `backlog/h.md` of kind `human-task` added 3 days ago, and an untracked `backlog/u.md` of kind `question`
- WHEN `clm health` runs with the default `stale_days`
- THEN it prints `backlog/q.md: health/unanswered: question open 45 days` and no `unanswered` line for `h` or `u`

#### Needs
- model/parse-backlog

### Requirement: uncited-decisions <!-- base: 4dc76b1491924a9d -->
`clm health` SHALL report each accepted decision whose name is the whole text of no code span in any component's `SPEC.md`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `AGENTS.md`, Markdown file (`*.md`) under `docs/guide/`, Markdown file under `changes/` outside `changes/archive/`, or other decision, as `docs/decisions/<name>.md: health/uncited-decision: nothing names it`. Code spans, fenced code blocks among what holds none, are read as `check/asset-references` reads them; a span names a decision when its whole text equals the decision's name, compared with case, so a span that holds the name with anything else, such as `docs/decisions/<name>.md` or the name with a suffix, doesn't. A source that is missing or isn't UTF-8 text SHALL name nothing. A superseded decision SHALL NOT be reported.

#### Scenario: cited and uncited
- GIVEN decisions `kept` (named as `` `kept` `` in `specs/a/SPEC.md`), `lone` (named only as `` `docs/decisions/lone.md` `` and `` `lone-ish` `` in `ARCHITECTURE.md`; as `` `lone` `` in a `CLAUDE.md` whose bytes aren't UTF-8; and in `AGENTS.md` on a line holding `` `other` ``, then a single backtick directly before `lone` and no backtick after it in its block) and `old` (`status: superseded`, named nowhere), and no `CAPABILITIES.md` or `docs/guide/`
- WHEN `clm health` runs
- THEN it prints `docs/decisions/lone.md: health/uncited-decision: nothing names it` and no `uncited-decision` line for `kept` or `old`

#### Scenario: named only in a fence
- GIVEN decision `fenced`, whose only mention is `` `fenced` `` inside a ```` ``` ```` block of `specs/a/SPEC.md`
- WHEN `clm health` runs
- THEN it prints an `uncited-decision` line for `fenced`

#### Needs
- check/asset-references

### Requirement: spec-bloat-signals <!-- base: 6a8806e4049653ee -->
`clm health` SHALL report, under `history-words`, each line of a component's Contract or Design outside fenced code blocks that holds, anywhere, a date (`20`, two ASCII digits, `-`, two ASCII digits, `-`, two ASCII digits), or one of the phrases `as before`, `today's`, `amended`, `predates`, `formerly`, `previously`, `was removed` and `is gone`, compared without regard to ASCII case, its words one space apart, with no ASCII letter or digit directly before or after it, as `specs/<component path>/SPEC.md:<line>: health/history-words: <match>`, `<match>` the line's leftmost date or phrase as written, once per line; under `design-heavy`, each component with at least one requirement or a Contract holding more than white space whose Design has more tokens than its Contract and Requirements sections together, counted as one, as `specs/<component path>/SPEC.md: health/design-heavy: design <n> tokens, contract and requirements <m>`; and under `outlier-size`, each component whose code tokens or spec tokens per requirement, the count divided by its number of requirements and rounded down, are more than twice the median of that ratio across the components with at least one requirement, the lower middle value for an even count, when there are at least three such components, as `specs/<component path>/SPEC.md: health/outlier-size: <code|spec> <r> tokens per requirement, median <m>`, one line for each ratio that is, `code` first, the summary counting components. Requirement and scenario text SHALL NOT be read for `history-words`.

#### Scenario: history in a design
- GIVEN component `a` whose Design line 14 says `The old path, removed 2026-10-01, no longer runs`, and whose requirement `a/x` has a scenario mentioning `2026-10-01`
- WHEN `clm health` runs
- THEN it prints exactly one `history-words` line for `a`, `specs/a/SPEC.md:14: health/history-words: 2026-10-01`

#### Scenario: a phrase in any case
- GIVEN component `a` whose Contract lines 9, 10 and 11 say `It Previously ran.`, `The refused run stays.` and `Today's build.`, and no other line of its Contract or Design holds a date or a history phrase
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md:9: health/history-words: Previously` and `specs/a/SPEC.md:11: health/history-words: Today's`, and no `history-words` line for line 10

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

#### Scenario: nothing to weigh
- GIVEN component `a` with a 2000-byte Design, a Contract of white space alone and no requirement
- WHEN `clm health` runs
- THEN it prints no `design-heavy` line for `a`

#### Scenario: outlier
- GIVEN components `a`, `b` and `c` with one requirement each and 1000, 1200 and 1100 bytes of code, and component `d` with one requirement and 8000 bytes of code, the four `SPEC.md` files of equal size
- WHEN `clm health` runs
- THEN it prints an `outlier-size` line for `d` naming `code` and `median 275` (the lower middle of 250, 275, 300 and 2000), and none for `a`, `b` or `c`

#### Scenario: over on both counts
- GIVEN components `a`, `b` and `c` with one requirement each and 1000, 1200 and 1100 bytes of code, and component `d` with one requirement, 8000 bytes of code and a `SPEC.md` eight times the size of each of theirs, theirs of equal size
- WHEN `clm health` runs
- THEN it prints two `outlier-size` lines for `d`, the one naming `code` and then the one naming `spec`, and the summary `health/outlier-size: 1 components`

#### Serves
- capability/know-health
- architecture/slim-components
- architecture/spec-is-current

### Requirement: unexercised-code <!-- base: 2c85f1a23bb2cb05 -->
When the settings set `lcov` and the file it names, joined to the repository root, can be read as UTF-8 text, `clm health` SHALL read it as lcov and report, for each file some component owns (`check.coverage/owner`), each run of consecutive line numbers whose count is 0 as `<path>:<start>-<end>: health/unexercised: <n> lines no test runs`, `<path>:<start>` alone for a run of one line, the summary counting lines. Each line of the file is read with its leading and trailing white space taken off: `SF:<path>` SHALL start a file's record, `DA:<line>,<count>` give a line's count, and `end_of_record` end the record; every other line, and a `DA:` line outside a record or whose line or count isn't ASCII digits, optionally after one `+`, or is larger than the largest 64-bit unsigned number, SHALL be skipped; a third field after a further comma SHALL be ignored. The counts the records give one line of one file SHALL be added, a sum past the largest 64-bit unsigned number staying at that number, so a line any record ran isn't reported. An `SF:` path SHALL be read relative to the repository root: one that is absolute and under the root, the root made absolute or with its symbolic links resolved, with the root taken off; one that is relative as written; and one that is absolute and outside the root as written, so no component owns it. With `lcov` unset it SHALL print `health/unexercised: no lcov file configured`, and with the file missing or not readable as text `health/unexercised: <lcov> not found`, `<lcov>` the setting as written, and in both cases report nothing else for the signal and still exit 0.

#### Scenario: a run of unexercised lines
- GIVEN `lcov = "cov.info"` whose record for `crates/a/src/lib.rs` (owned by component `a`) has `DA:3,1`, `DA:4,0`, `DA:5,0`, `DA:6,2` and `DA:9,0`, and a record for `vendor/x.rs` (owned by no component) with `DA:1,0`
- WHEN `clm health` runs
- THEN it prints `crates/a/src/lib.rs:4-5: health/unexercised: 2 lines no test runs` and `crates/a/src/lib.rs:9: health/unexercised: 1 lines no test runs`, its summary says `3 lines`, and no line names `vendor/x.rs`

#### Scenario: records merged
- GIVEN a repository `r` with component `a` owning `crates/a/**`, and `lcov = "cov.info"` holding: for `crates/a/src/lib.rs`, named by its absolute path under `r`, a record with `DA:4,0` and `DA:8,0,abc` and another with `DA:4,3`, `DA:x,0`, `DA:9,` and `DA:+10,0`; a record for `crates/a/src/b.rs`, named so, with `DA:2,0`; a record for `/elsewhere/crates/a/src/b.rs` with `DA:3,0`; a record for `r/crates/a/src/c.rs` with `DA:1,0`; and `DA:7,0` before any `SF:` line
- WHEN `clm health --root r` runs from the directory holding `r`
- THEN it prints `crates/a/src/lib.rs:8: health/unexercised: 1 lines no test runs`, `crates/a/src/lib.rs:10: health/unexercised: 1 lines no test runs` and `crates/a/src/b.rs:2: health/unexercised: 1 lines no test runs` and no other `unexercised` line, and its summary says `3 lines`

#### Scenario: no file
- GIVEN `lcov = "cov.info"` and no such file
- WHEN `clm health` runs
- THEN it prints `health/unexercised: cov.info not found` and exits 0

#### Serves
- capability/know-health
- architecture/proven-by-tests

#### Needs
- config/health-setting

### Requirement: requirement-signals <!-- base: 55b8d0c4aba9f69f -->
`clm health` SHALL report each `workaround` backlog item under `workaround`, as `backlog/<slug>.md: health/workaround: <component>/<slug> <title>`; each requirement that two or more archived changes modified, each with a delta for its component whose `## MODIFIED Requirements` holds it, under `rewritten`, as `specs/<component path>/SPEC.md: health/rewritten: <component>/<slug> modified by <n> archived changes`; each requirement with more scenarios than `scenarios` under `many-scenarios`, as `specs/<component path>/SPEC.md: health/many-scenarios: <component>/<slug> <n> scenarios`; and each requirement whose text below its heading and above its first line that starts `####`, fenced lines included, holds more occurrences than `exceptions` of the exception phrases the Design lists under `many-exceptions`, as `specs/<component path>/SPEC.md: health/many-exceptions: <component>/<slug> <n> exception clauses`, each phrase compared without regard to ASCII case, its words one space apart, with no ASCII letter or digit directly before or after it, so `except` never matches `exception`, and the search going on after each occurrence it counts.

#### Scenario: workaround and rewrites
- GIVEN `backlog/workaround-a-x-1.md` naming `a/x` with title `Faked the clock`, and archived changes `20260101-p` and `20260102-q` whose deltas for `a` both list `x` under `## MODIFIED Requirements`, and archived change `20260103-r` whose delta modifies `a/y` only
- WHEN `clm health` runs
- THEN it prints `backlog/workaround-a-x-1.md: health/workaround: a/x Faked the clock` and `specs/a/SPEC.md: health/rewritten: a/x modified by 2 archived changes`, and no `rewritten` line for `a/y`

#### Scenario: scenarios and exceptions
- GIVEN `scenarios = 2` and `exceptions = 1` under `[health]`, requirement `a/x` with three scenarios, and requirement `a/y` whose text says `... Unless the file is missing, EXCEPT on Windows; an exception` and whose only scenario mentions `unless` twice
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/many-scenarios: a/x 3 scenarios` and `specs/a/SPEC.md: health/many-exceptions: a/y 2 exception clauses`

#### Serves
- capability/know-health
- architecture/redesign-before-workaround

#### Needs
- model/parse-backlog
- config/health-setting

### Requirement: churn-signal <!-- base: 497eb239fb4ba36c -->
`clm health` SHALL report, under `churn`, each requirement of the graph whose ID the commits reachable from `HEAD` cite at three or more distinct versions, as `specs/<component path>/SPEC.md: health/churn: <component>/<slug> cited at <n> versions`, counting the versioned markers `check.coverage/marker-spans` finds in the added lines of every commit's patch, in every file whose name's extension isn't `md`, compared with case, whatever its path and whether or not the marker sits in a test, since a patch line shows no test around it. It SHALL NOT count a marker without a version, a marker in a Markdown file, or a marker no commit holds, and SHALL skip an ID no component holds. The count SHALL be the same whatever the repository's git configuration sets for diff prefixes (`diff.noprefix`, `diff.srcPrefix`, `diff.dstPrefix`, `diff.mnemonicPrefix`), path quoting (`core.quotePath`), colour, rename detection, external diff drivers, text conversion and `log.showRoot`, and a file whose path git quotes in a patch, one holding `"` or a control character, SHALL count as any other. When `HEAD` names no commit its summary SHALL be `health/churn: no history`, with nothing listed. The library's `churn(root)` SHALL return every cited ID, held by a component or not, with its distinct versions, sorted, the IDs sorted; `None` when `HEAD` names no commit; and `Failure::Git` when git fails.

#### Scenario: re-cited across commits and a rename
- GIVEN component `a` with requirements `x` and `y`, and a history in which `tests/old.rs` cites `req:a/x@11111111`, then `req:a/x@22222222`, then is renamed to `tests/new.rs` where it cites `req:a/x@33333333`, and `tests/y.rs` cites `req:a/y@aaaaaaaa`, then `req:a/y@bbbbbbbb`
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/churn: a/x cited at 3 versions` and no `churn` line for `a/y`

#### Scenario: what doesn't count
- GIVEN component `a` with requirement `z`, a history whose commits cite `req:a/z@cccccccc` and then `req:a/z@dddddddd` in `tests/z.rs`, `req:a/z@eeeeeeee` in `notes.md` only, and `req:a/z` without a version in `tests/z.rs`, and cite `a/gone`, which no component holds, at four versions in `tests/gone.rs`; and an uncommitted edit of `tests/z.rs` that cites `req:a/z@ffffffff`
- WHEN `clm health` runs, and `churn` is called for the same root
- THEN it prints no `churn` line for `a/z` or `a/gone`, and `churn` returns `a/z` with the two versions `cccccccc` and `dddddddd` and `a/gone` with four

#### Scenario: no history
- GIVEN a git repository with no commit, and a directory that isn't a git repository
- WHEN `churn` is called on each
- THEN it returns `None` for the repository and `Failure::Git` for the directory

#### Scenario: the patch's shape
- GIVEN component `a` with requirement `x`, a repository whose git configuration sets `diff.noprefix` to `true`, `diff.dstPrefix` to `new/`, `log.showRoot` to `false`, `color.ui` to `always` and `core.quotePath` to `true`, and a root commit that adds `req:a/x@11111111` in `tests/my test.rs`, `req:a/x@22222222` in `tests/a"b.rs`, `req:a/x@33333333` in `tests/é.rs` and `req:a/x@44444444` in `docs/my notes.md`
- WHEN `churn` is called, and `clm health` runs
- THEN `churn` returns `a/x` with the versions `11111111`, `22222222` and `33333333`, and `clm health` prints `specs/a/SPEC.md: health/churn: a/x cited at 3 versions`

#### Needs
- check.coverage/marker-spans

### Requirement: workaround-command <!-- base: d9c5203d0411475d -->
`clm workaround <component>/<slug> --title <title>` SHALL read the body from stdin and write `backlog/workaround-<component>-<slug>-<n>.md`, `<n>` the smallest positive integer whose file doesn't exist and whose slug no `decided` record's `settles` lists, a ledger that can't be read settling none, with `title` as a quoted string that reads back as given, `kind: workaround`, `components: [<component>]` (`model/component-keys`), `requirement: <component>/<slug>@<version>` at the requirement's current version, and the body, and print its path. It SHALL refuse, exit 1 and write nothing, for a requirement the graph doesn't hold, naming it. It SHALL NOT commit. After writing the flag, when some capability its requirement serves has at least `workaround_flag` unsettled flags (`config/effort-setting`, `effort/unsettled-flags`), this one included, it SHALL print to stderr `clm workaround: warning: <n> unsettled workarounds on capability/<slug>: stop and ask whether to redesign it (redesign-before-workaround)` for each such capability, and exit 3, the flag written. When the settings or the ledger can't be read it SHALL warn on stderr `clm workaround: warning: the workarounds weren't counted: <why>`, and exit 0, whether or not its requirement serves a capability. When standard input can't be read it SHALL print to stderr `clm workaround: reading the body: <why>`, write nothing and exit 1; when the flag can't be written, because `backlog/` can't be made or the file can't be created or written, `clm workaround: <path>: <why>` and exit 1; and when the flag's path can't be written to standard output, `clm workaround: <why>` and exit 1, the flag left written and no flag counted.

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

#### Scenario: nothing to count
- GIVEN component `a` whose requirement `a/y` serves no capability, in a directory that isn't a git repository, so its ledger can't be read
- WHEN `clm workaround a/y --title "x"` runs
- THEN it prints `backlog/workaround-a-y-1.md`, prints `clm workaround: warning: the workarounds weren't counted: ` and why on stderr, and exits 0

#### Scenario: the body can't be read or written
- GIVEN component `a` with requirement `a/x`, and in turn: a standard input whose read fails; a file named `backlog` where the directory would be; and an output stream that refuses writes
- WHEN `clm workaround a/x --title "x"` runs on each
- THEN it exits 1 each time: with `clm workaround: reading the body: ` and why on stderr and no flag written for the first; with `clm workaround: backlog: ` and why on stderr for the second; and with `clm workaround: ` and why on stderr for the third, `backlog/workaround-a-x-1.md` written

#### Serves
- capability/know-health
- architecture/redesign-before-workaround

#### Needs
- model/parse-backlog
- model/component-keys

### Requirement: says-component <!-- base: 15cdb4facc3238f4 -->
The text `clm health` and `clm workaround` print, and the reconsider items `reconsider` writes, SHALL say component and components, never the old words, as `component-words` defines them: the summary noun of the `pending`, `oversized`, `design-heavy` and `outlier-size` sections is `components` (`health/pending: <n> components`), `outlier-size`'s reason without a median is `no median: fewer than 3 components with requirements`, and a reconsider item says deleting it without a spec change brings it back on the component's next build. The rules a `check` line names keep their spelling, and a workaround flag's file name keeps its form, `workaround-<component>-<slug>-<n>.md`.

#### Scenario: a pending component
- GIVEN component `a` with an uncited requirement
- WHEN `clm health` runs
- THEN its `pending` summary line is `health/pending: 1 components`, followed by the line for `specs/a/SPEC.md`

#### Serves
- architecture/plain-terms

### Requirement: component-size-field <!-- base: 7cb5ae482ff979bd -->
The field of `Settings` that holds the size budgets `oversized` compares against SHALL be `component_size`, named as the `config` table the `clm` crate fills it from (`config/component-size-setting`), so the crate's own name for the budgets says component as the setting does (`plain-terms`).

#### Scenario: the budgets by their name
- GIVEN a `Settings` whose `component_size` gives a spec-token budget of 1
- WHEN `report` runs on a repository with a component whose `SPEC.md` is longer than 4 bytes
- THEN its `oversized` section names that component

#### Serves
- architecture/plain-terms

### Requirement: land-time-signal <!-- base: ba4f73a60b5ae630 -->
`clm health` SHALL report a `land-time` signal, between `churn` and the signal after it, over the last 50 timed lands (`ledger/landed-times`): one line for each of the up to five steps, by name, with the most total seconds across those lands, most first and ties by name, as `<path>: health/land-time: <step>: <runs> runs, median <n>s, total <n>s, selected by <paths>`, `<runs>` the times the step ran across them, the median the lower middle value for an even count, `<paths>` up to three paths that most often selected it, most first and ties by path, joined by `, `, and `<path>` the first of them. A step no path selected SHALL be listed at `clm.toml`, ending `selected by no file`. Its summary line SHALL be `<m> of <n> timed lands over <t>s`, `<n>` the timed lands read and `<m>` those whose check took more than `<t>` seconds, `<t>` being `Settings`' `land_check_target_seconds`. With no timed land it SHALL print no finding line, and its summary SHALL count 0 lands.

#### Scenario: slow steps
- GIVEN a ledger with three timed lands over a target of 60 seconds, each with a step `web` of 100 seconds selected by `web/src/main.ts` and a step `fmt` of 1 second, and one under it with only `fmt`
- WHEN `clm health` runs
- THEN its `land-time` lines name `web` first, with 3 runs, a median of 100 and a total of 300 seconds and `web/src/main.ts`, then `fmt` with 4 runs, the summary is `3 of 4 timed lands over 60s`, and it exits 0

#### Scenario: a step no file selected
- GIVEN a ledger with one timed land whose only step, `fmt` of 1 second, no path selected
- WHEN `clm health` runs
- THEN it prints `clm.toml: health/land-time: fmt: 1 runs, median 1s, total 1s, selected by no file`

#### Scenario: no timings
- GIVEN a ledger with no timed `landed` record
- WHEN `clm health` runs
- THEN it prints no `health/land-time` finding line, and the signal's summary counts 0 lands

#### Needs
- ledger/landed-times

### Requirement: commands-in-crate <!-- base: 9f6796118a837c79 -->
`clm-health` SHALL offer each of `clm health` and `clm workaround` as a call of its own that takes the command's arguments, the words after its name, as the command line gives them; the repository root the caller found, which the command's `--root` overrides; what the caller loaded for the command, for `clm health` its `Settings` or the violation that rejected them, and for `clm workaround` its `[effort] workaround_flag` or why it couldn't be read, together with a reader of standard input; the environment as a lookup, which neither command reads; and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. Given `--help`, each call SHALL write the command's help to the output stream, nothing to the error stream, and return 0: its first line the command's description, `Print one advisory report of the repository's health; exits 0 whatever it finds` for `clm health` and ``Flag that a requirement was hard to build: write `backlog/workaround-<component>-<slug>-<n>.md` with the situation from stdin and print its path. Commits nothing`` for `clm workaround`; then an empty line; then the usage line, `Usage: clm health [OPTIONS]` and `Usage: clm workaround [OPTIONS] --title <TITLE> <REQUIREMENT>`.

#### Scenario: a command called in a test
- GIVEN a fixture repository
- WHEN it calls `clm-health`'s call with the arguments of `clm health` and two buffers
- THEN the buffers hold what `clm health` prints to standard output and to standard error there, and the call returns the status `clm health` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm health` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Scenario: help
- GIVEN the call of `clm health` and that of `clm workaround`
- WHEN each is passed `--help`
- THEN each writes its help to the output buffer, its first line the command's description and its third its usage line, as stated above, writes nothing to the error buffer, and returns 0

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: rule-signals <!-- base: 015eed9a8caa83de -->
After `reconsider`, `clm health` SHALL report four signals of the rule for a requirement (`requirements-state-what-the-contract-shows`), each advisory. A requirement's *statement* is its lines below its heading up to its first line that starts `####` outside a fenced block, the statement `gate/cuts` reads. `outside-contract` reads the code spans of the statement's lines as written, joined by line breaks, as `check/asset-references` reads them, fenced code blocks among what holds none; `test-placement` reads the statement's lines with its fenced lines left out, joined by line breaks.
- under `outside-contract`, for each requirement, each distinct code name in its statement's code spans, in the order it first appears, that the component's Contract text doesn't contain, compared with case, as `specs/<component path>/SPEC.md: health/outside-contract: <component>/<slug> names <name>`, the summary counting `names`. A code name is the text of a code span with one trailing parenthesised list taken off, when that text is identifiers, each an ASCII letter or `_` followed by ASCII letters, digits or `_`, joined by `::`, and the span held such a list, or the text holds `::`, or holds an ASCII lowercase letter directly followed by an ASCII uppercase one;
- under `test-placement`, each requirement whose statement, read as above, holds a sentence that places a test as `gate/cuts` reads a sentence and a placement, as `specs/<component path>/SPEC.md: health/test-placement: <component>/<slug> <phrase>`, `<phrase>` the placement phrase that comes first in the first such sentence, spelled as `gate/cuts` lists it, the summary counting `requirements`;
- under `design-exit`, each line of a component's Design outside fenced blocks that holds `exit`, `exits` or `exited`, then optionally `with`, then optionally `status` or `code`, then a number of ASCII digits, the words compared without regard to ASCII case, each word and the number with no ASCII letter, digit or `_` directly before or after it, and each separated from the next by one or more white-space characters, as `specs/<component path>/SPEC.md:<line>: health/design-exit: <match>`, `<match>` the line's first match as written, once per line, the summary counting `lines`. A Contract SHALL NOT be read for it;
- under `uncited-test`, each test that `check.coverage/test-lists` lists as holding no marker, among the files git lists for the checkout, tracked or untracked and not ignored, that some component owns (`check.coverage/owner`) and that read as UTF-8 text, as `<file>:<line>: health/uncited-test: <name>`, `<line>` the test's first line, the summary counting `tests`.

#### Scenario: a name the Contract doesn't hold
- GIVEN component `a` whose Contract names `select(graph)`, and whose requirement `a/x` says its command SHALL print what `select(graph)` returns, as `walk_up(path)` finds it, in the file `clm.toml` names
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/outside-contract: a/x names walk_up` and no `outside-contract` line naming `select` or `clm.toml`

#### Scenario: a requirement that places a test
- GIVEN requirement `a/y` whose statement ends `Every test of a's requirements SHALL sit in a's own files.`
- WHEN `clm health` runs
- THEN it prints `specs/a/SPEC.md: health/test-placement: a/y SHALL sit`

#### Scenario: a phrase in a sentence of behaviour
- GIVEN requirement `a/z` whose statement is `The directory SHALL be deleted once nothing else is left in it. Every test SHALL pass.`
- WHEN `clm health` runs
- THEN it prints no `test-placement` line naming `a/z`, since neither sentence holds both a placement phrase and the word `test` or `tests`

#### Scenario: an exit in a Design
- GIVEN component `a` whose Design line 12 says `A missing file exits 1 with the path`, and whose Contract says `it exits 2 when git fails`
- WHEN `clm health` runs
- THEN it prints exactly one `design-exit` line, `specs/a/SPEC.md:12: health/design-exit: exits 1`

#### Scenario: a test that cites nothing
- GIVEN component `a` owning `crates/a/tests/t.rs`, which holds a test citing `a/x` and, at line 9, a test `helper_rounds_up` with no marker, and `crates/a/tests/bin.rs`, whose bytes aren't UTF-8 and which holds a test with no marker, and `vendor/v.rs`, owned by no component, holding a test with no marker
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
- check/asset-references
