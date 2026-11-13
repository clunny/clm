---
component: check.health
base: { contract: "2a47643c8ade7e75" }
---

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

A requirement's *score* is defined by `reconsider-score`. Rewrites and counts alone never file anything: the report shows them, and a build's flag is what starts the count, so a long archive doesn't fill the human's queue. `clm rebase` files a `question` item for a requirement of the branch's component whose score reaches the threshold (`sync/file-reconsider-item`). Churn is reported and never scored: it is the same moves `rewritten` counts, seen from the code, so scoring both would count each built rewrite twice, and the score is computed without git, which `clm rebase` can't afford to read the whole history for (`churn-from-marker-history`).

`clm workaround <component>/<slug> --title <title>`, the situation on stdin, files a `workaround` backlog item naming the requirement at its current version (`model/parse-backlog`) and prints its path (`workaround-command`). A build session runs it when a requirement is hard to build in one of the ways the build prompt names (`orchestrator.run/prompt-workaround`). A flag is evidence, never a block: it changes no state the queue, the gate or `clm check` reads. It exits 3 when a capability the requirement serves has `[effort] workaround_flag` unsettled flags or more, telling the session to stop and ask whether to redesign it (`redesign-before-workaround`).

The library, for `sync` and the `clm` crate: `report(root, settings, expiry)` returns what `clm health` prints as `Report { sections }`, each section a signal with its lines; `signals(graph, items, settings)` returns each requirement's `RequirementSignals { id, workarounds, rewritten, scenarios, exceptions, score }` from a loaded graph, its backlog items (the map of slugs to items) and the settings the score compares with (`reconsider-score`), with no git; `reconsider(graph, items, settings, component)` returns the requirements of one component whose score reaches the threshold, each with the text of the item `clm rebase` files; `workaround(root, requirement, title, body)` writes a flag; `crowded(root, requirement, threshold)` gives each capability the requirement serves whose unsettled flags number `threshold` or more, with the count; `churn(root)` returns `None` when `HEAD` names no commit, and otherwise, for each requirement ID the commits reachable from `HEAD` ever cited with a version, whether the graph holds it or not, its distinct versions as `Churn { id, versions }`, reading git and nothing else. `Settings { reconsider, scenarios, exceptions, stale_days, lcov, .. }`, whose last field holds the size budgets (`Settings::component_size`), is this crate's own struct, which the `clm` crate fills from `config`'s `health` and `component_size` (the budgets as `clm_check::Budgets`), so `clm-health` doesn't depend on `clm-config`, as `clm-check` takes its detectors.

## ADDED Requirements

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
