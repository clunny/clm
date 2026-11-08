---
status: applied
from: backlog/slow-land-checks-go-unnoticed
---

## Intent

Give an agent what it needs to judge whether a land's test selection is right and whether a suite belongs after the land (`[verify]`) instead of in every land. A project sets a target time for its land check. `clm land` times each step the check marks and records the times in the ledger. A land that goes over the target explains itself: its costliest steps, the chain of components and files that selected each one, and how recent lands fared. The agent then decides whether anything needs to change. CLM applies no rule and files nothing.

## Problem

`[land] check` is one opaque command to CLM. CLM keeps no record of how long checks take, which steps take the time, or why a change selected them. A check that slows down goes unnoticed. When someone does notice, nothing separates the two causes:

- **The selection reaches too far.** A change selects suites it can't break. On 2026-11-08 a one-test change to `clm-web-replica` took 218 s, 198 s of it in `web`, `web/site` and `capabilities/web`. `clm affected` reached `web` through `web.worker`, and the check script runs `web/site` whenever it runs `web`, though `web.site` wasn't affected.
- **A suite is slow wherever it's selected.** It's a candidate for `[verify]` (`slow-tests-after-land`).

The fix differs for each: a `depends_on` edge, an ownership or a script rule in the first case, a move after the land in the second. Telling them apart takes per-step times and the chain that selected each step.

## Approaches

- **Time the steps the check already marks (chosen).** `clm land` already reads GitHub Actions' `::group::` lines (`check-step-markers`) to name a failing step. It times each step, and a step names the selected files that called for it with `::clm-selected::<path>` lines. From those files CLM traces the chain through the graph itself. The timing works with any runner and reuses a format CLM already reads, and it's the only approach that ties a cost to a chain. Its granularity is whatever the check marks, typically a suite, which is also what a project moves after the land.
- **Per-test times from JUnit XML reports.** Finer, but it needs several report files, runner configuration and XML parsing. A test's time also doesn't say which selected file called it in, so it can't give the chain. It could be added later as a second source if suites prove too coarse.
- **Whole-check time only.** Cheap, but it says neither what cost the time nor why it ran.

Also rejected: a rule that files a backlog item when lands exceed the target often enough (such as 3 of the last 10). Whether a run of slow lands needs action depends on what caused it: one large shared change, a loaded machine, or a suite that grew. That's a judgement, so the agent makes it with the facts in front of it.

## Design

**Target** (`config`). `[land] check_target_seconds`, a positive integer, default 120, set in `clm.toml` or `clm.local.toml` like the rest of `[land]`. A land *counts* when it ran the check command. A land whose code key was already checked runs only `clm check`, so it doesn't count.

**Steps and their selectors** (the check, `ci` in this repository). The check wraps each phase in `::group::<name>` and `::endgroup::`. Inside a step it prints one `::clm-selected::<path>` line for each selected file that made it run the step. A step that runs by the script's own rule, such as `web/site` after `web`, names the files behind that rule, so the reach is attributed to the script, not the graph. A step that runs on every land, such as fmt or clippy, names none. This repository's `ci-check.sh` marks every phase it times today (`phase-times`).

**Timing** (`land`). The runner that reads the check's output already follows its steps. It also takes each step's wall-clock time, from its `::group::` line to its end, and the `::clm-selected::` paths inside it (`check-selector-lines`), along with the whole command's time. `clm land` keeps them on the `landed` record it appends. Before it leaves the merged checkout, it computes the chains for the costliest steps. It prints the report after appending `landed`.

**The record** (`ledger`, `land-times-on-the-landed-record`). `landed` gains two optional fields, present exactly when the land ran the check command: `check_seconds`, the command's time, and `steps`, a list of `{name, seconds, selected_by}`. A land without them reads as untimed. The `checked` record stays a fact about a code key (`graph-test-selection`): what a run did depends on its selection, not its code, so `--check-only` and `verify` record no times. They're a few dozen bytes on a record every land already writes to its writer's own ledger branch, so they add no write, no contention and no rebase (`per-writer-ledgers`).

**The chain** (`check.affected`). `clm affected --why <path> [--base <ref>]` prints why the change selects `<path>`, one link per line. A changed file selects itself. Otherwise the file is selected through the component its `req:` marker cites, or through its owner, and that component is affected either because it owns a changed file or through a `depends_on` path to one, ending at a changed file. A file the change doesn't select is reported as such. `land` runs it as it already runs `clm affected`, against the same base.

**After a passing land** (`land`). `clm land` prints one line with the check's time against the target. When the time is over the target it also prints:
- the three costliest steps, each with its time and the chain for its first `selected_by` path;
- how many of the last 20 counted lands, read from the ledger's fold, went over the target, and the steps that cost the most across them;
- one line pointing to `clm guide ci-and-deployment`, which says how to weigh this.

Nothing is filed and the land isn't affected. The output itself carries the hint, so every caller sees it: a session, `clm run`'s unattended build and a human.

**The trend** (`check.health`). A `land-time` signal, read from the ledger: over the last 50 counted lands, how many went over the target, and for each of the five steps with the most total time, its runs, median and total time, and the selector paths that most often called it in. `health` stays advisory (`health-advises`).

**Guidance** (`docs/guide/ci-and-deployment.md`). The "where slow tests go" section gains how to read an over-target land:
- a step reached through a long chain, or through a script rule the change can't break, points to a selection fix: a `depends_on` edge, a component's `paths`, or the script's mapping;
- a step that's costly however it's reached is a candidate for `[verify]`;
- a single overshoot, from a big shared change or a loaded machine, needs nothing;
- when the agent judges that a pattern needs work, it files a backlog item naming the steps and chains.

`docs/guide/settings.md`'s `land` section documents `check_target_seconds`.

**The capability.** `CAPABILITIES.md`'s `slow-tests-after-land` gains what a person learns from a land over the target, with a scenario. `capability`'s own build writes that scenario's test once `land/check-time-report` is current (`capability-tests-follow-rules`).

**The CLI** (`clm`). The root wires `clm affected --why` to `clm-affected`'s `why`, and `clm health`'s settings to `[land] check_target_seconds` (`affected-why-command`, `health-land-target`).

## Constraints

- Advisory only (`health-advises`, `deterministic-enforcement`): no gate, check or land outcome depends on the timings.
- Language-agnostic: CLM knows no test runner (`graph-test-selection`). Timing reads only the markers.
- A check that marks no steps still records `seconds` and prints the time against the target, with no steps or chains.
- Times are noisy, since builds share the machine (`build-places`). The per-land output reports that land's times, and `health` uses medians.

## Decisions

- `land-times-on-the-landed-record`: a land's check times go on its `landed` record, not on `checked`.
- `check-selector-lines`: a check names a step's selecting files with `::clm-selected::` lines.

## Left to the builder

- The exact wording of the lines `clm land` prints, within the shape above.
