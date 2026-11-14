---
title: Build loop
status: active
owners: ["@clunny"]
depends_on: [orchestrator, orchestrator.run, orchestrator.triage, orchestrator.checkpoint, verify, upgrade, land, view, agent-harness, config, vcs, model]
paths: ["crates/clm-loop/**"]
successor: ""
serves: [capability/build-from-spec, architecture/agent-harness-adapters, architecture/git-is-truth]
---

## Contract
`clm build` runs the clone's build loop (`build-loop-in-clm`): it decides every step of draining the queue continuously (what to launch, in what order, on which model, how to handle a run's return, when to fold, archive and verify, and when to escalate to the human) and prints the actions the session must take with the agent harness's subagents. The session carries out the actions and reports what comes back; it decides nothing. There is one loop per clone, held by a lease, and its state is in git and the loop file, so any session can resume it.

- `clm build start --session-model <alias> [--take-over]` begins or adopts the loop and prints the first actions.
- `clm build step` prints the actions to take now.
- `clm build launched <name> <task-id>` records the agent harness's task ID for a run the session launched.
- `clm build returned <name> --result <line> [--cause <cause> [--until <time>]]` records a run's return and prints the actions to take now.
- `clm build approved <batch>` records that the human approved a fold batch through `clm approve`, and prints the actions to take now.
- `clm build cleared disk` records that the human freed the disk, and prints the actions to take now.
- `clm build wait [--interval <minutes>]` waits until a step would act, renewing the lease, and prints why it woke.
- `clm build stop --drain|--now` records a request to drain or to stop the loop.

An action is one line on stdout: `launch <name> <kind> <model> <prompt-file>`, `resume <name> <task-id> <message-file>`, `stop <name> <task-id>`, `run <name> <command>`, `wait`, `say <file>` or `end`. A run's `<name>` is `build:<component>`, `draft:<change-id>`, `fold:<first-id>`, `archive:<id>`, `land:<batch>` or `verify`; its `<kind>` is `build`, `draft`, `fold` or `archive`. A `--cause` is the session's reading of why a run that didn't land ended: `limit` (with `--until <time>` when the error says when it lifts), `disk`, `outage`, `tool`, `work`, `context`, `blocked`, `stopped` (the loop stopped it), or `running` for a run whose task the agent harness still lists as running background work. `start`, `step`, `returned`, `approved`, `cleared` and `wait` fetch `origin`; `launched` and `stop` don't. Every command exits 0 on success, 1 when the work isn't possible (with the reason on stderr), and 2 when git fails.

As a library it offers `plan(snapshot)`, a pure function from what a step read to `view`'s queue model, and `stored_plan(root)`, the plan the clone's last step stored with its runs as the loop file holds them now, read without fetching, so `view.repo` shows what the loop acts on (`views-rendered-by-clm`).

## Design
Crate `clm-loop`, at `crates/clm-loop/`, dispatched by `crates/clm` as `clm build`, which passes it `max_builds` and its own executable as the `clm` it runs. It drives the other commands as subprocesses through their Contracts, reading their output as those Contracts state it: `clm next` for the ready components in priority order, `clm view flight --at origin/main --format json` for each component's priority, `clm triage` for the `group` lines and their priorities, `clm run <component> --review-model <alias>` and `clm run --finish`, `clm draft`, `clm draft --resume`, `clm draft --finish` and `clm draft --fold`, `clm wrap-up` for the housekeeping, the complete changes and what isn't safe, `clm verify --due --dry-run`, `clm upgrade` for the pending migrations, and `clm land`. So every rule of those commands stays theirs, the loop holds none of them, and its build changes no other component's files. Its own git calls (fetching, reading `origin/main`'s tree, the escalation's commit and push) go through `clm-git` (`git-plumbing-layer`).

**Why in `clm`** (`agent-harness-adapters`, `deterministic-enforcement`). The skill `build` used to hold the loop's rules as prose a model followed: return handling, retries, the order of drafts and builds, the fold, the archive and the watch. A model following prose got several of them wrong in practice, and its state lived only in its context, which compaction loses. Here each rule is code with a test, and the one judgement left to the session is the cause of a stop, which only a model can read from a subagent's output: `clm` keeps the counts and decides.

