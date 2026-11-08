---
status: done
resolves: [slow-tests-after-land]
priority: P2
---

## Intent

Let a project take its slow tests out of `clm land` and run them after the land, at moments the project chooses, on its CI host or on its own machines, with the result recorded in the ledger and a red result coming back as a bug that stops code lands on the component it broke. Today every land waits for the whole `[land] check`: in a project whose check takes 15 minutes, a one-file migration waited the full run (GitHub issue #262), and the guide's only answer for slow tests after the land is a by-hand bisect and a hand-filed bug.

## Problem

`clm land` is the project's CI: it checks the merged tree and pushes `main` only when it passes (`deploy-is-downstream`). Everything in `[land] check` costs every land. `clm affected` already narrows the check to what a branch can break, but a browser end-to-end suite or an integration run still makes each land slow, and lands are serial. Moving those tests after the land needs four things CLM lacks: a moment that triggers them, a place they run, a record of the result, and a way back when they fail after people and builds have pulled `main`.

The moment is the hard part. With AI builds landing all day, running the slow tests continuously on the newest `main` (the usual continuous-delivery answer) keeps the suite busy without pause, and each red run spans dozens of lands. The trigger should follow units of work CLM already has, and which unit fits depends on the project.

## Approaches

**Trigger.**
- *Continuously on the newest `main`, latest wins.* The usual practice; rejected by the human: with AI landing continuously it never rests and blames dozens of lands at once.
- *Per land.* Only if the suite is cheap, which is the case it isn't for.
- *Project-chosen triggers over CLM's own units of work (chosen).* A change finishing, the queue draining, the end of a piece of work, a backstop of N lands or N days, and a release asking for it. The project picks any set; CLM computes what is due.

**Where it runs and who records it** (all three chosen, as a per-project `mode`):
- *`pipeline`*: the host's pipeline runs `clm verify --due` itself and appends the result to the ledger. Needs the pipeline to push `clm/ledger/*` (`contents: write` on GitHub's workflow token) and, in a signed repository, a signing key of its own.
- *`import`*: the host's pipeline runs the slow tests and only reports; `clm verify` on a developer's machine reads the pipeline's outcome for the due commit through the host adapter and appends the record. The host needs no write access; each host needs a "list runs" call.
- *`local`*: `clm verify` runs the slow tests on this machine and appends the record.
- Rejected: *CLM reads host results directly, with no ledger record*: triage, the stop and a release check would each need two readers, and a project with no host would have none. *Splitting `[land] check` into gating and after parts*: CLM would then have to make sure the after part runs somewhere, which is the same design with a worse name.

**How a session changes the settings.** A general `/clm:configure` skill, covering every setting, is its own next change (`backlog/settings-changed-only-by-hand`, P2). This change sets `[verify]` up through `/clm:init` and offers it through `clm upgrade`.

## Design

