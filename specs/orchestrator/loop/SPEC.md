---
title: Build loop
status: active
owners: ["@clunny"]
depends_on: [orchestrator, orchestrator.run, orchestrator.triage, orchestrator.checkpoint, verify, upgrade, land, view, agent-harness, config, vcs, host, model, machine]
paths: ["crates/clm-loop/**"]
successor: ""
serves: [capability/build-from-spec, architecture/agent-harness-adapters, architecture/git-is-truth]
---

## Contract
`clm build` runs the clone's build loop (`build-loop-in-clm`): it decides every step of draining the queue continuously (what to launch, in what order, on which model, how to handle a run's return, when to fold, archive and verify, and when to escalate to the human) and prints the actions the session must take with the agent harness's subagents. The session carries out the actions and reports what comes back; it decides nothing. There is one loop per clone, held by a lease, and its state is in git and the loop file, so any session can resume it.

- `clm build start --session-model <alias> [--take-over]` begins or adopts the loop and prints the first actions.
- `clm build step` prints the actions to take now.
- `clm build launched <name> <task-id>` records the agent harness's task ID for a run the session launched.
- `clm build returned <name> [--task <task-id>] --result <line> [--cause <cause> [--until <time>]]` records a run's return and prints the actions to take now; `--task` names the task the return came from, the ID the session reported with `clm build launched`.
- `clm build approved <batch>` records that the human approved a fold batch through `clm approve`, and prints the actions to take now.
- `clm build cleared disk` records that the human freed the disk, and prints the actions to take now.
- `clm build release <component>` records the human's word that a component's build is given up and its work discarded: it deletes the build's branch on `origin`, renames the clone's `impl/<component>` to `released/<component>/<time>`, keeping its commits and worktree for the human, frees its place and forgets its run, so the plan builds the component afresh from `origin/main` once it is ready, and prints the actions to take now.
- `clm build pause <name>` records the human's word that a run the loop printed and the session hasn't launched waits: no step prints it or plans another run of its name until `clm build pause --end <name>` records their word to go on. Both print the actions to take now.
- `clm build wait [--interval <minutes>]` waits until a step would act, renewing the lease, and prints why it woke.
- `clm build stop --drain|--now` records a request to drain or to stop the loop.

An action is one line on stdout: `launch <name> <kind> <model> <prompt-file>`, `resume <name> <task-id> <message-file>`, `stop <name> <task-id>`, `run <name> <command>`, `wait`, `say <file>` or `end`. A run's `<name>` is `build:<component>`, `draft:<change-id>`, `fold:<first-id>`, `archive:<id>`, `land:<batch>` or `verify`; its `<kind>` is `build`, `draft`, `fold` or `archive`. A `--cause` is the session's reading of why a run that didn't land ended: `limit` (with `--until <time>` when the error says when it lifts), `disk`, `outage`, `tool`, `work`, `context`, `blocked`, `stopped` (the loop stopped it), or `running` for a run whose task the agent harness still lists as running background work. `start`, `step`, `returned`, `approved`, `cleared`, `release`, `pause` and `wait` fetch `origin`; `launched` and `stop` don't. Every command exits 0 on success, 1 when the work isn't possible (with the reason on stderr), and 2 when git fails.

As a library it offers `plan(snapshot)`, a pure function from what a step read to `view`'s queue model, and `stored_plan(root)`, the plan the clone's last step stored with its runs as the loop file holds them now, read without fetching, so `view.repo` shows what the loop acts on (`views-rendered-by-clm`).

## Design
Crate `clm-loop`, at `crates/clm-loop/`, dispatched by `crates/clm` as `clm build`, which passes it `max_builds` and its own executable as the `clm` it runs. It drives the other commands as subprocesses through their Contracts, reading their output as those Contracts state it: `clm next` for the ready components in priority order, `clm view flight --at origin/main --format json` for each component's priority, `clm triage` for the `group` lines and their priorities, `clm run <component> --review-model <alias>` and `clm run --finish`, `clm draft`, `clm draft --resume`, `clm draft --finish` and `clm draft --fold`, `clm wrap-up` for the housekeeping, the complete changes and what isn't safe, `clm verify --due --dry-run`, `clm upgrade` for the pending migrations, `clm machine --format json` for the disk's `room` and `state`, and `clm land`. So every rule of those commands stays theirs, the loop holds none of them, and its build changes no other component's files. Its own git calls (fetching, reading `origin/main`'s tree, the escalation's commit and push) go through `clm-git` (`git-plumbing-layer`).

**Why in `clm`** (`agent-harness-adapters`, `deterministic-enforcement`). The skill `build` used to hold the loop's rules as prose a model followed: return handling, retries, the order of drafts and builds, the fold, the archive and the watch. A model following prose got several of them wrong in practice, and its state lived only in its context, which compaction loses. Here each rule is code with a test, and the one judgement left to the session is the cause of a stop, which only a model can read from a subagent's output: `clm` keeps the counts and decides.

**The plan is recomputed** (`git-is-truth`). Each step computes the plan afresh from a snapshot: what the commands above printed after the step's fetch, the loop file and the time. `plan` is a pure function of it, so the scheduler is tested on snapshots built in the test with no git. The step stores the plan it acted on in the loop file with the `origin/main` commit and the time, and `clm view queue` shows that plan with the runs as they stand, so the view never shows an order the loop didn't compute. An explicit queue kept in the file was rejected: it goes stale whenever another clone lands, claims or files an item, so it would need recomputing anyway, plus reconciling.

**The loop file** is `<git common dir>/clm/loop`, beside `clm/builds/` (`build-places`), so every worktree of the clone shares it. It is written whole through a temporary file and a rename while holding its own lock, `<git common dir>/clm/loop.lock`, which is never held while a subprocess runs, so a `clm run` or `clm draft` the step starts, which takes the places' lock, can't deadlock against it. Its format is the builder's, with a `version` the loader checks. A file that can't be read or parsed is set aside as `loop.damaged` and the loop starts afresh, which loses the history, the drafted set, the conditions, the stored plan, the fold list and batches, the loop's red verify results and the escalations not yet landed. Claims, branches and places are in git and the places, and a step finds them again; review drafts a lost fold list held stay on their spec branches, which `clm proposals` lists and `/clm:triage` folds, a red verify result stays in the ledger for `clm intake`, and the next drain's stops escalate again. Losing it is a damaged file, not a step of the loop, so the cost is accepted rather than rebuilding those from git at every step.

