---
component: orchestrator.loop
base: { design: "c9eccbe5908209b3" }
---

## MODIFIED Design
Crate `clm-loop`, at `crates/clm-loop/`, dispatched by `crates/clm` as `clm build`, which passes it `max_builds` and its own executable as the `clm` it runs. It drives the other commands as subprocesses through their Contracts, reading their output as those Contracts state it: `clm next` for the ready components in priority order, `clm view flight --at origin/main --format json` for each component's priority, `clm triage` for the `group` lines and their priorities, `clm run <component> --review-model <alias>` and `clm run --finish`, `clm draft`, `clm draft --resume`, `clm draft --finish` and `clm draft --fold`, `clm wrap-up` for the housekeeping, the complete changes and what isn't safe, `clm verify --due --dry-run`, `clm upgrade` for the pending migrations, and `clm land`. So every rule of those commands stays theirs, the loop holds none of them, and its build changes no other component's files. Its own git calls (fetching, reading `origin/main`'s tree, the escalation's commit and push) go through `clm-git` (`git-plumbing-layer`).

**Why in `clm`** (`agent-harness-adapters`, `deterministic-enforcement`). The skill `build` used to hold the loop's rules as prose a model followed: return handling, retries, the order of drafts and builds, the fold, the archive and the watch. A model following prose got several of them wrong in practice, and its state lived only in its context, which compaction loses. Here each rule is code with a test, and the one judgement left to the session is the cause of a stop, which only a model can read from a subagent's output: `clm` keeps the counts and decides.

**The plan is recomputed** (`git-is-truth`). Each step computes the plan afresh from a snapshot: what the commands above printed after the step's fetch, the loop file and the time. `plan` is a pure function of it, so the scheduler is tested on snapshots built in the test with no git. The step stores the plan it acted on in the loop file with the `origin/main` commit and the time, and `clm view queue` shows that plan with the runs as they stand, so the view never shows an order the loop didn't compute. An explicit queue kept in the file was rejected: it goes stale whenever another clone lands, claims or files an item, so it would need recomputing anyway, plus reconciling.

**The loop file** is `<git common dir>/clm/loop`, beside `clm/builds/` (`build-places`), so every worktree of the clone shares it. It is written whole through a temporary file and a rename while holding its own lock, `<git common dir>/clm/loop.lock`, which is never held while a subprocess runs, so a `clm run` or `clm draft` the step starts, which takes the places' lock, can't deadlock against it. Its format is the builder's, with a `version` the loader checks. A file that can't be read or parsed is set aside as `loop.damaged` and the loop starts afresh, which loses the history, the drafted set, the conditions, the stored plan, the fold list and batches, the loop's red verify results and the escalations not yet landed. Claims, branches and places are in git and the places, and a step finds them again; review drafts a lost fold list held stay on their spec branches, which `clm proposals` lists and `/clm:triage` folds, a red verify result stays in the ledger for `clm intake`, and the next drain's stops escalate again. Losing it is a damaged file, not a step of the loop, so the cost is accepted rather than rebuilding those from git at every step.

**Steps.** A step fetches, finishes the builds whose claim has left `origin` (below), runs `clm wrap-up` for the housekeeping, reads the rest of the snapshot, computes the plan, prepares each run the plan launches (`clm run` or `clm draft`, which claim, take the place and write the prompt), records each action in the loop file, stores the plan, and prints the actions. A run's state is `planned` from the step that prints its `launch` until `launched` reports it, `running` until `returned`, then `returned`, or `stopped` when it returned with cause `stopped`; a planned run's `launch` is printed again, unchanged, by every step until it is reported, and it is never prepared twice. So a session that runs `step` twice, or loses an action line to compaction, launches each run once.