**The plan is recomputed** (`git-is-truth`). Each step computes the plan afresh from a snapshot: what the commands above printed after the step's fetch, the loop file and the time. `plan` is a pure function of it, so the scheduler is tested on snapshots built in the test with no git. The step stores the plan it acted on in the loop file with the `origin/main` commit and the time, and `clm view queue` shows that plan with the runs as they stand, so the view never shows an order the loop didn't compute. An explicit queue kept in the file was rejected: it goes stale whenever another clone lands, claims or files an item, so it would need recomputing anyway, plus reconciling.

**The loop file** is `<git common dir>/clm/loop`, beside `clm/builds/` (`build-places`), so every worktree of the clone shares it. It is written whole through a temporary file and a rename while holding its own lock, `<git common dir>/clm/loop.lock`, which is never held while a subprocess runs, so a `clm run` or `clm draft` the step starts, which takes the places' lock, can't deadlock against it. Its format is the builder's, with a `version` the loader checks. A file that can't be read or parsed is set aside as `loop.damaged` and the loop starts afresh, which loses the history, the drafted set, the conditions, the stored plan, the fold list and batches, the loop's red verify results and the escalations not yet landed. Claims, branches and places are in git and the places, and a step finds them again; review drafts a lost fold list held stay on their spec branches, which `clm proposals` lists and `/clm:triage` folds, a red verify result stays in the ledger for `clm intake`, and the next drain's stops escalate again. Losing it is a damaged file, not a step of the loop, so the cost is accepted rather than rebuilding those from git at every step.

**Steps.** A step fetches, runs `clm wrap-up` for the housekeeping, reads the rest of the snapshot, computes the plan, prepares each run the plan launches (`clm run` or `clm draft`, which claim, take the place and write the prompt), records each action in the loop file, stores the plan, and prints the actions. A run's state is `planned` from the step that prints its `launch` until `launched` reports it, `running` until `returned`, then `returned`, or `stopped` when it returned with cause `stopped`; a planned run's `launch` is printed again, unchanged, by every step until it is reported, and it is never prepared twice. So a session that runs `step` twice, or loses an action line to compaction, launches each run once.

**The lease.** The holder renews it with every command it runs and every interval while `clm build wait` runs; it is live for 15 minutes after the last renewal. 15 minutes is a constant, not a setting (`yagni`): it only has to outlast the wait's interval, which is at most 7 minutes. A session that dies takes its subagents and its `clm build wait` with it, so the lease lapses and the next `start` adopts the loop. Runs the file lists as planned, running or stopped belong to a session that is gone: *orphans*. A build orphan is relaunched with `clm run`, which resumes its branch; a draft orphan with `clm draft --resume`, which continues its spec branch (`orchestrator.triage/draft-resume`); a `land:` or `verify` orphan is run again, since `clm land` of a landed branch and `clm verify --due` are safe to repeat; a fold or archive orphan is dropped and the plan computes it again, its prompt continuing a branch it finds on `origin`. `--take-over` is the human's word that a live lease's session is gone; since that session's subagents may still be running when it isn't, an orphan whose branch the last fetch saw advance within the last 15 minutes is relaunched only once it has stayed still for 15 minutes, so two subagents never push over one branch.

**Stops.** Each run counts its stops by cause. What stopped from outside the work is resumed in the same subagent, keeping its context: a limit or a full disk each time it recurs, once it clears, and an outage or a tool failure once, at once. What went wrong in the work is given one resume, then a rebuild with a new subagent on the session's model. A rebuilt run that stops again for the cause that rebuilt it is escalated, as is a run that runs out of context twice. These are `resume-sound-builds`' criteria, now code.