**Steps.** A step fetches, finishes the builds whose claim has left `origin` (below), runs `clm wrap-up` for the housekeeping, reads the rest of the snapshot, computes the plan, prepares each run the plan launches (`clm run` or `clm draft`, which claim, take the place and write the prompt), records each action in the loop file, stores the plan, and prints the actions. A run's state is `planned` from the step that prints its `launch` until `launched` reports it, `running` until `returned`, then `returned`, or `stopped` when it returned with cause `stopped`, and `paused` from `clm build pause` until `--end` (`pause`); a planned run's `launch` is printed again, unchanged, by every step until it is reported, and it is never prepared twice. So a session that runs `step` twice, or loses an action line to compaction, launches each run once.

**The lease.** The holder renews it with every command it runs and every interval while `clm build wait` runs; it is live for 15 minutes after the last renewal. 15 minutes is a constant, not a setting (`yagni`): it only has to outlast the wait's interval, which is at most 7 minutes. A session that dies takes its subagents and its `clm build wait` with it, so the lease lapses and the next `start` adopts the loop. Runs the file lists as planned, running or stopped belong to a session that is gone: *orphans*. A paused run is no orphan: it waits on the human's word, not on a session. A build orphan is relaunched with `clm run`, which resumes its branch; a draft orphan with `clm draft --resume`, which continues its spec branch (`orchestrator.triage/draft-resume`); a `land:` or `verify` orphan is run again, since `clm land` of a landed branch and `clm verify --due` are safe to repeat; a fold or archive orphan is dropped and the plan computes it again, its prompt continuing a branch it finds on `origin`. A build or draft orphan whose relaunch is refused, `clm run` or `clm draft --resume` exiting 1, has no subagent and no way to get one, so keeping it would keep the loop busy for ever: the step finishes it as a run that returned with no closing line, tells the human the reason once unless the finish finds it landed, and leaves its component or items to the plan, which launches them afresh when `clm next` or `clm triage` offers them (`loop-start`). `--take-over` is the human's word that a live lease's session is gone; since that session's subagents may still be running when it isn't, an orphan whose branch the last fetch saw advance within the last 15 minutes is relaunched only once it has stayed still for 15 minutes, so two subagents never push over one branch.

**Stops.** Each run counts its stops by cause. What stopped from outside the work is resumed in the same subagent, keeping its context: a limit or a full disk each time it recurs, once it clears, and an outage or a tool failure once, at once. What went wrong in the work is given one resume, then a rebuild with a new subagent on the session's model. A rebuilt run that stops again for the cause that rebuilt it is escalated, as is a run that runs out of context twice. These are `resume-sound-builds`' criteria, now code.

**The disk** (`loop-holds-launches-on-disk`). Builds that run beside each other fill the disk with their worktrees' build output, and a build that runs out of space fails its land check for a reason that has nothing to do with its code; once the disk is full, no build lands. `max_builds` bounds the disk only as a count, which can't know how much a build of this project takes, while `machine` measures it: its `room` is how many more builds free space covers once the running ones have grown to the largest build seen. So each step reads `clm machine --format json` with the rest of the snapshot and launches no more new builds and drafts than that `room`, and none at all while a `disk` condition holds, since a run already stopped on a full disk says the measure fell short. Before any build has been measured there is no `room`, and the loop holds back only while the disk reads `critical`, the state `machine` judges on the filesystem's size alone. A reading that fails, or a disk the system won't read (`unknown`), holds back nothing: the loop then starts builds by places, as it did before it read the disk. An orphan's relaunch isn't held back, since its place counts already and its worktree's growth is in what `room` sets aside. What a land check writes beyond the worktree (the shared land build directory, a browser suite's artifacts) isn't in the measure; the land directory is one, whatever the number of builds, and already takes its space, and the rest is small beside a build's output, so the `room` is not padded for it (`yagni`). The disk frees mostly when a build lands, `clm land` deleting its worktree's build output (`build-places`), so a step that holds work back on the disk while a run of the loop is going waits for it as for a place; only when none of the loop's runs is going, so nothing will free space, does the human's queue ask for disk. Not taken: `clm run` and `clm draft` refusing on the disk themselves, which would hold a rule of the loop's in two commands, and a free-space setting the human sets, which can't be right for every toolchain (`machine-load-measured-by-clm`).

**A build that is over with no return** (`claim-gone-ends-a-build`). The agent harness doesn't always deliver a subagent's final report, and a run the loop finished only on `clm build returned` would then stay `running` for ever. A build's work is over when its claim has left `origin`: `clm run` pushes the claim before the step prints the `launch`, a build pushes nothing but that branch, and only landing (`clm land` deletes the branch after the merge) or a deletion by hand takes it away. So before the housekeeping, while the local branch that tells a landed build from a deleted one is still there, every step finishes each `running` build whose `origin/impl/<component>` is gone or merged, as a run that returned with no closing line (`unclosed-build-reads-finish`). The loop reads the refs only to tell that the build is over; whether it landed stays `clm run --finish`'s to say, so there is one reading of landed. The test reads the remote-tracking refs alone and holds for as long as the run is listed, so it doesn't depend on the step that first sees the land: another session's `clm wrap-up` may have removed the local branch already, and the run is then finished without being counted as landed, which only leaves it out of the summary. Finishing early changes nothing a prompt return would have done, since the housekeeping removes a landed build's worktree at the same step anyway. The report may still arrive after the plan has launched the component again under the same name, and a report names a run only by that name. So `clm build returned` ignores a build's `landed <sha>` line whose commit is older than the run it would be recorded for: no run lands a commit made before it started (`return-rules`). Any other line from it, such as `none --cause context`, would finish or count a stop against the new run, so the session names the task a return came from with `--task`, and a return from a task the loop keeps as an earlier run's of the name changes nothing (`returned-ignores-an-earlier-task`), the same IDs `clm build launched` refuses (`launched-refuses-an-earlier-task`). The commit-time rule stays for a return that names no task. Not taken for this: naming each run uniquely, which changes every command, the prompt files and the queue for one case (`yagni`); and leaving the session to tell an old subagent's return from the new one's, which is a rule in a model's hands (`build-loop-in-clm`). The claim's state on `origin` wouldn't do for this, since another clone may claim the component between a land and its report. Not taken: an action asking the session to judge whether the run is over, which puts a rule of the loop back in a model's hands (`build-loop-in-clm`); a time limit on a run, which can't tell a long build from a lost report; reading `clm wrap-up`'s `removed impl/<component>` line, which is printed once and not at all for a worktree left dirty; and the same rule for drafts, since a draft whose spec change has landed may still be fixing an item on a code branch, so a landed change doesn't mean the draft is over.

