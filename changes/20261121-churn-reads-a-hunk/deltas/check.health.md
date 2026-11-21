---
component: check.health
base: { design: "78894344018dbec3" }
---

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
- `churn`: `churn(root)` runs, through `clm-git`, `git -c diff.noprefix=false -c diff.srcPrefix=a/ -c diff.dstPrefix=b/ -c log.showRoot=true -c diff.mnemonicPrefix=false -c core.quotePath=false log -p --format= --no-color --no-renames --no-ext-diff --no-textconv` over the commits reachable from `HEAD`, the `-c` settings and the `--no-` options pinning the patch's shape against the machine's git configuration, and reads the output as patches: a file's header runs from its `diff --git ` line, which leaves no file named, to its first `@@` line, which starts its hunks, and a file with no hunk (a binary file, a mode change, an empty file) has only a header. Within a header a `+++ ` line names, as `+++ b/<path>`, the file the added lines after it belong to (`+++ /dev/null`, a deleted file, names none; git ends the header of a path holding a space with a tab, which is taken off; a path git quotes, `+++ "b/<path>"`, is unquoted as git quotes it, its C-style escapes and octal bytes read back). Within the hunks every line starting with `+` is an added line, whatever follows the `+`, so a removed line whose text starts `-- ` followed by an added one whose text starts `++ `, which show as `--- ` and `+++ `, is a removed line and an added one, never a header; the added line's text after the `+` is handed to `clm-coverage`'s `marker_spans`. Every hunk line starts with ` `, `+`, `-` or `\`, so none reads as a `diff --git ` or `@@` line. A removed line changes nothing, so a version once cited stays counted. The history isn't filtered by path: the scan costs the same with or without one (`clm health` and the test that runs it on this checkout pay for the whole history) and a path filter loses a file's history before a rename. A merge commit shows no patch, so a marker its resolution alone added isn't seen; the working tree's uncommitted markers aren't either. Before the log, `git rev-parse --verify --quiet HEAD` says whether there is a history: its status of 1 means `HEAD` names no commit and `churn` returns `None`, and any other failure is git failing (`health-report`). The entries are sorted by `id`, each `versions` sorted, and the section lists them in the graph's order, as `rewritten` does.
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

### Requirement: churn-signal <!-- base: 06a1b9c3877580b1 -->
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

#### Scenario: a hunk line that reads like a header
- GIVEN component `a` with requirement `x`, and a history whose first commit adds `tests/x.rs` holding a line `-- one` and a line `req:a/x@11111111`, and whose second commit, in one hunk of `tests/x.rs`, replaces `-- one` with `++ b/notes.md` and adds after it a line citing `req:a/x@22222222` and a line citing `req:a/x@33333333`
- WHEN `churn` is called, and `clm health` runs
- THEN `churn` returns `a/x` with the versions `11111111`, `22222222` and `33333333`, and `clm health` prints `specs/a/SPEC.md: health/churn: a/x cited at 3 versions`

#### Needs
- check.coverage/marker-spans