**Escalations** need no subagent. A `question` item is a file the loop writes itself: it builds a tree from `origin/main`'s with the item added through a temporary index, as `orchestrator`'s `next` reads a tree, commits it with `git commit-tree`, pushes the commit as `loop/<slug>` and lands that branch with `clm land` from the main worktree. A branch outside `impl/`, `spec/`, `propose/` and `autosync/` that only adds a backlog item lands on checks as a small change (`small-change-no-claim`, `gate/backlog-additions`), so `gate` needs no rule for it; its `/` keeps it out of the claim namespaces (`nested-branches-not-claims`).

**Waiting.** `clm build wait` reads the loop file and the places every 5 seconds, which needs no network, so a stop request or a place freed on the clone wakes it at once, and fetches `origin` and reruns the snapshot's commands once per interval for work landed elsewhere. It replaces `clm next --wait`, which only the skill ran, and differs from it in waking on everything a step would launch, drafts, folds, archives and verify runs included, while runs are still going, so no place stays idle while work is ready.

**Tests** start the commands as processes on fixture repositories with a fake `clm` for the subprocesses, and test `plan` and the return rules on snapshots built in the test.

## Requirements

### Requirement: loop-file
`clm-loop` SHALL keep the loop's state in `<git common dir>/clm/loop`, written whole through a temporary file and a rename while holding the lock `<git common dir>/clm/loop.lock`, which it SHALL NOT hold while a subprocess runs. The file SHALL hold: the lease's last renewal; the `origin/main` commit when the loop started; each run with its name, kind, model, state (`planned`, `running`, `returned`, `stopped`), task ID once reported, the time it started, its round and its stops by cause; the backlog items drafts of the loop left; the changes drafts of the loop left for review and the fold batches waiting for approval or approved; the red verify results of the loop's verify runs; the escalations not yet landed; the conditions (`disk`, and `limit` with the time it lifts); the stop request; and the plan the last step acted on, with its `origin/main` commit and time. A file it can't read or parse SHALL be renamed `loop.damaged` and the loop SHALL start afresh, saying so on stderr.

#### Scenario: state survives the session
- GIVEN a loop that launched `build:a` and recorded its task ID, and the session ended
- WHEN a new session runs `clm build start --session-model opus` after the lease lapsed
- THEN the loop file still lists `build:a`, and the first step relaunches it as an orphan

#### Scenario: damaged file
- GIVEN `<git common dir>/clm/loop` holding bytes that don't parse
- WHEN `clm build start --session-model opus` runs
- THEN `loop.damaged` holds those bytes, stderr says the loop started afresh, and it exits 0

#### Scenario: no lock across a subprocess
- GIVEN a fake `clm run` that, while it runs, tries to take `<git common dir>/clm/loop.lock` without blocking
- WHEN `clm build step` prepares a build with it
- THEN the fake's attempt succeeds

#### Needs
- view/queue-view

### Requirement: loop-start
`clm build start --session-model <alias>` SHALL take the loop's lease when there is no loop or its lease lapsed (no renewal for 15 minutes), record the session's model, clear a stop request, mark every run listed as `planned`, `running` or `stopped` an orphan, and print the first step's actions. When the lease is live it SHALL refuse with exit 1, printing when it was last renewed and that `clm view queue` shows the loop, unless `--take-over` is given. The next step SHALL relaunch an orphan by its kind: a build with `clm run <component>`, which resumes its branch; a draft with `clm draft --resume <change-id>`; a `land:` or `verify` run by running it again; and a fold or archive by dropping it for the plan to compute again. A relaunched orphan SHALL count no stop.

#### Scenario: a live loop
- GIVEN a loop whose lease was renewed 2 minutes ago
- WHEN `clm build start --session-model opus` runs
- THEN it exits 1, stderr names the renewal and `clm view queue`, and the loop file is unchanged

#### Scenario: adopting a lapsed loop
- GIVEN a loop whose lease was renewed 20 minutes ago, listing `build:a` as running with a live claim `origin/impl/a`, and `draft:d` as running with `origin/spec/d`
- WHEN `clm build start --session-model opus` runs
- THEN it prints a `launch build:a build sonnet <prompt-file>` whose prompt resumes `impl/a`, and a `launch draft:d draft opus <prompt-file>` whose prompt continues `spec/d`, and the runs record no stop