**Releasing a build** (`release`). The human gives up a component's build when its session died and they don't want it resumed, or when it went wrong in a way no resume would mend. Deleting `origin/impl/<component>` alone isn't a release: `clm run` refuses a local `impl/<component>` that has no `origin/impl/<component>`, since it may hold commits nobody pushed (`orchestrator.run`'s `run`), so every step would skip the component until someone moved the local branch by hand, a step no skill states. So a release is one command, and the session needs no step of its own (`build-loop-in-clm`). It runs `clm branch delete impl/<component>` for the remote branch, so the rule that a clone deletes only a tip it pushed stays `host`'s, and a refusal there (another clone's claim, or a branch that moved since the fetch) leaves the clone as it was. It then renames the local `impl/<component>` to `released/<component>/<time>` with `git branch -m` through `clm-git`, which git allows while another worktree has the branch checked out and which moves that worktree's `HEAD` with it, and runs `clm run --finish <component>` to free the place, so the place's rules stay `orchestrator.run`'s. A release discards nothing the clone holds: the build's commits stay on the renamed branch and its worktree, with any uncommitted work, stays where it is, for the human to delete or take from, so the command needs no guard beyond the allow rule `clm build` already has, and no worktree rule of `orchestrator.run`'s moves into the loop. Running it again after a failure finishes the release, since `clm branch delete` of a branch already gone does nothing. Not taken: `clm branch delete` deleting the local branch too, which would give a command unattended sessions run with no classifier judging it (`unattended-steps-run-clm`) the power to discard commits nobody pushed; deleting the local branch and the worktree in the release, which would need a guard asking the human, since the same allow rule covers `clm build`, and would copy `run-finish`'s worktree rules into the loop; and `clm run` resuming a local branch with no remote, which would drop `run`'s guard against building on commits nobody pushed.

**Pausing a run** (`pause`). A pause is the human's, unlike a take-over's hold on a moving orphan, which is the loop's. A printed action is the loop's decision, and the session carries it out; but the human may know what the loop can't, such as that `clm verify --due` hangs until a fix lands. Without a command the session had two choices, both wrong: run a step the human said not to run, or leave the action unlaunched, a `planned` run that every step printed again and `clm view queue` listed as running with nothing behind it. `clm build pause <name>` records the human's word in the loop file: the run becomes `paused`, keeping its claim, branch, place and prompt, no step prints it, and since a run that hasn't returned already stops the plan from planning another of its name (`plan-verify`'s due check, an approved batch's land, a claimed component), nothing else is needed for that. It sits in the human's queue until `clm build pause --end <name>` makes it `planned` again, and the next action printed is the one the loop printed before. Only a `planned` run that isn't an orphan can be paused, since an orphan belongs to a session that is gone and the next step relaunches it: a launched run is the agent harness's to stop first (`clm build stop --now`, or the session stopping its task and reporting `--cause stopped`), after which what the loop prints again can be paused. A paused run doesn't keep a drain from ending, and the summary names it; it outlives the session, since the pause is the human's and not the session's; and `clm build release` gives up a paused build as it does a stopped one. Not taken: a pause until a condition, such as a change landing, which would need the loop to read conditions no command of its reports, for a case the human settles with one word (`yagni`); pausing by kind (every verify), which a single run's pause covers, since none of its name is planned while it is paused; and the session leaving the action alone, which keeps a run listed that nothing will launch.

**What the queue shows.** A run the loop has printed and the session hasn't reported launched is not running: nothing is behind it. So the queue model lists it at the head of the next section, as `printed, not launched yet`, and lists under running only runs that are `running` or `stopped`, which hold a subagent or a task, or a stopped one's claim and place, and the orphans, which the next step relaunches or a take-over holds (`take-over-holds-moving-orphans`); a paused run is in the human's queue alone (`queue-tells-printed-from-running`). This is the loop's choice of what to put in each section of `view`'s model, so `view` changes nothing.

**Escalations** need no subagent. A `question` item is a file the loop writes itself: it builds a tree from `origin/main`'s with the item added through a temporary index, as `orchestrator`'s `next` reads a tree, commits it with `git commit-tree`, pushes the commit as `loop/<slug>` and lands that branch with `clm land` from the main worktree. A branch outside `impl/`, `spec/`, `propose/` and `autosync/` that only adds a backlog item lands on checks as a small change (`small-change-no-claim`, `gate/backlog-additions`), so `gate` needs no rule for it; its `/` keeps it out of the claim namespaces (`nested-branches-not-claims`).

**Waiting.** `clm build wait` reads the loop file and the places every 5 seconds, which needs no network, so a stop request, a place freed on the clone or a running build whose claim has left `origin`, which the next step finishes, wakes it at once, before the wait runs any housekeeping of its own, and fetches `origin` and reruns the snapshot's commands once per interval for work landed elsewhere. It replaces `clm next --wait`, which only the skill ran, and differs from it in waking on everything a step would launch, drafts, folds, archives and verify runs included, while runs are still going, so no place stays idle while work is ready.

**Tests** start the commands as processes on fixture repositories with a fake `clm` for the subprocesses, and test `plan` and the return rules on snapshots built in the test. A fixture gives every build a scenario calls running a claim on `origin` that `origin/main` lacks, as `clm run` leaves one, unless the scenario says the claim is gone.

## Requirements