**The lease.** The holder renews it with every command it runs and every interval while `clm build wait` runs; it is live for 15 minutes after the last renewal. 15 minutes is a constant, not a setting (`yagni`): it only has to outlast the wait's interval, which is at most 7 minutes. A session that dies takes its subagents and its `clm build wait` with it, so the lease lapses and the next `start` adopts the loop. Runs the file lists as planned, running or stopped belong to a session that is gone: *orphans*. A build orphan is relaunched with `clm run`, which resumes its branch; a draft orphan with `clm draft --resume`, which continues its spec branch (`orchestrator.triage/draft-resume`); a `land:` or `verify` orphan is run again, since `clm land` of a landed branch and `clm verify --due` are safe to repeat; a fold or archive orphan is dropped and the plan computes it again, its prompt continuing a branch it finds on `origin`. A build or draft orphan whose relaunch is refused, `clm run` or `clm draft --resume` exiting 1, has no subagent and no way to get one, so keeping it would keep the loop busy for ever: the step finishes it as a run that returned with no closing line, tells the human the reason once unless the finish finds it landed, and leaves its component or items to the plan, which launches them afresh when `clm next` or `clm triage` offers them (`loop-start`). `--take-over` is the human's word that a live lease's session is gone; since that session's subagents may still be running when it isn't, an orphan whose branch the last fetch saw advance within the last 15 minutes is relaunched only once it has stayed still for 15 minutes, so two subagents never push over one branch.

**Stops.** Each run counts its stops by cause. What stopped from outside the work is resumed in the same subagent, keeping its context: a limit or a full disk each time it recurs, once it clears, and an outage or a tool failure once, at once. What went wrong in the work is given one resume, then a rebuild with a new subagent on the session's model. A rebuilt run that stops again for the cause that rebuilt it is escalated, as is a run that runs out of context twice. These are `resume-sound-builds`' criteria, now code.

**A build that is over with no return** (`claim-gone-ends-a-build`). The agent harness doesn't always deliver a subagent's final report, and a run the loop finished only on `clm build returned` would then stay `running` for ever. A build's work is over when its claim has left `origin`: `clm run` pushes the claim before the step prints the `launch`, a build pushes nothing but that branch, and only landing (`clm land` deletes the branch after the merge) or a deletion by hand takes it away. So before the housekeeping, while the local branch that tells a landed build from a deleted one is still there, every step finishes each `running` build whose `origin/impl/<component>` is gone or merged, as a run that returned with no closing line (`unclosed-build-reads-finish`). The loop reads the refs only to tell that the build is over; whether it landed stays `clm run --finish`'s to say, so there is one reading of landed. The test reads the remote-tracking refs alone and holds for as long as the run is listed, so it doesn't depend on the step that first sees the land: another session's `clm wrap-up` may have removed the local branch already, and the run is then finished without being counted as landed, which only leaves it out of the summary. Finishing early changes nothing a prompt return would have done, since the housekeeping removes a landed build's worktree at the same step anyway. The report may still arrive after the plan has launched the component again under the same name, and a report names a run only by that name. So `clm build returned` ignores a build's `landed <sha>` line whose commit is older than the run it would be recorded for: no run lands a commit made before it started (`return-rules`). The claim's state on `origin` wouldn't do for this, since another clone may claim the component between a land and its report. Not taken: an action asking the session to judge whether the run is over, which puts a rule of the loop back in a model's hands (`build-loop-in-clm`); a time limit on a run, which can't tell a long build from a lost report; reading `clm wrap-up`'s `removed impl/<component>` line, which is printed once and not at all for a worktree left dirty; and the same rule for drafts, since a draft whose spec change has landed may still be fixing an item on a code branch, so a landed change doesn't mean the draft is over.