#### Needs
- view/queue-view
- orchestrator.triage/draft-resume

### Requirement: take-over-holds-moving-orphans
`clm build start --take-over` SHALL take a live lease, and an orphan it finds whose branch on `origin` advanced within the 15 minutes before the fetch SHALL NOT be relaunched until the branch has stayed at the same commit for 15 minutes; until then the plan SHALL list it as running, held by a session taken over.

#### Scenario: a branch still moving
- GIVEN a live lease, and `origin/impl/a` pushed 3 minutes ago for a run `build:a`
- WHEN `clm build start --session-model opus --take-over` runs, and then `clm build step` runs 5 minutes later
- THEN neither prints a `launch` for `build:a`, and `clm view queue` lists it under Running as held by a session taken over

#### Needs
- view/queue-view

### Requirement: step-actions
`clm build step` SHALL fetch `origin`, do the housekeeping (`step-housekeeping`), read the snapshot, compute the plan, prepare each run the plan launches, record it as `planned`, store the plan, and print the actions, one per line, in the forms the Contract gives, the `launch` lines in the plan's order. The prompt file a `launch` names SHALL be the loop's own, under `.clm/prompts/loop/`, holding the line `loop run <name>` first and then, unchanged, the launcher `clm run`, `clm draft` or `clm draft --fold` printed for it, or the archive prompt (`plan-archive`), so the panel finds which entry a subagent belongs to (`plugin.panel/build-rows`). A run still `planned` SHALL be printed again unchanged by every step until `clm build launched` reports it, and SHALL NOT be prepared again. When the loop has nothing to launch, resume or stop and no `clm build wait` of the holder is running, it SHALL print `wait`. Every `clm build` command SHALL renew the lease.

The build of this requirement updates the capability test `capability/build-from-spec` (`crates/clm/tests/capability/build_from_spec.rs`) to drain its queue through `clm build start`, `step`, `launched` and `returned` rather than `clm next` and `clm run`, keeping its citation; the test of the new scenario *resume in a new session* is `capability`'s own build's (`capability-tests-follow-rules`).

#### Scenario: an unreported launch
- GIVEN a step that printed `launch build:a build sonnet <file>`, and no `launched` for it
- WHEN `clm build step` runs again
- THEN it prints the same line, and `clm run a` ran once

#### Scenario: launched
- GIVEN a planned run `build:a`
- WHEN `clm build launched build:a t-17` runs
- THEN the loop file lists `build:a` as running with task ID `t-17`

#### Scenario: the launcher names the run
- GIVEN a step that launches `build:a`
- WHEN its prompt file is read
- THEN its first line is `loop run build:a` and the rest is what `clm run a` printed

#### Needs
- view/queue-view

### Requirement: return-rules
`clm build returned <name> --result <line>` SHALL, by `<line>`, the run's last line as it stands: for `waiting: <command>`, leave the run running and free nothing; for `landed <sha>` with `--cause running`, print `stop <name> <task-id>` and then finish it as landed; for any line with `--cause stopped`, mark the run `stopped`, keeping its claim, branch and place, and finish nothing; otherwise finish the run with `clm run --finish <component>` or `clm draft --finish <change-id>`, and, for a run whose line is neither `landed <sha>`, `proposed <id> review` (with or without a URL) nor `stopped: <reason>`, count it as landed exactly when the finish prints `landed impl/<component>` (`unclosed-build-reads-finish`). It SHALL pass each `left <path>: ...` line a finish prints to a `say`, and record each `left <slug>: <reason>` line a draft's finish prints among the items drafts of the loop left. A draft's `proposed <id> review` line, with or without a URL after it, SHALL add `<id>` to the fold list.

#### Scenario: waiting
- GIVEN a running `build:a`
- WHEN `clm build returned build:a --result 'waiting: clm land impl/a'` runs
- THEN `build:a` is still running and its place still counts