### Requirement: loop-file
`clm-loop` SHALL keep the loop's state in `<git common dir>/clm/loop`, written whole through a temporary file and a rename while holding the lock `<git common dir>/clm/loop.lock`, which it SHALL NOT hold while a subprocess runs. The file SHALL hold: the lease's last renewal; the `origin/main` commit when the loop started; each run with its name, kind, model, state (`planned`, `running`, `returned`, `stopped`, `paused`), task ID once reported, the time it started, the time it was paused, its round and its stops by cause; the backlog items drafts of the loop left; the changes drafts of the loop left for review and the fold batches waiting for approval or approved; the red verify results of the loop's verify runs; the escalations not yet landed; the conditions (`disk`, and `limit` with the time it lifts); the stop request; and the plan the last step acted on, with its `origin/main` commit and time. A file it can't read or parse SHALL be renamed `loop.damaged` and the loop SHALL start afresh, saying so on stderr.

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
`clm build start --session-model <alias>` SHALL take the loop's lease when there is no loop or its lease lapsed (no renewal for 15 minutes), record the session's model, clear a stop request, mark every run listed as `planned`, `running` or `stopped` an orphan, and print the first step's actions. When the lease is live it SHALL refuse with exit 1, printing when it was last renewed and that `clm view queue` shows the loop, unless `--take-over` is given. The next step SHALL relaunch an orphan by its kind: a build with `clm run <component>`, which resumes its branch; a draft with `clm draft --resume <change-id>`; a `land:` or `verify` run by running it again; and a fold or archive by dropping it for the plan to compute again. A relaunched orphan SHALL count no stop. When the relaunch of a build or draft orphan is refused, `clm run <component>` or `clm draft --resume <change-id>` exiting 1, the step SHALL finish the run as `return-rules` finishes a run that returned with no closing line and no cause (`clm run --finish <component>` or `clm draft --finish <change-id>`, counted as landed exactly when a build's finish prints `landed impl/<component>`), record it `returned`, count no stop, and, unless it counted the run as landed, pass `<name> wasn't relaunched: <reason>` to a `say`, `<reason>` what the refusing command printed on stderr; the plan SHALL then launch its component or items only as `plan-order` launches any other. An orphan whose relaunch exits 3, for want of a place, SHALL stay an orphan for the next step.

#### Scenario: a live loop
- GIVEN a loop whose lease was renewed 2 minutes ago
- WHEN `clm build start --session-model opus` runs
- THEN it exits 1, stderr names the renewal and `clm view queue`, and the loop file is unchanged

#### Scenario: adopting a lapsed loop
- GIVEN a loop whose lease was renewed 20 minutes ago, listing `build:a` as running with a live claim `origin/impl/a`, and `draft:d` as running with `origin/spec/d`
- WHEN `clm build start --session-model opus` runs
- THEN it prints a `launch build:a build sonnet <prompt-file>` whose prompt resumes `impl/a`, and a `launch draft:d draft opus <prompt-file>` whose prompt continues `spec/d`, and the runs record no stop

#### Scenario: a refused relaunch
- GIVEN a loop whose lease was renewed 20 minutes ago, listing `build:a` as running with a claim `origin/impl/a` that `origin/main` lacks, a fake `clm run a` that exits 1 saying `a has uncommitted changes in its worktree`, a fake `clm run --finish a` that prints no `landed` line, and nothing ready
- WHEN `clm build start --session-model opus` runs, then `clm build stop --drain` and `clm build step`
- THEN the start runs `clm run --finish a`, prints no `launch` for `build:a` and prints a `say` whose file names `build:a` and that reason, the loop file lists `build:a` as returned with no stop, and the step prints the summary's `say` and `end`

#### Scenario: a draft with nothing to resume
- GIVEN a loop whose lease was renewed 20 minutes ago, listing `draft:d` as running, and a fake `clm draft --resume d` that exits 1 saying the draft left nothing to resume
- WHEN `clm build start --session-model opus` runs
- THEN it runs `clm draft --finish d`, prints no `launch` for `draft:d` and prints a `say` whose file names `draft:d` and that reason, and the loop file lists `draft:d` as returned with no stop

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
- GIVEN a step that printed `launch build:a build <model> <file>`, and no `launched` for it
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
`clm build returned <name> --result <line>` SHALL, by `<line>`, the run's last line as it stands: for `waiting: <command>`, leave the run running and free nothing; for `landed <sha>` with `--cause running`, print `stop <name> <task-id>` and then finish it as landed; for any line with `--cause stopped`, mark the run `stopped`, keeping its claim, branch and place, and finish nothing; otherwise finish the run with `clm run --finish <component>` or `clm draft --finish <change-id>`, and, for a run whose line is neither `landed <sha>`, `proposed <id> review` (with or without a URL) nor `stopped: <reason>`, count it as landed exactly when the finish prints `landed impl/<component>` (`unclosed-build-reads-finish`). It SHALL pass each `left <path>: ...` line a finish prints to a `say`, and record each `left <slug>: <reason>` line a draft's finish prints among the items drafts of the loop left. A draft's `proposed <id> review` line, with or without a URL after it, SHALL add `<id>` to the fold list. Before any of these, a `landed <sha>` line for a build run SHALL change nothing but the lease, print no `stop`, and print the actions to take now, when `<sha>` names a commit the clone holds whose committer time is before the time the run started: no run lands a commit made before it started, so the line is the late report of an earlier run of that name, which the loop finished itself (`claim-gone-ends-a-build`). A `<sha>` the clone can't resolve SHALL be handled as any other `landed` line.

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

