---
status: applied
resolves: [reconsider-plugin.panel-build-opens-panel]
---

## Intent

`/clm:build` becomes one continuous loop per clone that `clm` drives: a new `clm build` stepper decides every step of the loop (what to launch next, in what order, how to handle a build's return, when to fold, archive and verify) and keeps the loop's state in git and in a loop file beside the build places, so a crash, a cleared context or a new session resumes the loop with nothing lost. The session only carries out the stepper's actions with the agent harness's subagents and reports their returns. Everything that needs the human waits in sight in a new Queue tab of the CLM panel, beside what is running and what runs next, until it is dealt with, and the panel's buttons drain or stop the loop.

## Problem

An audit of `/clm:build` (2026-11-12) found the queue's foundations sound (the queue read from `origin/main`, compare-and-swap claims, places, effective priority, `Needs`-only order) but the loop that drives them wrong in five ways, all from one cause: the loop's rules live in a 22.5 KB skill that a model follows, against `agent-harness-adapters` ("every rule SHALL live in `clm` and git").

1. **The scheduler is prose.** Return handling (`waiting:` lines, a landed build still running a task, a build with no closing line, resume against rebuild against "stopped twice for the same reason") is a set of precedence rules in prose, several of them patches over the session getting it wrong (`build-waiting-builds`, `build-stops-landed-builds`, `build-unclosed-builds`). `clm runner` runs the same loop deterministically in Rust.
2. **The loop's state lives only in the context.** Which items this session drafted, each component's stops, which builds were resumed and which review drafts wait to be folded exist nowhere else; compaction in a long session loses them, re-drafting items or retrying a failing build without end.
3. **Free places sit idle mid-drain.** The session looks at the queue again only when one of its own builds returns, so work landed by another session, or a place another session frees, waits for that return, which can be an hour.
4. **Drafts go before builds.** Drafts and builds share `max_builds`, and each round drafts first, so three P4 bug groups fill every place ahead of a P1 build.
5. **The approval waits for the whole drain.** Review drafts are folded only once every build has returned, so the one approval the human gives, and everything it unblocks, waits hours in a long drain.

Beyond the audit, the human set the shape of the loop:
- **Continuous, not a session ritual.** The loop runs until asked to stop, with no "watch for new work?" offer at the end. What `clm wrap-up` does (removing landed work, freeing stale places, naming complete changes, reporting unsafe work and ledger findings) is housekeeping that runs at every step, so there is nothing to wrap up; one summary at the end is enough.
- **Resumable anywhere.** Interrupted, crashed, ended or cleared, a fresh session's `/clm:build` resumes the loop.
- **What belongs to the AI is in the loop; what belongs to the human is not.** The loop escalates to the human when it must, and every escalation waits in sight, never only in the chat history.
- **Stop on request,** from the chat or from the panel: drain (finish what runs, launch nothing new) or stop now.
- **`clm verify` runs as the project's `[verify]` says,** at the moments it names. GitHub intake and upgrade migrations stay the human's (`/clm:triage`, `/clm:upgrade`).

Two other ideas from the audit were tested and dropped before this design:
- **A shared compile cache.** A fresh worktree compiles the whole workspace from nothing in 19 s on this machine (16 s with a warm cache), a 4.3 GB target since `debug = 0`, so a cache saves nothing a build notices.
- **Holding a component while a spec branch naming it is in flight.** Over 9 days only 6 of 370 builds (2%) started while such a spec was in flight, each saving one rebuild for a wait of 4 to 19 minutes. The repeated builds of a component come from specs written after its build started, which no hold can foresee.

## Approaches

The shape is fixed by the constraints: the session launches the builds, since only the agent harness's subagents give each build the human's permission mode, its own worktree and the panel's rows (`builds-in-subagents`), and `clm` decides, since rules live in `clm` (`agent-harness-adapters`). `clm runner` as the loop would bring back headless sessions for attended builds, which `builds-in-subagents` dropped; the panel's hooks module driving the loop would put rules in the agent harness. The choice was how the stepper knows what's next:

- **A. Recompute the plan at every step (chosen).** `clm build step` computes the plan afresh from `origin/main` (the queue, `clm triage`'s groups, complete changes, verify triggers), the places and a small loop file that holds only what git can't (return history, drafted items, the stop request, the heartbeat, conditions that clear themselves). `clm view queue` renders the same plan, so the panel and the loop can't disagree, and resuming is running a step. A step fetches and reads the tree, about a second, which a loop waiting on builds that take many minutes doesn't notice.
- **B. An explicit queue in the loop file**, entries added when work appears and removed when launched. Cheaper per step and easy to show, but stale against git whenever another clone lands, claims or files an item, so it must be recomputed anyway, with code to reconcile the two: the in-context state problem moved into a file.

Chosen: A, since git is already the queue's source (`git-is-truth`) and the loop file then holds only what git can't.

## Design

### Components

- **`orchestrator.loop`**, new, a child of `orchestrator`, crate `crates/clm-loop`: `clm build` and its subcommands, the loop file, the lease, the return rules and the plan. The plan is a pure function over a snapshot (the queue, the triage groups, the complete changes, the places, the loop file, whether a verify run is due, the agent harness's abilities and the human's items), so the scheduler is tested without git; a thin layer gathers the snapshot from git and the clone. It runs `clm next`, `clm triage`, `clm run`, `clm draft`, `clm wrap-up`, `clm verify`, `clm upgrade` and `clm land` as subprocesses through their Contracts, and uses only `orchestrator`'s places, `agent-harness`'s abilities and `view`'s queue model as libraries.
- **`view`**: the `queue` view's model and renderers, the model a plain type with no dependency it lacks today.
- **`view.repo`**: `clm view queue`, reading the snapshot through `orchestrator.loop`.
- **`orchestrator`**: `next-wait` removed.
- **`orchestrator.triage`**: `clm draft --resume`, which continues an orphaned or rebuilt draft on the spec branch it pushed.
- **`orchestrator.run`**: its Contract names the loop, not the skill, as what runs it.
- **`plugin.build`**: the skill rewritten as above; the requirements whose rules move into `orchestrator.loop` removed.
- **`plugin.panel`**: the Queue tab, its buttons and the build rows' new place.
- **`init`, `upgrade`**: the allow rule and its migration.
- **`clm`**: dispatching `clm build`, and allowing its two rules in this repository's `.claude/settings.json` before `init` seeds them. The glossary gains the loop's terms.

### The loop and its lease

One loop per clone. Its state is the loop file, `<git common dir>/clm/loop`, beside `clm/builds/` (`build-places`), shared by every worktree of the clone and written under the places' lock. It holds:
- the **lease**: the holder's heartbeat, renewed by every `clm build` command the holder runs and every interval by its `clm build wait`;
- the **runs**: each entry the loop launched (build, draft, fold, archive, verify) with its name, kind, the agent harness's task ID once the session reports it, its model and when it started;
- the **history**: each return's result and cause, per component or draft;
- the **drafted items**, so an item a draft of the loop took is never drafted again by the loop, even after a draft that left it released its claim;
- the **review drafts** waiting to be folded, and the fold batches waiting for approval;
- the **conditions** that clear themselves (a full disk, a usage limit until a time), and the **stop request**.

**A run's states.** A run is *planned* from the step that prints its `launch` until the session reports it with `clm build launched`, then *running* until `clm build returned` records its return, and then *returned*. A step prints a planned run's `launch` again, unchanged, until it is reported, and never prepares that component or draft a second time, so a session that runs `step` twice, or loses an action to compaction, launches each run once. On a `start` that adopts a stale loop, planned and running runs alike are orphans: the session that held them is gone, and its subagents with it.

**Taking over a live lease.** `--take-over` is the human's word that the other session is gone, but its subagents may still be running when it isn't. So an orphan that a take-over found, whose branch on `origin` the last fetch saw advance within the lease's 15 minutes, is relaunched only once its branch has stayed still for 15 minutes; until then the queue view shows it under Running as `held by a session taken over`. A build that two subagents work at once would push over each other's branch.

Deleting the loop file loses the history, the drafted set, the fold list and batches, the loop's red verify results and unlanded escalations (review drafts stay on their spec branches for `/clm:triage`, red results in the ledger for `clm intake`): the claims, branches, places and backlog are in git or the places, and the next step rebuilds the rest.

`clm build start` begins or adopts the loop. When the lease is live (renewed within 15 minutes), another session holds the loop: `start` refuses, naming when it was last renewed, and the session shows the queue instead (`clm view queue`); `--take-over` takes it anyway, for a human who knows the other session is gone. When the lease is stale or there is no loop, `start` takes it, and every run the file lists as running belongs to a session that is gone, an *orphan*: its subagent died with that session, but its claim, branch and place remain, so the next step relaunches it (`clm run` resumes a build's branch; `clm draft` a draft's), which counts as a resume, never as a stop. The lease's 15 minutes is a constant, not a setting (`yagni`): it only has to outlast the wait's interval.

### The stepper

`clm build step` fetches `origin`, does the housekeeping, computes the plan and prints the actions the session must take now, one per line, then exits. Everything that needs no model it does itself, by running the other commands as subprocesses through their Contracts, as `clm runner` does: `clm next`, `clm view flight --format json` for priorities, `clm triage`, `clm run` and `clm run --finish`, `clm draft`, `clm draft --resume`, `clm draft --finish` and `--fold`, `clm wrap-up`, `clm verify --due --dry-run`, `clm upgrade` and `clm land`. So every rule of those commands stays theirs and the loop's build edits no other component's files. It prepares each run (the command claims, takes the place and writes the prompt), records each action it prints, and stores the plan it acted on in the loop file, which `clm view queue` shows. The session's work is to carry out the actions:
- `launch <name> <kind> <model> <prompt-file>`: start a background subagent with the launcher in that file, in its own worktree, with that model; then run `clm build launched <name> <task-id>`;
- `resume <name> <task-id> <message-file>`: send the subagent that message;
- `stop <name> <task-id>`: stop the subagent;
- `run <name> <command>`: run that `clm` command in the background (used for `clm verify`), reporting its last line as the run's result;
- `wait`: run `clm build wait` in the background when none is running;
- `say <file>`: show the human that text once (the end summary, a notice that a condition cleared);
- `end`: the loop has stopped; nothing more to run.

`clm build returned <name> --result <closing line> [--cause <cause>]` records a return and prints the next actions as `step` does. `<closing line>` is the subagent's last line as it stands (`landed <sha>`, `stopped: <reason>`, `waiting: <command>`, `proposed <id> review <url>`, or `none`); `<cause>` is the session's reading of why a run that didn't land ended, one of `limit` (with `--until <time>` when the error says when it resets), `disk`, `outage`, `tool`, `work`, `context`, `blocked`, `stopped` (the loop stopped it), or `running` (a `landed` run whose task still runs). The cause is the one judgement left to the session, since only a model can read a stop's reason; `clm` keeps the counts and decides.

**Return rules**, in `clm`, replacing the skill's prose:
- `waiting: <command>`: the run is still running; nothing changes, and no place is freed.
- `landed <sha>` while the agent harness still lists the run as running background work: `stop` it, then finish it as landed.
- A run returned with cause `stopped` keeps its claim, branch and place, unfinished, and the next `/clm:build` relaunches it.
- Every other return is finished as `clm run --finish` or `clm draft --finish` finishes it today (the place freed, a landed run's worktree removed), and a run with no closing line is landed exactly when the finish says so (`unclosed-build-reads-finish`).
- A stop is **resumed** in the same subagent when its cause is outside the work: `limit` and `disk` each time they recur, once the condition clears (the reset time passes, or the human says the disk is freed); `outage` and `tool` once, at once; `work` once. A stop is **rebuilt** with a new subagent on the session's model when its cause is `context`, a second `work` stop, or a second `outage` or `tool` stop. When the same limit or full disk stops a second run while it holds, the stopped runs are resumed one at a time, and nothing new launches until they have returned (`resume-sound-builds`).
- When a rebuilt run stops again for the cause that rebuilt it, or a run runs out of context twice, the loop launches it no more and **escalates** (stops the loop made, limits, full disks and blocked runs never escalate): it files a `question` backlog item on the component, whose `blocks` names the component's pending requirements and whose body names the stops, so the queue holds the component back until the human answers or deletes the item (`requirement-blocks`). The stepper files it itself, with no subagent and no place: it commits a tree that adds the item to `origin/main`'s, built through a temporary index, with `git commit-tree` (as a claim is made), pushes it as the branch `loop/<slug>` and lands that branch with `clm land`, run from the clone's main worktree as a subprocess, then deletes the branch. The branch adds one backlog item and nothing else, so its gate prints `spec`, a spec-path-only edit that needs no review, and it lands on checks as any small change outside `impl/`, `spec/`, `propose/` and `autosync/` does (`small-change-no-claim`, `yes-no-review`); `gate` needs no rule for `loop/*`. A land that fails leaves the escalation in the loop file, shown in the human's queue, and the next step tries again.
- `blocked`: nothing is relaunched; the queue already holds the component back by the item the build filed.

**The plan**, in order of what the next step launches. Up to the free places, the step launches the entries below in one list sorted by priority: each ready component `clm next` gives, at its effective priority, and each `clm triage` `group` line whose items the loop hasn't drafted, at its priority; at equal priority a draft goes first, since a bug's spec fix can make its component pending. A fold, an archive, a verify run and an escalation take no place, and are launched when due:
- a **fold** of the review drafts the loop's drafts left, once no draft of the loop is running (no longer once every build has returned), as `clm draft --fold` writes it; its `proposed <batch> review <url>` puts the batch in the human's queue;
- the **land** of a fold batch the human has approved: after `clm approve` through the guard (`approval-policy`), the session reports `clm build approved <batch>`, and the loop runs `clm land spec/<batch>`, which judges the approval, then `clm draft --finish` for each folded draft;
- an **archive** of the changes complete on `origin/main`, as a subagent on the build model that cuts `impl/archive/<id>` in its own checkout with `git switch -c` (or continues it), archives them and lands it, as the skill did (`build-archive-branch`);
- a **verify** run when `clm verify --due --dry-run` says one is due, after a land the loop saw, with `--event work-done` when the loop is idle (nothing running, nothing ready). A red result is the human's to take in (`slow-tests-after-land`), so it joins the human's queue.

Builds, folds and archives launch on the build model, `sonnet` as `clm agent-harness` names it, and rebuilds and drafts on the session's model, which the session names with `clm build start --session-model <alias>` (`model-per-launch`). The stepper reads `clm agent-harness` itself: where `subagents` is `no`, it launches one run at a time and its `launch` lines say to run the prompt in the session itself; where `isolated-worktrees` is `no`, it adds the worktree under `.claude/worktrees/agent-<name>` before printing the `launch`; where `model-choice` is `no`, every run is on the session's model (`skills-follow-agent-harness`).

**Housekeeping at every step.** Each step does what `clm wrap-up` does that changes nothing of the human's: removes landed builds' worktrees and branches, frees the places that don't count, and finds the complete changes to archive, by running `clm wrap-up` itself. What wrap-up only reports (unsafe work, ledger findings) joins the human's queue. `clm wrap-up` stays as it is for the other skills and for the human.

### Waiting and stopping

`clm build wait` runs in the background whenever the loop has nothing to do this moment: it renews the lease every interval and exits, printing why, when a step would launch something (a place frees and work is ready, new work lands, a draft becomes possible, a condition clears or its time passes, a fold is due) or a stop is requested. The session then runs `clm build step`. It reads the loop file and the places every few seconds, which needs no network, so a stop request or a place another session on this clone frees wakes it at once, and fetches `origin` once per interval (5 minutes by default, at most 7 so it renews the 15-minute lease in time) for work that lands elsewhere. It replaces `clm next --wait`, which only the build skill ran, so `clm next` then refuses `--wait` and `--interval`: a requirement that waits until the rewritten skill is current, so no land removes the flag while the shipped skill still runs it (`expand-contract`). It differs from it in waking on everything the plan launches, drafts included, while builds still run, which closes the idle-place gap.

`clm build stop --drain` records a drain: the plan launches nothing new; what runs finishes; the review drafts are folded; then the step says the summary and `end`. `clm build stop --now` records a stop: the next step prints `stop` for every running run and, once they are returned with cause `stopped`, the summary and `end`. Their claims, branches and places stay, so the next `/clm:build` resumes them as orphans. Either is run by the session when the human asks in the chat, or by the panel's buttons; `clm build wait` exits on it, so the session acts at once. A new `clm build start` clears a stop request.

### The human's queue

Everything that needs the human is computed into the plan, in sight until it is resolved, never only said in the chat:
- from git: the fold batches waiting for approval; `question` and `human-task` items that block requirements, with what they block, and the loop's escalations among them; the red results of the loop's own verify runs until an item takes them in; unsafe work and ledger findings, as `clm wrap-up` computes them; pending upgrade migrations (`clm upgrade`'s list), naming `/clm:upgrade`;
- from the loop file: the conditions that need the human (a full disk to free), and those that will clear by themselves, with when.

Intake (`clm intake`) reads the host and stays `/clm:triage`'s: the queue doesn't call the host.

### The queue view

`clm view queue` renders the plan as the other views render theirs (`views-rendered-by-clm`), in `text`, `ansi` and `json`, reading the local refs and the clone's loop file and places, never fetching, with the checkout line (`views-name-their-commit`) and `--at <rev>` as `view.repo` gives every view. Its sections:
- **Running**: each run with its kind, model, round and how long it has run;
- **Next up**: in the order the next step launches them, each with its priority and the reason for its place (`P1 · pulled forward by <change>`, `waits for a place`);
- **Waiting on you**: the human's queue, oldest first, each with what to do (`say "approve <batch>"`, `answer backlog/<slug>`, `free disk`, `run /clm:upgrade`);
- **Waiting on work**: components held by a `Needs` or a lapsing claim, and the free places.

The plan is a model of `view` (a plain data type with no git, so the crate still builds for `wasm32-unknown-unknown`), computed by `orchestrator.loop`, which stores the plan each step acted on in the loop file; `view.repo` reads that stored plan with the runs as they stand, so the view shows the order the loop computed, never one of its own, and imports run one way: `view` below `orchestrator.loop` below `view.repo`. With no loop on the clone, the view lists the ready components alone. The queue tab has no pressable lines and no inspector.

### The panel

The pane gains a **Queue** tab, first, drawing `clm view queue --format ansi --at origin/main`, and opens on it when `/clm:build` starts (`build-opens-panel`). The build rows (status, model, context, tokens, cost, time) move from the Flight tab's component lines to the Queue tab's Running entries, matched by the run's name. Above the sections, two buttons, **Drain** and **Stop now**, run `clm build stop --drain` and `clm build stop --now`; the pane decides nothing: the loop file records the request, and `clm build wait` wakes the session. Spec, Flight and Blockers stay.

### The skill

`/clm:build` shrinks to: tell the human once about the permission mode and the allow rules (as today), run `clm build start --session-model <alias>`, then carry out each action every `clm build` command prints and report each return with `clm build returned`, until `end`. When the human asks in the chat to drain or stop, run `clm build stop`. When the human approves a fold batch, run `clm approve` through the guard. Never decide a launch, a retry or an order itself.

### Unattended runner

`clm runner` keeps its own loop for now. Moving it onto the stepper needs a spec change of its own (`backlog/runner-loop-on-the-stepper`, `kind: idea`), so this change stays one reviewable piece.

### Allow rules

`clm build` joins `clm init`'s seeded allow rules for unattended steps (`unattended-steps-run-clm`), since the loop runs its steps without the human, and an `advised` migration adds the rule to projects set up by an older `clm` (`upgrades-carry-migrations`).

## Goals, non-goals, anti-goals

- **Goals:** every scheduling decision in `clm`; a loop that resumes after any interruption; no idle place while work is ready; priority honoured across drafts and builds; nothing for the human only in the chat; a view of what runs, what's next and what waits.
- **Non-goals:** moving `clm runner` onto the stepper; GitHub intake and upgrades inside the loop; a compile cache; holding builds for specs in flight.
- **Anti-goals:** the loop approving or landing anything that needs review without the human's yes through the guard; the panel deciding anything; two loops on one clone without the human's `--take-over`.

## Left to the builder

- The loop file's format (one `key value` per line as the place files, or JSON), so long as it is written atomically under the places' lock.
- The wait's interval (the `next-wait` default of 5 minutes is the starting point) and how `clm build wait` sees a stop request at once (a file watch or a short poll).
- The exact text of each `say` and each line of the queue view, within the sections above.

## Backlog

- The workaround flag on `plugin.panel/build-opens-panel` (sampling timer not started on open) no longer applies: `machine-charts` is built and `openPane` starts the sampling timer, so this change, which rewrites that requirement, deletes the flag and the `reconsider-plugin.panel-build-opens-panel` question that rested on it.
- `backlog/runner-loop-on-the-stepper` (`kind: idea`) records moving `clm runner` onto the stepper.

## Related

- `backlog/nothing-left-only-in-the-chat` asks every skill to review its conversation before a session ends. This change makes the build loop keep nothing for the human only in the chat; it doesn't review conversations, so that item stays.