#### Scenario: landed with a task left running
- GIVEN a running `build:a` with task ID `t-17`
- WHEN `clm build returned build:a --result 'landed 1a2b3c4' --cause running` runs
- THEN it prints `stop build:a t-17` first, and runs `clm run --finish a`

#### Scenario: no closing line, landed
- GIVEN a running `build:a` and a fake `clm run --finish a` that prints `landed impl/a`
- WHEN `clm build returned build:a --result none --cause context` runs
- THEN the run is returned as landed and records no stop

#### Scenario: stopped keeps its place
- GIVEN a running `build:a` and a stop requested with `--now`
- WHEN `clm build returned build:a --result none --cause stopped` runs
- THEN `build:a` is `stopped`, `clm run --finish a` didn't run, and its place still counts

#### Needs
- view/queue-view

### Requirement: stop-causes
For a run that returned without landing, `clm build returned` SHALL decide by `--cause` and the run's stops so far: `limit` and `disk` SHALL resume the same subagent each time they recur, once the condition clears (`--until <time>` passes for `limit`; `clm build cleared disk` for `disk`), recording the condition meanwhile; `outage` and `tool` SHALL resume at once the first time they stop the run, and `work` the first time; a second stop of the run by `outage`, `tool` or `work`, and a first by `context`, SHALL finish the run and rebuild it, launching a new subagent on the session's model with its round one higher; `blocked`, `stopped` and `running` SHALL relaunch nothing. A resume the session reports it couldn't deliver, `clm build returned <name> --result none --cause tool` for a run whose resume is outstanding, SHALL finish the run and rebuild it, counting a `tool` stop. A resume SHALL be a `resume <name> <task-id> <message-file>` action whose message says why the run stopped, that the cause has cleared, to carry on in its own worktree, and to run again any review or check subagent of its own the stop killed. When the same `limit` or `disk` condition stops a second run while it holds, the loop SHALL resume the runs it stopped one at a time, each once the previous has returned, and launch no new run until they all have (`resume-sound-builds`).

#### Scenario: a limit
- GIVEN a running `build:a`
- WHEN `clm build returned build:a --result none --cause limit --until 2026-11-13T14:00:00Z` runs at 13:00, and `clm build step` runs at 14:01
- THEN the first prints no action for `build:a` and keeps its place, and the second prints `resume build:a <task-id> <message-file>`

#### Scenario: a full disk cleared
- GIVEN `build:a` stopped with cause `disk`
- WHEN `clm build step` runs, and then `clm build cleared disk`
- THEN the step prints no `resume` and the plan's human queue holds the disk, and `cleared` prints `resume build:a <task-id> <message-file>`

#### Scenario: a second work stop
- GIVEN `build:a` in round 1, resumed once after a `work` stop
- WHEN `clm build returned build:a --result 'stopped: check fails' --cause work` runs
- THEN it runs `clm run --finish a` and prints a `launch build:a build opus <prompt-file>`, the run in round 2

#### Scenario: a recurring limit
- GIVEN `build:a` and `build:b` both stopped by `limit` until 14:00, and component `c` ready
- WHEN `clm build step` runs at 14:01, and again after `build:a` returns
- THEN the first prints one `resume`, for `build:a`, and no `launch`; the second prints the `resume` for `build:b`

#### Needs
- view/queue-view

### Requirement: repeated-stops-escalate
The loop SHALL escalate a run, launching it no more, when a rebuilt run (round 2 or more) stops for the same cause as the stop that rebuilt it, or when a run stops with cause `context` for the second time; `limit`, `disk`, `blocked`, `stopped` and `running` stops SHALL never escalate. To escalate, it SHALL file a `kind: question` backlog item whose `components` name the component (or the draft's items' components), whose `blocks` name the component's pending requirements (none for a draft), and whose body names the stops with their results, by committing a tree that adds the item to `origin/main`'s with `git commit-tree`, pushing that commit as `loop/<slug>`, landing the branch with `clm land loop/<slug>` run from the main worktree, and deleting the branch. A land that fails SHALL keep the escalation in the loop file, listed in the human's queue, and the next step SHALL try again.