#### Scenario: a late report of a run the loop finished
- GIVEN a commit `c` on `origin/main` committed at 12:00, and a `build:a` started at 12:05, running with task ID `t-2`
- WHEN `clm build returned build:a --result 'landed <c>' --cause running` runs, `<c>` that commit's hash
- THEN `clm run --finish a` didn't run, it prints no `stop`, and `build:a` is still running

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
The plan SHALL launch, up to the clone's free places (`max_builds` less the places that count, as `orchestrator`'s `places` reads them), in one list sorted by priority (`P1` first): each component `clm next` prints, at the priority `clm view flight --at origin/main --format json` gives it, and each `group` line `clm triage` prints none of whose items a draft of the loop left, at the line's priority, save a draft that waits for a build of its component (`draft-waits-for-its-build`); at equal priority a draft SHALL come before a build, and otherwise each keeps its own command's order. A build SHALL launch on the build model `clm agent-harness` names, with `--review-model` the session's model, and a draft on the session's model (`model-per-launch`). It SHALL launch no new build or draft while a drain or a stop is requested. It SHALL also hold new builds and drafts back on the disk, as `clm machine --format json` read in the step's snapshot gives it: no more of them in a step than its `disk.room` when that is a number; none while `disk.room` is `null` and `disk.state` is `critical`; and none while a `disk` condition holds. A `clm machine` that exits non-zero or prints output that doesn't parse, or a `disk.state` of `unknown`, SHALL hold nothing back. An orphan's relaunch SHALL NOT be held back on the disk. The disk SHALL hold back only the entries the free places would launch, the room counting new launches alone: an entry beyond the free places SHALL keep the reason `waits for a place`, one the places would launch but the disk holds back SHALL be listed in the plan's next section with the reason `waits for disk`, and while stopped runs are being resumed one at a time every entry SHALL keep the reason `waits for stopped runs to resume`. When `clm run` or `clm draft` exits 3 because another session took the place meanwhile, the step SHALL launch nothing more that step and print no action for that run.

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

#### Scenario: room for one build
- GIVEN `max_builds` 3, no places held, components `a`, `b` and `c` ready at `P2`, and `clm machine --format json` giving `disk.room` 1 and `disk.state` `warn`
- WHEN `clm build step` runs
- THEN it prints `launch build:a` and no other `launch`, and the stored plan lists `build:b` and `build:c` with `waits for disk`

#### Scenario: a disk stop holds new builds
- GIVEN `max_builds` 3, `build:b` stopped with cause `disk`, component `a` ready, and `clm machine --format json` giving `disk.room` 2
- WHEN `clm build step` runs
- THEN it prints no `launch build:a`, and the stored plan lists `build:a` with `waits for disk`

#### Scenario: no measurement yet
- GIVEN `max_builds` 2, component `a` ready, and `clm machine --format json` giving `disk.room` `null`
- WHEN `clm build step` runs with `disk.state` `critical`, and again in a fresh fixture with `disk.state` `warn`
- THEN the first prints no `launch`, and the second prints `launch build:a`

#### Scenario: no reading
- GIVEN `max_builds` 2, component `a` ready, and a `clm machine` that exits 1
- WHEN `clm build step` runs
- THEN it prints `launch build:a`

#### Needs
- view/queue-view
- machine/disk-room

### Requirement: plan-fold
Once the fold list isn't empty and no draft of the loop is running, the plan SHALL launch one fold, `fold:<first-id>`, with the prompt `clm draft --fold` writes for the fold list's IDs in the order their `group` lines came, on the build model, taking no place, also while a drain is requested. Its `proposed <batch> review <url>` SHALL put the batch in the human's queue. After `clm build approved <batch>`, the plan SHALL print `run land:<batch> clm land spec/<batch>`; a land that reports `gate/approval` SHALL put the batch back in the human's queue, and a land that lands SHALL be followed by `clm draft --finish` for each folded draft.

#### Scenario: fold while builds run
- GIVEN drafts `x` and `y` returned `proposed x review <url>` and `proposed y review <url>`, no draft running, and `build:a` running
- WHEN `clm build step` runs
- THEN it prints a `launch fold:x fold <model> <prompt-file>` for `x` then `y`, `<model>` the build model `agent-harness/abilities` gives

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
- THEN it prints `launch archive:c archive <model> <prompt-file>`, `<model>` the build model `agent-harness/abilities` gives,, whose prompt names `git switch -c impl/archive/c origin/main`

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
`clm build wait [--interval <minutes>]` (a positive decimal number of at most 7, default 5) SHALL renew the lease every interval and exit 0, printing one line naming why, as soon as a step would print an action: a place frees while work is ready, the disk's room grows while work waits for disk, new work is on `origin`, a fold is due, a condition clears or its time passes, a draft becomes possible, or a stop is requested; and as soon as a running build's claim has left `origin`, as `claim-gone-ends-a-build` judges it on the remote-tracking refs at each of those 5-second reads, naming the run. It SHALL make that judgement before each run of the housekeeping, its first included, so the step that finishes the build finds its local branch. It SHALL read the loop file, the places and those refs every 5 seconds and fetch `origin` and read the snapshot, `clm machine`'s disk included, once per interval. When a fetch fails it SHALL print `clm build wait: fetch failed: <reason>` on stderr and go on. An `--interval` that isn't a positive number of at most 7 SHALL exit 1 before waiting.

#### Scenario: a place frees
- GIVEN `max_builds` 1, component `a` ready, and the one place held by `build:b`
- WHEN `clm build wait` runs and `build:b`'s place is freed
- THEN within 10 seconds it prints a line naming the freed place and exits 0

#### Scenario: the disk frees
- GIVEN `max_builds` 2, component `a` ready, no places held, and `clm machine --format json` giving `disk.room` 0
- WHEN `clm build wait --interval 0.1` runs and `clm machine` then gives `disk.room` 1
- THEN within 20 seconds it prints a line naming the disk and exits 0

#### Scenario: a build lands
- GIVEN a running `build:a` with a claim `origin/impl/a` that `origin/main` lacks, and nothing ready
- WHEN `clm build wait` runs, and the clone's `origin/main` gains a merge of `impl/a` and its `origin/impl/a` is deleted, as a `clm land impl/a` run in the clone leaves them
- THEN within 10 seconds it prints a line naming `build:a` and exits 0

#### Scenario: a stop request
- GIVEN `clm build wait --interval 5` running
- WHEN `clm build stop --drain` runs
- THEN within 10 seconds it prints a line naming the drain and exits 0

#### Scenario: interval too long
- WHEN `clm build wait --interval 10` runs
- THEN it exits 1 before waiting

