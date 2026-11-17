---
status: applied
resolves: [the-build-loop-shows-a-printed-run-action-as-running-and-gives]
---

## Intent

The human can hold a run the build loop has printed and the session hasn't launched, and the queue no longer shows such a run as running. `clm build hold <name>` records the human's word that the run waits: no step prints it again or plans another of its name, and it sits in the human's queue until `clm build hold --lift <name>` records their word to go on. The queue lists a printed run the session hasn't reported launched at the head of next up, as `printed, not launched yet`, and under running only runs with something behind them. The fix is outside the plan's freeze, so a question item holds its build back until the human says whether to make the exception; the build skill's sentence for the new command waits in a deferred item on `plugin` until the command is built.

## Problem

In a `/clm:build` session the human had the session stop a hung `clm verify --due` and asked it to hold the verify until a fix for the hang had landed. The session reported the stop with `clm build returned verify ... --cause stopped`; the loop finished the verify and, idle with verify due, planned a new one, so every later `returned`, `step` and `approved` printed `run verify clm verify --due --event work-done` again. The session never ran it, as the human asked, and never reported it launched. `clm view queue` and the panel's Queue tab then listed it under running (`verify  round 1  48m`) with no task behind it, and the human asked whether a verify was running.

Two things are wrong in the design, not the code. The loop's queue model puts every run that hasn't returned under running, a `planned` run included, though `orchestrator.loop`'s own Design says a planned run is one the session hasn't launched. And the build skill's rule "Never decide a launch, an order, a retry or a resume yourself" leaves the human's "hold it" with no command to carry it, so the session can only run a step the human said not to run, or leave the action alone and the queue wrong. No requirement states either behaviour, so both are requirements of this change (`code-fix-needs-a-written-rule`).

No change or code fix since 2026-11-09 fixes it: `20261115-clm-build-returned-fails` settled that a printed `run` stays `planned` until `clm build launched` reports it, which is the state the view now reads apart.

## Approaches

- **Taken: a run state `held`, set and lifted by the human's word through `clm build hold [--lift] <name>`.** It reuses what the loop already has: a run that hasn't returned already keeps the plan from planning another of its name (verify's due check, an approved batch's land, a claimed component), and `start`, `stop --now`, the drain's end and `release` read states, so the new state needs only the rules that say it isn't printed, isn't an orphan and isn't waited for. It is a command because the session decides nothing (`build-loop-in-clm`): a hold kept in the session's context would be lost to compaction and unknown to the next session.
- **Taken: the loop, not `view`, moves a printed run to next up.** Which run goes in which section of `view`'s queue model is the loop's choice (`plan`), so the fix changes nothing in `view`, whose model would otherwise gain a field the loop builds by struct literal, a coupled change across two components (`coupled-changes-sequence`) for a word the next section's reason already carries.
- **Not taken: a hold until a condition, such as a change landing.** The loop would have to read conditions no command of its reports, for a case the human settles with one word (`yagni`).
- **Not taken: holding by kind, or before anything is printed.** One run's hold covers its kind, since none of its name is planned while it is held; a hold on a name nothing printed has no run to record it on.
- **Not taken: holding a launched run.** Stopping it is the agent harness's, through the session (`clm build stop --now`, or stopping its task and reporting `--cause stopped`); what the loop then prints again can be held.
- **Not taken: accepting the defect,** the plan's default answer while the freeze holds. A queue that lists a run as going when nothing is behind it misleads the human about the one thing the queue is for, and a held step has no other route; whether to build it now is the human's, so a question item asks.

## Outcome

- `orchestrator.loop/hold` (new): `clm build hold <name>` marks a `planned` run `held`, keeping its claim, branch, place and prompt; no step prints it or plans another of its name; `start` leaves it held, not an orphan; `stop --now` leaves it; a drain ends without it and its summary names it; `--lift` makes it `planned` again, its action printed unchanged. Any other run, or none, exits 1 naming its state.
- `orchestrator.loop/queue-tells-printed-from-running` (new): the queue model lists under running only `running` and `stopped` runs, a `planned` run heads next up with `printed, not launched yet`, and a `held` run is in neither.
- `orchestrator.loop/loop-file` (modified): the state `held`, and the time a run was held.
- `orchestrator.loop/humans-queue` (modified): each held run, since it was held, with `say "go on with <name>"`.
- `orchestrator.loop`'s Contract gains the command and its fetch; its Design states the state and gains *Holding a run* and *What the queue shows*.
- `backlog/the-fix-for-a-printed-run-shown-as-running-is-outside-the-plan-s-freeze` (`kind: question`) blocks the four requirements until the human answers.
- `backlog/the-build-skill-has-no-sentence-for-holding-a-printed-run` (`kind: deferred`, `plugin`) adds the skill's sentence once `orchestrator.loop/hold` is current.

## Constraints

- `view`'s queue model and charts are unchanged; the panel still finds running entries by the lines under `running`.
- A planned run's action is still printed, unchanged, by every step until it is reported launched or held (`step-actions`).

## Edge cases

- **A held build or draft keeps its claim, which can lapse.** `clm run` pushed it before the `launch` was printed, and nothing pushes it while the run is held, so after its expiry another clone may claim the component. Lifting the hold then launches a prompt on a claim no longer the clone's, which the build's own push finds; the human who holds a build for long gives it up with `clm build release` instead. Accepted: the hold exists for the run the human holds for a short time, and a held build keeps its place, which they see in the queue.
- **A held run keeps its place.** A held build or draft counts against `max_builds` until lifted or released, as a stopped one does.
- **The idle verify while a build is held.** A held build isn't running, so the loop can be idle and plan a verify meanwhile; that is the loop's rule for an idle loop, unchanged.