**Settings** (`config`). A `[verify]` section in `clm.toml`, absent by default (today's behaviour, slow tests stay in `[land] check`):
- `check`: the slow-test command, run as `[land] check` is (through `sh -c`, `CLM_SELECTION` naming the tests `clm affected --base <last verified commit>` selects, so a run covers only what changed since its tests last passed).
- `mode`: `local`, `pipeline` or `import`.
- `triggers`: any of `change-done` (a land archived a change since the last verified commit), `queue-drained` (nothing is pending and something landed since the last verified commit), `work-done` (the caller says a piece of work ended: the end of `/clm:build`, a draft, or `clm wrap-up`; `local` mode only), `every = { lands = N, days = D }` (a backstop on how much may pile up unverified), and `release` (always on: `clm verify --commit <sha>` runs or answers for that commit).
- For `import`, the pipeline to read (its workflow name on GitHub).

**`clm verify`** (new command).
- `clm verify --due [--event work-done]` works out from git and the ledger whether a trigger has fired since the last verified commit on `main`; when none has, it prints why not and exits 0 in seconds, so a pipeline can run it on every push and a session at every end of work. When one has, it verifies `origin/main`'s head: runs `[verify] check` there (`local`, `pipeline`) or reads the pipeline's outcome for that commit (`import`, reporting `pending` while the run hasn't finished), and appends a `verified` record.
- `clm verify --commit <sha>` does the same for one commit of `main` and exits 0 when it passed, so a release or deploy step can require it; a commit already verified green answers from the ledger.
- It runs under the clone's check lock, like every other check on the clone (`check-runs-one-at-a-time`): CLM can't know what a project's tests share, so a slow run and a land's check never overlap. A land on that clone waits while a slow run's check runs; triggers make that rare, and a project that can't accept the wait uses `pipeline` mode or runs `clm verify` from another clone. A run that is killed records nothing, so its trigger stays due.

**The record** (`ledger`). `verified`: the commit, `passed`, the previous verified commit it covers from, the lands between them with their changes and components, the mode, and for `import` the run's URL. Unsigned, like every kind but approvals and lands: it is a report, and holds no land by itself.

**The way back.**
- *Finding the culprit.* A red `verified` record covers the lands since the last green one. `clm verify` in `local` and `pipeline` mode then bisects those lands along `main`'s first parents, re-running the selected slow tests, to name the land that broke them; `import` records the range and leaves the bisect to a local `clm verify --bisect`.
- *Into the backlog.* `clm intake` lists each red `verified` record that no backlog item takes in, with the culprit land, its change and components, as it lists an open issue or alert, and `/clm:triage` turns it into a `kind: bug` item on those components on the human's yes, offering `P1` (priority stays the human's call: `priority-caps`). The fix claims the component with `--fix`, either fixing forward (with a fast test reproducing the failure where it can) or reverting the land with `git revert -m 1 <land>` through `clm land`; a revert reverts the land's `req:` markers too, so its requirements show pending and the queue rebuilds them. Nothing rewrites `main`.
- *Stopping the line, narrowly.* A red record is a report. `clm land` warns at once when a branch touches the components of a red result nobody has taken in yet. Once the human takes the failure in as a `kind: bug` item (the human's judgement that it is real, not a flake), `clm land` refuses code lands touching that item's components until a branch whose commit carries `Resolves: backlog/<slug>` for it lands. Other components keep landing. A host's run never holds a land by itself, so `land-locally` and `deploy-is-downstream` hold as written: the host reports, a human decides the failure is real, and `clm land` enforces it from git. A red result is also due again at once after any later land, so a fix is checked.

**Setup** (`init`, `upgrade`, `plugin`). `/clm:init` asks whether the project has slow tests to move out of the land, and if so where they run, which mode and which triggers, writes `[verify]`, and for `pipeline` and `import` says which workflow change the project makes (CLM generates no pipeline: `deploy-is-downstream`). `clm upgrade` offers the same as an advised migration to projects set up before this change; declining leaves today's behaviour. `/clm:build` and `/clm:triage` run `clm verify --due --event work-done` at their end, before `clm wrap-up`, which keeps a slow run out of a cleanup command; it does nothing unless the project's triggers include `work-done`. The guide's `ci-and-deployment` page replaces its by-hand recipe with the triggers and the three modes.

## Outcome

A project with a slow suite lands at the speed of its fast checks, runs the slow suite when its own triggers say, wherever suits it, and learns of a break as a top-priority bug on the component that caused it, with code lands there held until it's fixed. A project that sets nothing sees no change.

## Constraints

- `land-locally`, `deploy-is-downstream`: `clm land` stays the only thing that pushes `main`; host pipelines run and report, and CLM writes none.
- `git-is-truth`: results live in the ledger; `clm verify --due` reads git and the ledger, plus the host only in `import` mode.
- `deterministic-enforcement`: the stop is a land rule, decided by a program from ledger records.
- Untrusted host text: `import` reads a run's status, commit and URL, never its log or title.

## Settled after review

- **When the stop starts** (architecture audit: `land-locally`). The first draft held a component as soon as a red record named it. In `pipeline` and `import` mode that let a host's run decide a land, the role `land-locally` took away from the host, and a flaky run would stop the line with no human having looked. The human chose the stop to start when they take the failure in as a bug, with `clm land` warning at once until then.
- **The check lock** (architecture audit: `check-runs-one-at-a-time`). The first draft ran slow tests under their own lock so lands never waited, leaving collisions to the project, the warn-only approach that decision rejected. The human chose that a local slow run takes the clone's check lock.
- **The name.** `clm verify` keeps its usual meaning, as Maven's `verify` phase runs integration tests after the build. `clm ledger verify`, an audit, is the odd one out: the glossary defines both, and `backlog/ledger-verify-is-an-audit` renames it.
- **The upgrade migration** (builder check: `upgrade-corpus`). A note with no `old`/`new` would stay pending after `--apply`. It is an advised apply migration instead, appending the commented section, which changes no setting, and `/clm:upgrade` asks the human whether to set it.