#### Scenario: escalated
- GIVEN `build:a` stopped with cause `work`, resumed, stopped with `work` again and rebuilt in round 2, `a` pending `a/x`
- WHEN `clm build returned build:a --result 'stopped: check fails' --cause work` records the round-2 stop
- THEN a `kind: question` item on `a` whose `blocks` names `a/x` is on `origin/main`, no `loop/*` branch is left on `origin`, and the next step launches nothing for `a`

#### Scenario: stops the loop made never escalate
- GIVEN `build:a` returned twice with cause `stopped`
- WHEN the next step runs
- THEN no item is filed for `a`

#### Needs
- view/queue-view

### Requirement: plan-order
The plan SHALL launch, up to the clone's free places (`max_builds` less the places that count, as `orchestrator`'s `places` reads them), in one list sorted by priority (`P1` first): each component `clm next` prints, at the priority `clm view flight --at origin/main --format json` gives it, and each `group` line `clm triage` prints none of whose items a draft of the loop left, at the line's priority; at equal priority a draft SHALL come before a build, and otherwise each keeps its own command's order. A build SHALL launch on the build model `clm agent-harness` names, with `--review-model` the session's model, and a draft on the session's model (`model-per-launch`). It SHALL launch no new build or draft while a drain or a stop is requested. When `clm run` or `clm draft` exits 3 because another session took the place meanwhile, the step SHALL launch nothing more that step and print no action for that run.

#### Scenario: a P1 build before P4 drafts
- GIVEN `max_builds` 3, three `group` lines at `P4`, and component `a` ready at `P1`
- WHEN `clm build step` runs
- THEN its first `launch` is `build:a`, followed by two drafts

#### Scenario: a draft before a build at equal priority
- GIVEN `max_builds` 2, component `a` ready at `P3` and a `group` line at `P3`
- WHEN `clm build step` runs
- THEN it launches the draft first and then `build:a`

#### Scenario: an item a draft left
- GIVEN a draft of the loop that returned `left b: needs the human`, and `clm triage` printing a `group` line holding `b`
- WHEN `clm build step` runs
- THEN it launches no draft of that line

#### Needs
- view/queue-view

### Requirement: plan-fold
Once the fold list isn't empty and no draft of the loop is running, the plan SHALL launch one fold, `fold:<first-id>`, with the prompt `clm draft --fold` writes for the fold list's IDs in the order their `group` lines came, on the build model, taking no place, also while a drain is requested. Its `proposed <batch> review <url>` SHALL put the batch in the human's queue. After `clm build approved <batch>`, the plan SHALL print `run land:<batch> clm land spec/<batch>`; a land that reports `gate/approval` SHALL put the batch back in the human's queue, and a land that lands SHALL be followed by `clm draft --finish` for each folded draft.

#### Scenario: fold while builds run
- GIVEN drafts `x` and `y` returned `proposed x review <url>` and `proposed y review <url>`, no draft running, and `build:a` running
- WHEN `clm build step` runs
- THEN it prints a `launch fold:x fold sonnet <prompt-file>` for `x` then `y`

#### Scenario: approved batch
- GIVEN a fold batch `b` in the human's queue
- WHEN `clm build approved b` runs
- THEN it prints `run land:b clm land spec/b`

#### Needs
- view/queue-view

### Requirement: plan-archive
When `clm wrap-up` names complete changes, no `impl/archive/*` branch is on `origin` that a run of the loop doesn't hold, and neither a drain nor a stop is requested, the plan SHALL launch `archive:<id>`, `<id>` the first change it names, on the build model, taking no place, with a prompt that names `impl/archive/<id>` cut from `origin/main`.

#### Scenario: complete change
- GIVEN `clm wrap-up` printing `complete c`, and no `impl/archive/*` on `origin`
- WHEN `clm build step` runs
- THEN it prints `launch archive:c archive sonnet <prompt-file>`, whose prompt names `git switch -c impl/archive/c origin/main`

#### Needs
- view/queue-view