#### Needs
- view/queue-view
- machine/disk-room

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
The plan SHALL hold the human's queue, each entry with what the human does about it and since when, oldest first: the fold batches waiting for approval (`say "approve <batch>"`); the `question` and `human-task` items on `origin/main` that block requirements, with what they block (`answer backlog/<slug>`), since the commit that added them; the escalations the loop couldn't land; the red results of the loop's verify runs no item has taken in (`take in with /clm:triage`); the work `clm wrap-up` reports isn't safe and its ledger findings; the migrations `clm upgrade` lists (`run /clm:upgrade`); the conditions only the human can clear (`free disk, then say so`); each run the human paused (`pause`), since it was paused (`say "go on with <name>"`); and, when the disk holds back ready work (`plan-order`) while no run of the loop, of any kind, is planned or running, so no land will free space, since the step's time, the disk (`free disk; the loop goes on once a build fits`). It SHALL NOT call the host, so intake isn't in it.

#### Scenario: escalated build in the queue
- GIVEN an item `backlog/a-stops-on-work.md` the loop filed, blocking `a/x`
- WHEN the plan is computed
- THEN the human's queue lists `a-stops-on-work` with `answer backlog/a-stops-on-work` and `a/x`

#### Scenario: the disk holds work with nothing running
- GIVEN component `a` ready, no run of the loop planned or running, and a snapshot whose disk gives `room` 0
- WHEN the plan is computed
- THEN the human's queue lists the disk with `free disk; the loop goes on once a build fits`, and with `build:b` running instead it doesn't

#### Needs
- view/queue-view
- machine/disk-room

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

### Requirement: release
`clm build release <component>` SHALL fetch `origin` and, after refreshing the take-over holds as a step does, refuse with exit 1, changing nothing and naming the run, when the loop file lists `build:<component>` as `planned` and not an orphan, as `running` and not an orphan, or as an orphan a take-over holds (`take-over-holds-moving-orphans`); a run that is `stopped` or `returned`, or an orphan no take-over holds, doesn't refuse it. Otherwise it SHALL run `clm branch delete impl/<component>` and, when that exits other than 0, print its stderr and exit 1 when it exited 1 and 2 otherwise, changing nothing more. Then, when the local `impl/<component>` exists, it SHALL rename it to `released/<component>/<time>`, `<time>` the UTC time as `YYYYMMDDTHHMMSSZ`, with `git branch -m`, also when a worktree has it checked out, and print `kept released/<component>/<time>`; it SHALL remove no worktree and delete no local branch. It SHALL then run `clm run --finish <component>`, remove `build:<component>` from the loop file, and print the actions to take now as `clm build step` does. A component with no branch on `origin`, no local branch and no run SHALL be released with exit 0.

#### Scenario: a dead session's build released
- GIVEN the loop file listing `build:a` as an orphan no take-over holds, `origin/impl/a` pushed from this clone, and a worktree at `.claude/worktrees/agent-1` with the local `impl/a` checked out at a commit `c` that `origin/impl/a` lacks, and an untracked file `notes.txt`
- WHEN `clm build release a` runs
- THEN it exits 0, `origin` has no `impl/a`, the clone has no local `impl/a`, it printed `kept released/a/` with the time, that branch is at `c` and the worktree has it checked out with `notes.txt` still there, `clm run --finish a` ran, and the loop file lists no `build:a`

#### Scenario: a live run
- GIVEN the loop file listing `build:a` as running with task ID `t-1`, not an orphan, and `origin/impl/a` pushed from this clone
- WHEN `clm build release a` runs
- THEN it exits 1 naming `build:a`, `clm branch delete` didn't run, and the local `impl/a` and the loop file are unchanged

#### Scenario: another clone's claim
- GIVEN a `clm branch delete impl/a` that exits 1 saying this clone didn't push the tip, and a local `impl/a`
- WHEN `clm build release a` runs
- THEN it exits 1 with that reason on stderr, the local `impl/a` is unchanged, and `clm run --finish a` didn't run

#### Scenario: nothing to release
- GIVEN no `origin/impl/a`, no local `impl/a` and no run `build:a`
- WHEN `clm build release a` runs
- THEN it exits 0 and prints no `kept` line

### Requirement: claim-gone-ends-a-build
Every `clm build` command that prints the actions to take now SHALL, when it computes them, after the fetch and before the housekeeping (`step-housekeeping`), and so after `clm build returned` has recorded the return it was given, finish each build run in state `running`, an orphan or not, whose claim has left `origin`: the clone's remote-tracking refs hold no `origin/impl/<component>`, or hold one whose tip is an ancestor of `origin/main`. It SHALL finish it as `return-rules` finishes a run that returned with no closing line and no cause: run `clm run --finish <component>`, record the run `returned`, count it as landed exactly when the finish prints `landed impl/<component>`, and pass each `left <path>: ...` line the finish prints to a `say`. It SHALL count no stop for it and print no `stop` action for it. It SHALL leave as they are a build whose `origin/impl/<component>` holds a tip that isn't an ancestor of `origin/main`, and every run of another kind or in another state.

#### Scenario: a landed build whose return never arrived
- GIVEN a running `build:a` with task ID `t-1`, no `origin/impl/a`, a fake `clm run --finish a` that prints `landed impl/a`, and nothing else running or ready
- WHEN `clm build step` runs
- THEN `clm run --finish a` ran before `clm wrap-up`, the loop file lists `build:a` as returned and landed with no stop, and the step prints no `stop` and no `launch` for `build:a`

#### Scenario: a drain ends
- GIVEN the run of "a landed build whose return never arrived", and a drain requested
- WHEN `clm build step` runs
- THEN it prints the summary's `say`, which names `a` as landed, and `end`

#### Scenario: a merged claim still on origin
- GIVEN a running `build:a` whose `origin/impl/a` is the second parent of a merge on `origin/main`
- WHEN `clm build step` runs
- THEN `clm run --finish a` ran and the loop file lists `build:a` as returned

#### Scenario: a claim gone with nothing landed
- GIVEN a running `build:a`, no `origin/impl/a`, no local `impl/a`, and a fake `clm run --finish a` that prints no `landed` line
- WHEN `clm build step` runs
- THEN the loop file lists `build:a` as returned, not landed, with no stop

