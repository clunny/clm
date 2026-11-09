---
component: verify
base: { contract: "0be8fddc40796fa5", design: "20516a5673313df9" }
---

## MODIFIED Contract
Running a project's slow tests after the land, at the moments the project chooses, and recording the result in the ledger (`verify-after-land`). Every `clm verify` takes its `[verify]` and `[affected]` settings from the `clm.toml` committed at `origin/main`, which it fetches first, never from the checkout it runs in, so a checkout behind `main` judges `main` by `main`'s settings (`settings-from-main`). A project whose `origin/main` sets no `[verify]` (`config`) keeps its slow tests in `[land] check`, and every `clm verify` then prints `verify off: no [verify] in origin/main:clm.toml` and exits 0. Crate `clm-verify` offers, dispatched by `crates/clm`:
- `clm verify --due [--event work-done] [--dry-run]`: whether a trigger has fired since the last verified commit of `origin/main` (`verify-due`), and when one has, the verification of `origin/main`'s head: in `local` and `pipeline` mode by running `[verify] check` (`verify-run`), in `import` mode by reading the pipeline's result (`verify-import`). With `--dry-run` it only says whether a run is due.
- `clm verify --commit <sha>`: the verification of one commit of `main`, for a release or deploy step (`verify-commit`).
- `clm verify --import`: in `import` mode, the record of every completed pipeline run on `main` newer than the last record, run nothing; in the other modes it prints `verify nothing to import` and exits 0.
- `clm verify --bisect <sha>`: the land that broke a red commit, found by running `[verify] check` on the lands it covers (`verify-bisect`).

Every outcome is one line on stdout starting `verify `: `verify not-due <why>`, `verify due <trigger>`, `verify passed <sha> <n> lands`, `verify failed <sha> culprit <land|unknown> components <component>...`, `verify pending <sha>` and `verify nothing to import`, `<sha>` abbreviated as `git rev-parse --short` abbreviates it. It exits 0 when it passed, nothing was due or there was nothing to import; 1 when the verification failed, or with the reason when it can't run (a `<sha>` not on `origin/main`'s first parents, a `[verify]` it can't use); 2 when git fails; 3 when, with `--dry-run`, nothing is due; and 4 when the result is pending (`import`). Each verification it decides appends one `verified` record (`ledger`); nothing else writes one.

## MODIFIED Design
**Settings** (`settings-from-main`). `clm verify` verifies `origin/main`, so it reads how to verify it from `origin/main`: the commit it checks out is the one whose settings it follows. It first loads the root's own settings for `host` alone, which names the host adapter, fetches `origin` (every host has one: `none-keeps-its-own-remote`), and then loads the settings again with `config`'s `load_committed`, the repository layer being the text of `origin/main:<prefix>clm.toml` (none when `origin/main` or the file is missing) and its source `origin/main:clm.toml`. `[verify]` and `[affected] all` come from those; the clone's `clm.local.toml` still sets either for this clone, as everywhere (`local-settings`), since it belongs to the clone and is never behind. Whether `[verify]` is set is decided only after the fetch, so a project with no `[verify]` pays one fetch per call; a fetch that fails is exit 2 as for any other call. The sessions that run `clm verify --due --event work-done` at their end (`plugin/skills-run-verify`) run it in their own checkout, which they leave as it is and which may be far behind `main`: reading the checkout's file there skipped every run with `verify off` once `[verify]` had landed after the checkout was made. `host` stays the root's, as `land` keeps it (`land-policy-from-base`): it is needed before there is anything fetched to read, and names the tooling, not the policy. This is the decision `verify-settings-from-main`.

**Since when.** The *last verified commit* is the newest commit on `origin/main`'s first-parent chain that a `verified` record names, whatever its result; the *last green commit* the newest that one with `passed: true` names. The *unverified lands* are the first-parent commits of `origin/main` after the last verified commit, up to its head, or all of them when no record exists. A run's tests are selected by `check.affected`'s library call with the last green commit as its base, as `clm affected --base <last green commit>` selects them (`graph-test-selection`), written to a file and handed to the check as `CLM_SELECTION` as `[land] check` gets it, so a run covers everything changed since the tests last passed, the lands of an earlier red run included; with no green record there is no selection and every test runs.