### Requirement: plan-verify
After a run of the loop landed, and when the loop becomes idle (no run running and nothing to launch), the plan SHALL run `clm verify --due --dry-run`, with `--event work-done` when idle, and, only when its stdout starts `verify due`, print `run verify clm verify --due` (with `--event work-done` when idle), also while a drain is requested. The run's last line SHALL be reported with `clm build returned verify --result <line>`. A `verify failed` line SHALL be recorded and put in the human's queue until a backlog item on `origin/main` names its commit in `verify_failures`, and SHALL NOT be drafted, since the human takes a slow test's break in (`slow-tests-after-land`).

#### Scenario: idle and due
- GIVEN nothing running or ready, and a fake `clm verify --due --dry-run --event work-done` that prints `verify due work-done` and exits 0
- WHEN `clm build step` runs
- THEN it prints `run verify clm verify --due --event work-done`

#### Scenario: red result in sight
- GIVEN `clm build returned verify --result 'verify failed 1a2b3c4 culprit 9f8e7d6 components a'`
- WHEN the plan is computed, and again after an item naming the commit in `verify_failures` lands
- THEN the first human queue holds the red result with `take in with /clm:triage`, and the second doesn't

#### Needs
- view/queue-view

### Requirement: step-housekeeping
Every step SHALL run `clm wrap-up`, which removes landed worktrees and branches and frees the places that don't count (`orchestrator.checkpoint/checkpoint`), pass each `left <path>: ...` line it prints to a `say` once, take the changes it names `complete` for `plan-archive`, and put in the human's queue each line it prints of the caller's own work that isn't safe (`dirty`, `unpushed`) and each ledger finding, leaving out the lines about other checkouts' and the loop's runs' work (`elsewhere`, `open`). It SHALL run `clm wrap-up` in the directory `clm build` runs in.

#### Scenario: unsafe work in sight
- GIVEN a fake `clm wrap-up` that prints `unpushed spec/x` and `wrap-up incomplete`
- WHEN `clm build step` runs
- THEN the plan's human queue holds `unpushed spec/x`

#### Needs
- view/queue-view

### Requirement: loop-follows-agent-harness
The loop SHALL read the agent harness's abilities (`agent-harness/abilities`) and follow them: where `subagents` is `no`, it SHALL launch one run at a time, its `launch` lines saying to run the prompt in the session itself; where `isolated-worktrees` is `no`, it SHALL add the run's worktree at `.claude/worktrees/agent-<name>` from `origin/main`, `<name>` with `:` as `-`, before printing the `launch`, and its prompt SHALL say to work there; where `model-choice` is `no`, every run SHALL be on the session's model. An agent harness that can't send a message to a subagent or stop one keeps every rule through the session's reports: an undelivered resume is rebuilt (`stop-causes`), and under `stop --now` a run the session can't stop is waited for and reported with cause `stopped`.

#### Scenario: no subagents
- GIVEN an agent harness whose `subagents` is `no`, and components `a` and `b` ready with `max_builds` 3
- WHEN `clm build step` runs
- THEN it prints one `launch`, for `build:a`, and none for `b` until `build:a` returns

#### Needs
- view/queue-view

### Requirement: build-wait
`clm build wait [--interval <minutes>]` (a positive decimal number of at most 7, default 5) SHALL renew the lease every interval and exit 0, printing one line naming why, as soon as a step would print an action: a place frees while work is ready, new work is on `origin`, a fold is due, a condition clears or its time passes, a draft becomes possible, or a stop is requested. It SHALL read the loop file and the places every 5 seconds and fetch `origin` and read the snapshot once per interval. When a fetch fails it SHALL print `clm build wait: fetch failed: <reason>` on stderr and go on. An `--interval` that isn't a positive number of at most 7 SHALL exit 1 before waiting.

#### Scenario: a place frees
- GIVEN `max_builds` 1, component `a` ready, and the one place held by `build:b`
- WHEN `clm build wait` runs and `build:b`'s place is freed
- THEN within 10 seconds it prints a line naming the freed place and exits 0