#### Scenario: a live claim
- GIVEN a running `build:a` whose claim `origin/impl/a` is a commit `origin/main` lacks
- WHEN `clm build step` runs
- THEN `clm run --finish a` didn't run and `build:a` is still running

### Requirement: pause
`clm build pause <name>` SHALL fetch `origin` and, when the loop file lists `<name>` as `planned` and not an orphan, mark it `paused` with the time, keeping its claim, branch, place and prompt, and print the actions to take now as `clm build step` does. For a run listed as `paused` it SHALL change nothing and print the actions to take now; for any other run, or a name the loop file doesn't list, it SHALL exit 1 naming the run's state, that it is an orphan, or that there is no such run, the loop file unchanged. No step SHALL print an action for a paused run or prepare it again, and none SHALL plan another run of its name while it is paused. `clm build start` SHALL leave a paused run paused and SHALL NOT make it an orphan; `clm build stop --now` SHALL leave it paused; a drain SHALL end without waiting for it, and its summary SHALL name it as paused; and `clm build release` SHALL NOT refuse a paused build. `clm build pause --end <name>` SHALL fetch `origin`, mark a paused run `planned` again and print the actions to take now as `clm build step` does, the run's action among them unchanged; for a run that isn't paused it SHALL exit 1 naming its state, the loop file unchanged.

#### Scenario: a printed verify paused
- GIVEN a step that printed `run verify clm verify --due --event work-done`, and no `launched` for it
- WHEN `clm build pause verify` runs, and then `clm build step`
- THEN neither prints a line naming `verify`, the loop file lists `verify` as paused, and the step's human queue lists `verify` with `say "go on with verify"`

#### Scenario: a pause ended
- GIVEN the held `verify` of "a printed verify paused"
- WHEN `clm build pause --end verify` runs
- THEN it prints `run verify clm verify --due --event work-done`, and the loop file lists `verify` as planned

#### Scenario: a launched run
- GIVEN a running `build:a` with task ID `t-1`
- WHEN `clm build pause build:a` runs
- THEN it exits 1 naming `running`, and the loop file is unchanged

#### Scenario: a pause outlives the session
- GIVEN the held `verify` of "a printed verify paused", and a lease renewed 20 minutes ago
- WHEN `clm build start --session-model opus` runs, then `clm build stop --drain` and `clm build step`
- THEN the start prints no line naming `verify` and the loop file lists `verify` as paused and not an orphan, and the step prints the summary's `say`, whose file names `verify` as paused, and `end`

#### Scenario: a paused build released
- GIVEN a planned `build:a` paused with `clm build pause build:a`, and `origin/impl/a` pushed from this clone
- WHEN `clm build release a` runs
- THEN it exits 0, `clm branch delete impl/a` and `clm run --finish a` ran, and the loop file lists no `build:a`

#### Needs
- view/queue-view

### Requirement: queue-tells-printed-from-running
The queue model `plan` and `stored_plan` give SHALL list in its running section only the orphans and the runs whose state is `running` or `stopped`. Each run whose state is `planned` and that isn't an orphan SHALL head the next section, in the loop file's order, with its kind (`run` for a `land:` or `verify` run), the priority of the plan's next entry of the same name or `-` when it has none, and the reason `printed, not launched yet`, listed once even when the plan's next section holds an entry of its name. A `paused` run SHALL be listed in neither section.

#### Scenario: a printed run isn't running
- GIVEN a loop file listing `verify` as planned, not an orphan, with the command `clm verify --due --event work-done`, `build:a` as running, not an orphan, on `opus` with task ID `t-1`, and a stored plan whose next section holds `build:b` at `P2`
- WHEN `stored_plan` reads it, and again after `clm build launched verify t-2`
- THEN the first's running section holds only `build:a` and its next section is `verify` with `-` and `printed, not launched yet`, then `build:b`; the second's running section holds `build:a` and `verify`, and its next section only `build:b`

#### Needs
- view/queue-view

### Requirement: draft-waits-for-its-build
The plan SHALL NOT launch the draft of a `group` line any of whose `<components>` is the component of a build the loop holds, a run `build:<component>` in any state but `returned` (an orphan and a paused run included), or of a build the same plan lists in its next section, whatever that entry's reason. It SHALL list that draft in the next section, in the place `plan-order` sorts it to, with the reason `waits for build:<component>`, naming the first such component in the line's order, except while stopped runs are being resumed one at a time, when it keeps `waits for stopped runs to resume` (`plan-order`); and the draft SHALL take none of the free places and none of the disk's room, so the entries after it are launched as though it weren't listed. Once no such build is held or listed, the plan SHALL launch the draft as `plan-order` launches any other. A `group` line with no components SHALL never wait so.

#### Scenario: a build and a bug draft of one component
- GIVEN `max_builds` 2, no places held, component `a` ready at `P3`, and `clm triage` printing `group bug:a [a] P1 0 x` and `group bug:b [b] P4 0 y`
- WHEN `clm build step` runs
- THEN it prints `launch build:a` and one `launch draft:`, for `y`, and the stored plan lists `draft:bug:a` with `waits for build:a`

#### Scenario: a draft waits until the build lands
- GIVEN `max_builds` 3, a running `build:a` with a claim `origin/impl/a` that `origin/main` lacks, and `clm triage` printing `group bug:a [a] P5 0 x`
- WHEN `clm build step` runs, and again after `clm build returned build:a --result 'landed <sha>'` finishes `build:a` with nothing of `a` ready
- THEN the first prints no `launch` and lists `draft:bug:a` with `waits for build:a`, and the second prints a `launch draft:` for `x`

#### Needs
- view/queue-view