**Triggers** (`verify-due`). Read from `[verify] triggers` and `[verify] every` over the unverified lands: `change-done` when one of them adds a `changes/archive/<id>/change.md` (`git diff --diff-filter=A` against its first parent); `queue-drained` when `check.coverage` finds no pending requirement in `origin/main`'s head, checked out into the verify tree (only when `queue-drained` is among the triggers); `work-done` when the caller passed `--event work-done`; `every.lands` when there are at least that many; `every.days` when the oldest of them was committed at least that many days ago. A red last record makes any unverified land a trigger (`red-again`), since a fix may have landed. No unverified land means nothing is due, whatever else holds.

**The run** (`verify-run`). It checks `origin/main`'s head out into `<home>/.clm/verify-tree` as `land` keeps its land tree (`fixed-land-tree`), `<home>` found as `land` finds it, and runs `[verify] check` there through `land`'s shared check runner (`land/shared-check-runner`), so under the clone's check lock, `<home>/.clm/check.lock`, like every other check of the clone (`check-runs-one-at-a-time`): CLM can't know what a project's tests share, so a slow run and a land's check never overlap. A land on the same clone waits while a slow run's check runs; triggers make that rare, and a project that can't accept the wait runs its slow tests in `pipeline` mode or from another clone. Each bisect probe takes the lock afresh, so lands can go between probes. Its output goes to stderr and to `<home>/.clm/verify-logs/<sha>.log`, which a passing run removes and a failing one keeps. `local` and `pipeline` differ only in where the project calls it: `pipeline` is a host pipeline running `clm verify --due` on each push to `main`, with `CLM_WRITER` naming it and the right to push `clm/ledger/*`. A run killed before it records leaves its trigger due. `clm verify --commit <sha>` for a commit other than the head takes the last verified commit before it as its `since` and checks that commit out.

**Import** (`verify-import`). `host`'s `runs` for `[verify] workflow` (`host/list-runs`), each with its head commit, whether it completed and its conclusion. A completed run whose conclusion is `success` or `failure` on an unverified land is a result; the newest such run's commit is the one recorded, covering the unverified lands up to it, `passed` from its conclusion and `run` its URL. Any other conclusion (cancelled, skipped) is no result. With none, `verify pending <head>`, exit 4.

**Culprits and components** (`verify-bisect`). A red result over one unverified land names that land as the culprit. Over several, `local` and `pipeline` runs bisect at once: a binary search of the first-parent lands between the last green commit (exclusive) and the red one, each probe running the same selection in the verify tree, the first red land being the culprit; `import` records `culprit: null` and leaves the search to `clm verify --bisect <sha>` on a developer's machine, which appends a fresh red record for the same commit with the culprit found. The record's components are those whose `paths` hold a file the culprit's first-parent diff changes, or, with no culprit, a file any of the covered lands' diffs change; when no component's `paths` hold such a file (a land that changed only doc-only paths), the root component.

**What it stops.** A red record is a report: `clm intake` lists it for triage (`orchestrator.triage/intake-verify-failures`) and `clm land` warns a branch touching its components. Once the human takes it in as a `kind: bug` item with `verify_failures`, `clm land` holds code lands on the item's components until a branch whose commit carries `Resolves: backlog/<slug>` for it lands (`land/verify-stop`). A host's run never holds a land by itself (`land-locally`).

## ADDED Requirements

### Requirement: settings-from-main
Every `clm verify` SHALL take `host` from the settings of the CLM root it runs in, and, after fetching `origin`, on every host, every other setting it uses (`[verify]`, `[affected]`) from `config`'s `load_committed` with the text of the CLM root's `clm.toml` committed at `origin/main`, `None` when `origin/main` or that file doesn't exist, and `source` `origin/main:clm.toml`. It SHALL never take `[verify]` or `[affected]` from the working tree's `clm.toml`, and SHALL decide whether `[verify]` is set only after the fetch. When those settings set no `[verify]` it SHALL print `verify off: no [verify] in origin/main:clm.toml` and exit 0, running and appending nothing; when they fail to load it SHALL exit 1 with `config`'s message, which names `origin/main:clm.toml`. A fetch that fails SHALL exit 2, as git failures do.