#### Scenario: a stop request
- GIVEN `clm build wait --interval 5` running
- WHEN `clm build stop --drain` runs
- THEN within 10 seconds it prints a line naming the drain and exits 0

#### Scenario: interval too long
- WHEN `clm build wait --interval 10` runs
- THEN it exits 1 before waiting

#### Needs
- view/queue-view

### Requirement: build-stop
`clm build stop --drain` SHALL record a drain: no step launches a new build, draft or archive; running runs finish and are handled as usual; the fold list is folded, an approved batch is landed and a due verify is run; and once nothing runs, the step prints the summary's `say` and `end`. `clm build stop --now` SHALL record a stop: the next step prints `stop <name> <task-id>` for every running run, each then returned with cause `stopped`, and once none is running the summary's `say` and `end`. Neither SHALL fetch.

#### Scenario: stop now
- GIVEN running `build:a` (task `t-1`) and `draft:c` (task `t-2`)
- WHEN `clm build stop --now` runs and then `clm build step`
- THEN the step prints `stop build:a t-1` and `stop draft:c t-2`, and after both are returned with `--cause stopped` the next step prints a `say` and `end`, and `origin/impl/a` still exists

#### Needs
- view/queue-view

### Requirement: end-summary
The summary a drain or a stop ends with SHALL say what the loop's runs landed since `start`, what is still running, stopped or held and on what, the human's queue as it stands, and each `workaround` item and `reconsider-<component>-<slug>` question added on `origin/main` since the commit the loop started at, pointing at `/clm:health`; and, when the human's queue holds no unsafe work, that the session can end.

#### Scenario: summary with a workaround
- GIVEN a drain in which `build:a` landed and its branch added `backlog/workaround-a-x-1.md`
- WHEN the last run returns
- THEN the summary names `a` as landed, names `workaround-a-x-1` and `/clm:health`, and says the session can end

#### Needs
- view/queue-view

### Requirement: humans-queue
The plan SHALL hold the human's queue, each entry with what the human does about it and since when, oldest first: the fold batches waiting for approval (`say "approve <batch>"`); the `question` and `human-task` items on `origin/main` that block requirements, with what they block (`answer backlog/<slug>`), since the commit that added them; the escalations the loop couldn't land; the red results of the loop's verify runs no item has taken in (`take in with /clm:triage`); the work `clm wrap-up` reports isn't safe and its ledger findings; the migrations `clm upgrade` lists (`run /clm:upgrade`); and the conditions only the human can clear (`free disk, then say so`). It SHALL NOT call the host, so intake isn't in it.

#### Scenario: escalated build in the queue
- GIVEN an item `backlog/a-stops-on-work.md` the loop filed, blocking `a/x`
- WHEN the plan is computed
- THEN the human's queue lists `a-stops-on-work` with `answer backlog/a-stops-on-work` and `a/x`

#### Needs
- view/queue-view

### Requirement: plan-snapshot
`clm-loop` SHALL offer `plan(snapshot)`, a pure function returning `view`'s queue model with the sections the view shows, in the order and with the reasons the steps act on, and `stored_plan(root)`, which reads the plan the last step stored and the runs as the loop file holds them now, without fetching and without creating or writing any file or directory, or none when the clone has no loop file. Of `clm-loop`'s code, only what writes the loop file SHALL create `<git common dir>/clm/`. `clm build step` SHALL act on, and store, `plan` of the snapshot it read after its fetch.

#### Scenario: view and step agree
- GIVEN a snapshot with `a` ready at `P2` and a `group` line at `P4`, `max_builds` 1
- WHEN `plan` is computed and `clm build step` runs on the same snapshot
- THEN the plan's next section starts with `build:a`, the step's first `launch` is `build:a`, and `stored_plan` returns that plan

#### Scenario: no loop has run
- GIVEN a clone with no `<git common dir>/clm/` directory
- WHEN `stored_plan` reads it
- THEN it returns none, and `<git common dir>/clm/` still doesn't exist

#### Needs
- view/queue-view