### Requirement: launched-refuses-an-earlier-task
`clm build launched <name> <task-id>` SHALL refuse with exit 1, changing nothing in the loop file (its lease included), when `<task-id>` is a task ID the loop file records other than the one recorded for the run now listed as `<name>`: another run's, in any state, or one kept as an earlier run's. The loop file SHALL keep, with each run, the task IDs it reported before: whenever the loop clears a run's task ID or replaces a listed run with a new run of its name (a rebuild, `stop-causes`; an orphan's relaunch, `loop-start`; a step planning the name of a run that returned or that `claim-gone-ends-a-build` finished), the cleared or replaced run's task ID and the IDs it kept SHALL stay with the run of that name. The field SHALL be optional, so a loop file an earlier `clm` wrote loads unchanged and its `version` stays the same. Its stderr SHALL name the run that task belonged to and, for an earlier run of `<name>`, say that the loop has finished that run and that a late `landed` report of it changes nothing (`return-rules`), and that the action printed for `<name>` is a new run: for a `launch`, to be started as a new subagent, and for a `run`, as a new command, each reported with its own task ID. The build of this requirement updates the capability test `capability/build-from-spec` (`crates/clm/tests/capability/build_from_spec.rs`), whose `drain_the_queue` and `resume_in_a_new_session` report one task ID for two runs, to report a distinct ID for each run, keeping its citations (`capability-tests-follow-rules`).

#### Scenario: a landed build's task reported for its successor
- GIVEN a running `build:a` with task ID `t-1` and no `origin/impl/a`, a fake `clm run --finish a` that prints `landed impl/a`, and component `a` still ready
- WHEN `clm build step` runs, printing `launch build:a`, and then `clm build launched build:a t-1`
- THEN `launched` exits 1, its stderr names `t-1` as the task of the run of `build:a` the loop finished and says the printed `launch` is a new run, and the loop file lists `build:a` as planned with no task ID; `clm build launched build:a t-2` then records it running with `t-2`

#### Scenario: a rebuilt run's old task
- GIVEN `build:a` with task ID `t-1`, finished and rebuilt after a `context` stop, its `launch` printed again
- WHEN `clm build launched build:a t-1` runs
- THEN it exits 1 and the loop file is unchanged

#### Scenario: a relaunched orphan's old task
- GIVEN a loop whose lease lapsed, listing `build:a` as running with task ID `t-1` and a live claim `origin/impl/a`
- WHEN `clm build start --session-model opus` runs, printing `launch build:a`, and then `clm build launched build:a t-1`
- THEN it exits 1, and the loop file lists `build:a` as planned with no task ID

#### Scenario: another run's task
- GIVEN a running `build:b` with task ID `t-1`, and a planned `build:a`
- WHEN `clm build launched build:a t-1` runs
- THEN it exits 1 naming `build:b`, and the loop file is unchanged

### Requirement: returned-ignores-an-earlier-task
`clm build returned <name> --task <task-id> --result <line>`, whatever its `--cause`, SHALL record nothing of the return (no state, task ID, stop, condition or resume of any run), run no `clm run --finish` or `clm draft --finish` for it, print no `stop` or `resume` for it, and print the actions to take now as `clm build step` does, a planned run's `launch` printed again unchanged among them, when `<task-id>` is one the loop file keeps as an earlier run's of `<name>` (`launched-refuses-an-earlier-task`) and isn't the task ID recorded for the run now listed as `<name>`; its stderr SHALL say that the return is a late report of an earlier run of `<name>`, which the loop has finished, and that it changed nothing. This SHALL hold after the refusals of an invalid `--cause` or `--until`, which stay as they are, and before every rule of `return-rules` and `stop-causes`, and whatever the state of the run now listed as `<name>`. A return with no `--task`, or with a `<task-id>` the loop file doesn't keep as an earlier run's of `<name>`, SHALL be handled as `return-rules` handles it, so a session that doesn't pass `--task` gets what it got before.

#### Scenario: a late context stop of a landed build
- GIVEN a running `build:a` with task ID `t-2`, whose loop file keeps `t-1` as an earlier run's of `build:a`
- WHEN `clm build returned build:a --task t-1 --result none --cause context` runs
- THEN it exits 0, `clm run --finish a` didn't run, stderr names `build:a` as an earlier run's late report, no line printed names `build:a`, and the loop file lists `build:a` as running with `t-2` and no stop

#### Scenario: a late work stop before the new run is launched
- GIVEN a planned `build:a` with no task ID, whose loop file keeps `t-1` as an earlier run's of `build:a`, and the `launch build:a` line a step printed for it
- WHEN `clm build returned build:a --task t-1 --result none --cause work` runs
- THEN it prints that `launch build:a` line unchanged, and the loop file lists `build:a` as planned with no stop

#### Scenario: the run's own task
- GIVEN a running `build:a` with task ID `t-2`, whose loop file keeps `t-1` as an earlier run's of `build:a`
- WHEN `clm build returned build:a --task t-2 --result none --cause work` runs
- THEN it prints `resume build:a t-2 <message-file>`, and the loop file counts one `work` stop for `build:a`

### Requirement: runs-the-clm-now-at-its-path
Each time `clm-loop` starts a `clm` subprocess, it SHALL resolve the `clm` it was given (`Options.clm`, the path `std::env::current_exe` gave `clm build`) afresh: when that path ends in ` (deleted)` and names no file, as on Linux once the file the process started from has been replaced or deleted (the dev install renames a new `clm` over the running one whenever `main` moves, `dogfood-from-main`, while `clm build wait` and a step's subprocesses run for minutes), it SHALL run the file at the path without the suffix when one exists, and otherwise the running image at `/proc/self/exe`; any other path it SHALL run as given. So every subprocess a long `clm build` process starts after the replacement runs the `clm` now at the path, as `clm land` copies it (`land/land-binary-copy`), and none fails with `No such file or directory`. The resolution SHALL be a pure function of the path and of which files exist, so a test proves it without replacing a running binary.

#### Scenario: the clm it started from was replaced
- GIVEN a fixture repository, a fake `clm` at `<dir>/clm`, and `Options.clm` set to `<dir>/clm (deleted)`, a path that names no file
- WHEN `clm build step` runs through the library with those options
- THEN the fake `clm` at `<dir>/clm` receives the step's subcommands, and the step succeeds

#### Scenario: the clm it started from was deleted
- GIVEN a path `<dir>/clm (deleted)` where neither it nor `<dir>/clm` names a file, on Linux
- WHEN the loop resolves the `clm` to run
- THEN it runs `/proc/self/exe`

#### Scenario: a path that names a file
- GIVEN a path naming an existing file, whether or not it ends in ` (deleted)`
- WHEN the loop resolves the `clm` to run
- THEN it runs that path unchanged