#### Scenario: a checkout behind main
- GIVEN a clone whose checkout's `clm.toml` sets no `[verify]`, and an `origin/main` ahead of it whose `clm.toml` sets `[verify]` with `mode = "local"` and `triggers = ["work-done"]`, with one land since a passing `verified` record
- WHEN `clm verify --due --event work-done --dry-run` runs in the checkout
- THEN it prints `verify due work-done` and exits 0

#### Scenario: turned off on main
- GIVEN a checkout whose `clm.toml` sets `[verify]` with a `check` that writes a file, and an `origin/main` whose `clm.toml` sets none
- WHEN `clm verify --due --event work-done` runs in the checkout
- THEN it prints `verify off: no [verify] in origin/main:clm.toml`, exits 0, and the file isn't written

#### Scenario: an invalid setting on main
- GIVEN an `origin/main` whose `clm.toml` sets `[verify]` with no `check`
- WHEN `clm verify --due` runs
- THEN it exits 1 naming `origin/main:clm.toml` and `verify`, and appends nothing

#### Needs
- config/committed-settings
- config/verify-setting

## MODIFIED Requirements

### Requirement: verify-due <!-- base: 702ae35c187ac59b -->
`clm verify --due` SHALL compute the unverified lands of `origin/main` after fetching it and the ledgers, and SHALL find a run due when there is at least one and either the last `verified` record on `origin/main`'s first-parent chain is red, or one of `[verify] triggers` or `[verify] every` holds: `change-done` when an unverified land adds a file `changes/archive/<id>/change.md`, `queue-drained` when no requirement in `origin/main`'s tree is pending, `work-done` when `--event work-done` was passed and the mode is `local`, `every.lands` when the unverified lands number at least that many, and `every.days` when the oldest of them was committed at least that many days ago, the settings being those `settings-from-main` reads. When none holds it SHALL print `verify not-due <why>` and exit 0, appending nothing and running no check. With `--dry-run` it SHALL run and append nothing, print `verify due <trigger>` and exit 0 when a run is due, and print `verify not-due <why>` and exit 3 when not. With no `[verify]` set there, every `clm verify` SHALL print `verify off: no [verify] in origin/main:clm.toml` and exit 0.

#### Scenario: a finished change is due
- GIVEN `[verify]` with `mode = "local"`, `triggers = ["change-done"]`, a `verified` passing record for `origin/main~2`, and two lands since, the second adding `changes/archive/x/change.md`
- WHEN `clm verify --due --dry-run` runs
- THEN it prints `verify due change-done` and exits 0, and the ledger is unchanged

#### Scenario: nothing finished
- GIVEN the same settings, and two lands since the last record, neither adding a file under `changes/archive/`
- WHEN `clm verify --due` runs
- THEN it prints a line starting `verify not-due`, exits 0, runs no check and appends nothing

#### Scenario: work done only when asked
- GIVEN `mode = "local"`, `triggers = ["work-done"]` and one land since the last record
- WHEN `clm verify --due` runs, and then `clm verify --due --event work-done --dry-run`
- THEN the first prints `verify not-due` and the second `verify due work-done`

#### Scenario: backstop by lands
- GIVEN `triggers = []`, `every = { lands = 3 }`, and three lands since the last record
- WHEN `clm verify --due --dry-run` runs
- THEN it prints `verify due every.lands`

#### Scenario: red stays due
- GIVEN no triggers, a red `verified` record for `origin/main~1`, and one land since
- WHEN `clm verify --due --dry-run` runs
- THEN it prints `verify due red-again`

#### Scenario: nothing landed
- GIVEN `triggers = ["work-done"]` and a `verified` record for `origin/main`'s head
- WHEN `clm verify --due --event work-done` runs
- THEN it prints `verify not-due nothing landed since <sha>` and exits 0

#### Scenario: off
- GIVEN an `origin/main` whose `clm.toml` has no `[verify]`
- WHEN `clm verify --due` runs
- THEN it prints `verify off: no [verify] in origin/main:clm.toml` and exits 0

#### Needs
- config/verify-setting
- config/committed-settings
- ledger/verified-record