**Releasing a build** (`release`). The human gives up a component's build when its session died and they don't want it resumed, or when it went wrong in a way no resume would mend. Deleting `origin/impl/<component>` alone isn't a release: `clm run` refuses a local `impl/<component>` that has no `origin/impl/<component>`, since it may hold commits nobody pushed (`orchestrator.run`'s `run`), so every step would skip the component until someone moved the local branch by hand, a step no skill states. So a release is one command, and the session needs no step of its own (`build-loop-in-clm`). It runs `clm branch delete impl/<component>` for the remote branch, so the rule that a clone deletes only a tip it pushed stays `host`'s, and a refusal there (another clone's claim, or a branch that moved since the fetch) leaves the clone as it was. It then renames the local `impl/<component>` to `released/<component>/<time>` with `git branch -m` through `clm-git`, which git allows while another worktree has the branch checked out and which moves that worktree's `HEAD` with it, and runs `clm run --finish <component>` to free the place, so the place's rules stay `orchestrator.run`'s. A release discards nothing the clone holds: the build's commits stay on the renamed branch and its worktree, with any uncommitted work, stays where it is, for the human to delete or take from, so the command needs no guard beyond the allow rule `clm build` already has, and no worktree rule of `orchestrator.run`'s moves into the loop. Running it again after a failure finishes the release, since `clm branch delete` of a branch already gone does nothing. Not taken: `clm branch delete` deleting the local branch too, which would give a command unattended sessions run with no classifier judging it (`unattended-steps-run-clm`) the power to discard commits nobody pushed; deleting the local branch and the worktree in the release, which would need a guard asking the human, since the same allow rule covers `clm build`, and would copy `run-finish`'s worktree rules into the loop; and `clm run` resuming a local branch with no remote, which would drop `run`'s guard against building on commits nobody pushed.

**Escalations** need no subagent. A `question` item is a file the loop writes itself: it builds a tree from `origin/main`'s with the item added through a temporary index, as `orchestrator`'s `next` reads a tree, commits it with `git commit-tree`, pushes the commit as `loop/<slug>` and lands that branch with `clm land` from the main worktree. A branch outside `impl/`, `spec/`, `propose/` and `autosync/` that only adds a backlog item lands on checks as a small change (`small-change-no-claim`, `gate/backlog-additions`), so `gate` needs no rule for it; its `/` keeps it out of the claim namespaces (`nested-branches-not-claims`).

**Waiting.** `clm build wait` reads the loop file and the places every 5 seconds, which needs no network, so a stop request, a place freed on the clone or a running build whose claim has left `origin`, which the next step finishes, wakes it at once, before the wait runs any housekeeping of its own, and fetches `origin` and reruns the snapshot's commands once per interval for work landed elsewhere. It replaces `clm next --wait`, which only the skill ran, and differs from it in waking on everything a step would launch, drafts, folds, archives and verify runs included, while runs are still going, so no place stays idle while work is ready.

**Tests** start the commands as processes on fixture repositories with a fake `clm` for the subprocesses, and test `plan` and the return rules on snapshots built in the test. A fixture gives every build a scenario calls running a claim on `origin` that `origin/main` lacks, as `clm run` leaves one, unless the scenario says the claim is gone.

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: loop-start <!-- base: 4e2a3d8c4cecb6f5 -->
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

### Requirement: return-rules <!-- base: 03218c860bfc9886 -->
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

### Requirement: build-wait <!-- base: 838b1759f8facbbb -->
`clm build wait [--interval <minutes>]` (a positive decimal number of at most 7, default 5) SHALL renew the lease every interval and exit 0, printing one line naming why, as soon as a step would print an action: a place frees while work is ready, new work is on `origin`, a fold is due, a condition clears or its time passes, a draft becomes possible, or a stop is requested; and as soon as a running build's claim has left `origin`, as `claim-gone-ends-a-build` judges it on the remote-tracking refs at each of those 5-second reads, naming the run. It SHALL make that judgement before each run of the housekeeping, its first included, so the step that finishes the build finds its local branch. It SHALL read the loop file, the places and those refs every 5 seconds and fetch `origin` and read the snapshot once per interval. When a fetch fails it SHALL print `clm build wait: fetch failed: <reason>` on stderr and go on. An `--interval` that isn't a positive number of at most 7 SHALL exit 1 before waiting.

#### Scenario: a place frees
- GIVEN `max_builds` 1, component `a` ready, and the one place held by `build:b`
- WHEN `clm build wait` runs and `build:b`'s place is freed
- THEN within 10 seconds it prints a line naming the freed place and exits 0

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
