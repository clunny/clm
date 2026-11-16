---
component: orchestrator.loop
base: { frontmatter: "88c0ba1eaff2ca7b", design: "3e7da94b0d4e3d9b" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [orchestrator, orchestrator.run, orchestrator.triage, orchestrator.checkpoint, verify, upgrade, land, view, agent-harness, config, vcs, host, model, machine]
```

## MODIFIED Design
Crate `clm-loop`, at `crates/clm-loop/`, dispatched by `crates/clm` as `clm build`, which passes it `max_builds` and its own executable as the `clm` it runs. It drives the other commands as subprocesses through their Contracts, reading their output as those Contracts state it: `clm next` for the ready components in priority order, `clm view flight --at origin/main --format json` for each component's priority, `clm triage` for the `group` lines and their priorities, `clm run <component> --review-model <alias>` and `clm run --finish`, `clm draft`, `clm draft --resume`, `clm draft --finish` and `clm draft --fold`, `clm wrap-up` for the housekeeping, the complete changes and what isn't safe, `clm verify --due --dry-run`, `clm upgrade` for the pending migrations, `clm machine --format json` for the disk's `room` and `state`, and `clm land`. So every rule of those commands stays theirs, the loop holds none of them, and its build changes no other component's files. Its own git calls (fetching, reading `origin/main`'s tree, the escalation's commit and push) go through `clm-git` (`git-plumbing-layer`).

**Why in `clm`** (`agent-harness-adapters`, `deterministic-enforcement`). The skill `build` used to hold the loop's rules as prose a model followed: return handling, retries, the order of drafts and builds, the fold, the archive and the watch. A model following prose got several of them wrong in practice, and its state lived only in its context, which compaction loses. Here each rule is code with a test, and the one judgement left to the session is the cause of a stop, which only a model can read from a subagent's output: `clm` keeps the counts and decides.

**The plan is recomputed** (`git-is-truth`). Each step computes the plan afresh from a snapshot: what the commands above printed after the step's fetch, the loop file and the time. `plan` is a pure function of it, so the scheduler is tested on snapshots built in the test with no git. The step stores the plan it acted on in the loop file with the `origin/main` commit and the time, and `clm view queue` shows that plan with the runs as they stand, so the view never shows an order the loop didn't compute. An explicit queue kept in the file was rejected: it goes stale whenever another clone lands, claims or files an item, so it would need recomputing anyway, plus reconciling.

**The loop file** is `<git common dir>/clm/loop`, beside `clm/builds/` (`build-places`), so every worktree of the clone shares it. It is written whole through a temporary file and a rename while holding its own lock, `<git common dir>/clm/loop.lock`, which is never held while a subprocess runs, so a `clm run` or `clm draft` the step starts, which takes the places' lock, can't deadlock against it. Its format is the builder's, with a `version` the loader checks. A file that can't be read or parsed is set aside as `loop.damaged` and the loop starts afresh, which loses the history, the drafted set, the conditions, the stored plan, the fold list and batches, the loop's red verify results and the escalations not yet landed. Claims, branches and places are in git and the places, and a step finds them again; review drafts a lost fold list held stay on their spec branches, which `clm proposals` lists and `/clm:triage` folds, a red verify result stays in the ledger for `clm intake`, and the next drain's stops escalate again. Losing it is a damaged file, not a step of the loop, so the cost is accepted rather than rebuilding those from git at every step.

**Steps.** A step fetches, finishes the builds whose claim has left `origin` (below), runs `clm wrap-up` for the housekeeping, reads the rest of the snapshot, computes the plan, prepares each run the plan launches (`clm run` or `clm draft`, which claim, take the place and write the prompt), records each action in the loop file, stores the plan, and prints the actions. A run's state is `planned` from the step that prints its `launch` until `launched` reports it, `running` until `returned`, then `returned`, or `stopped` when it returned with cause `stopped`; a planned run's `launch` is printed again, unchanged, by every step until it is reported, and it is never prepared twice. So a session that runs `step` twice, or loses an action line to compaction, launches each run once.

**The lease.** The holder renews it with every command it runs and every interval while `clm build wait` runs; it is live for 15 minutes after the last renewal. 15 minutes is a constant, not a setting (`yagni`): it only has to outlast the wait's interval, which is at most 7 minutes. A session that dies takes its subagents and its `clm build wait` with it, so the lease lapses and the next `start` adopts the loop. Runs the file lists as planned, running or stopped belong to a session that is gone: *orphans*. A build orphan is relaunched with `clm run`, which resumes its branch; a draft orphan with `clm draft --resume`, which continues its spec branch (`orchestrator.triage/draft-resume`); a `land:` or `verify` orphan is run again, since `clm land` of a landed branch and `clm verify --due` are safe to repeat; a fold or archive orphan is dropped and the plan computes it again, its prompt continuing a branch it finds on `origin`. A build or draft orphan whose relaunch is refused, `clm run` or `clm draft --resume` exiting 1, has no subagent and no way to get one, so keeping it would keep the loop busy for ever: the step finishes it as a run that returned with no closing line, tells the human the reason once unless the finish finds it landed, and leaves its component or items to the plan, which launches them afresh when `clm next` or `clm triage` offers them (`loop-start`). `--take-over` is the human's word that a live lease's session is gone; since that session's subagents may still be running when it isn't, an orphan whose branch the last fetch saw advance within the last 15 minutes is relaunched only once it has stayed still for 15 minutes, so two subagents never push over one branch.

**Stops.** Each run counts its stops by cause. What stopped from outside the work is resumed in the same subagent, keeping its context: a limit or a full disk each time it recurs, once it clears, and an outage or a tool failure once, at once. What went wrong in the work is given one resume, then a rebuild with a new subagent on the session's model. A rebuilt run that stops again for the cause that rebuilt it is escalated, as is a run that runs out of context twice. These are `resume-sound-builds`' criteria, now code.

**The disk** (`loop-holds-launches-on-disk`). Builds that run beside each other fill the disk with their worktrees' build output, and a build that runs out of space fails its land check for a reason that has nothing to do with its code; once the disk is full, no build lands. `max_builds` bounds the disk only as a count, which can't know how much a build of this project takes, while `machine` measures it: its `room` is how many more builds free space covers once the running ones have grown to the largest build seen. So each step reads `clm machine --format json` with the rest of the snapshot and launches no more new builds and drafts than that `room`, and none at all while a `disk` condition holds, since a run already stopped on a full disk says the measure fell short. Before any build has been measured there is no `room`, and the loop holds back only while the disk reads `critical`, the state `machine` judges on the filesystem's size alone. A reading that fails, or a disk the system won't read (`unknown`), holds back nothing: the loop then starts builds by places, as it did before it read the disk. An orphan's relaunch isn't held back, since its place counts already and its worktree's growth is in what `room` sets aside. What a land check writes beyond the worktree (the shared land build directory, a browser suite's artifacts) isn't in the measure; the land directory is one, whatever the number of builds, and already takes its space, and the rest is small beside a build's output, so the `room` is not padded for it (`yagni`). The disk frees mostly when a build lands, `clm land` deleting its worktree's build output (`build-places`), so a step that holds work back on the disk while a run of the loop is going waits for it as for a place; only when none of the loop's runs is going, so nothing will free space, does the human's queue ask for disk. Not taken: `clm run` and `clm draft` refusing on the disk themselves, which would hold a rule of the loop's in two commands, and a free-space setting the human sets, which can't be right for every toolchain (`machine-load-measured-by-clm`).

**A build that is over with no return** (`claim-gone-ends-a-build`). The agent harness doesn't always deliver a subagent's final report, and a run the loop finished only on `clm build returned` would then stay `running` for ever. A build's work is over when its claim has left `origin`: `clm run` pushes the claim before the step prints the `launch`, a build pushes nothing but that branch, and only landing (`clm land` deletes the branch after the merge) or a deletion by hand takes it away. So before the housekeeping, while the local branch that tells a landed build from a deleted one is still there, every step finishes each `running` build whose `origin/impl/<component>` is gone or merged, as a run that returned with no closing line (`unclosed-build-reads-finish`). The loop reads the refs only to tell that the build is over; whether it landed stays `clm run --finish`'s to say, so there is one reading of landed. The test reads the remote-tracking refs alone and holds for as long as the run is listed, so it doesn't depend on the step that first sees the land: another session's `clm wrap-up` may have removed the local branch already, and the run is then finished without being counted as landed, which only leaves it out of the summary. Finishing early changes nothing a prompt return would have done, since the housekeeping removes a landed build's worktree at the same step anyway. The report may still arrive after the plan has launched the component again under the same name, and a report names a run only by that name. So `clm build returned` ignores a build's `landed <sha>` line whose commit is older than the run it would be recorded for: no run lands a commit made before it started (`return-rules`). The claim's state on `origin` wouldn't do for this, since another clone may claim the component between a land and its report. Not taken: an action asking the session to judge whether the run is over, which puts a rule of the loop back in a model's hands (`build-loop-in-clm`); a time limit on a run, which can't tell a long build from a lost report; reading `clm wrap-up`'s `removed impl/<component>` line, which is printed once and not at all for a worktree left dirty; and the same rule for drafts, since a draft whose spec change has landed may still be fixing an item on a code branch, so a landed change doesn't mean the draft is over.

**Releasing a build** (`release`). The human gives up a component's build when its session died and they don't want it resumed, or when it went wrong in a way no resume would mend. Deleting `origin/impl/<component>` alone isn't a release: `clm run` refuses a local `impl/<component>` that has no `origin/impl/<component>`, since it may hold commits nobody pushed (`orchestrator.run`'s `run`), so every step would skip the component until someone moved the local branch by hand, a step no skill states. So a release is one command, and the session needs no step of its own (`build-loop-in-clm`). It runs `clm branch delete impl/<component>` for the remote branch, so the rule that a clone deletes only a tip it pushed stays `host`'s, and a refusal there (another clone's claim, or a branch that moved since the fetch) leaves the clone as it was. It then renames the local `impl/<component>` to `released/<component>/<time>` with `git branch -m` through `clm-git`, which git allows while another worktree has the branch checked out and which moves that worktree's `HEAD` with it, and runs `clm run --finish <component>` to free the place, so the place's rules stay `orchestrator.run`'s. A release discards nothing the clone holds: the build's commits stay on the renamed branch and its worktree, with any uncommitted work, stays where it is, for the human to delete or take from, so the command needs no guard beyond the allow rule `clm build` already has, and no worktree rule of `orchestrator.run`'s moves into the loop. Running it again after a failure finishes the release, since `clm branch delete` of a branch already gone does nothing. Not taken: `clm branch delete` deleting the local branch too, which would give a command unattended sessions run with no classifier judging it (`unattended-steps-run-clm`) the power to discard commits nobody pushed; deleting the local branch and the worktree in the release, which would need a guard asking the human, since the same allow rule covers `clm build`, and would copy `run-finish`'s worktree rules into the loop; and `clm run` resuming a local branch with no remote, which would drop `run`'s guard against building on commits nobody pushed.

**Escalations** need no subagent. A `question` item is a file the loop writes itself: it builds a tree from `origin/main`'s with the item added through a temporary index, as `orchestrator`'s `next` reads a tree, commits it with `git commit-tree`, pushes the commit as `loop/<slug>` and lands that branch with `clm land` from the main worktree. A branch outside `impl/`, `spec/`, `propose/` and `autosync/` that only adds a backlog item lands on checks as a small change (`small-change-no-claim`, `gate/backlog-additions`), so `gate` needs no rule for it; its `/` keeps it out of the claim namespaces (`nested-branches-not-claims`).

**Waiting.** `clm build wait` reads the loop file and the places every 5 seconds, which needs no network, so a stop request, a place freed on the clone or a running build whose claim has left `origin`, which the next step finishes, wakes it at once, before the wait runs any housekeeping of its own, and fetches `origin` and reruns the snapshot's commands once per interval for work landed elsewhere. It replaces `clm next --wait`, which only the skill ran, and differs from it in waking on everything a step would launch, drafts, folds, archives and verify runs included, while runs are still going, so no place stays idle while work is ready.

**Tests** start the commands as processes on fixture repositories with a fake `clm` for the subprocesses, and test `plan` and the return rules on snapshots built in the test. A fixture gives every build a scenario calls running a claim on `origin` that `origin/main` lacks, as `clm run` leaves one, unless the scenario says the claim is gone.

## MODIFIED Requirements

### Requirement: plan-order <!-- base: 2f9f0cc744217929 -->
The plan SHALL launch, up to the clone's free places (`max_builds` less the places that count, as `orchestrator`'s `places` reads them), in one list sorted by priority (`P1` first): each component `clm next` prints, at the priority `clm view flight --at origin/main --format json` gives it, and each `group` line `clm triage` prints none of whose items a draft of the loop left, at the line's priority; at equal priority a draft SHALL come before a build, and otherwise each keeps its own command's order. A build SHALL launch on the build model `clm agent-harness` names, with `--review-model` the session's model, and a draft on the session's model (`model-per-launch`). It SHALL launch no new build or draft while a drain or a stop is requested. It SHALL also hold new builds and drafts back on the disk, as `clm machine --format json` read in the step's snapshot gives it: no more of them in a step than its `disk.room` when that is a number; none while `disk.room` is `null` and `disk.state` is `critical`; and none while a `disk` condition holds. A `clm machine` that exits non-zero or prints output that doesn't parse, or a `disk.state` of `unknown`, SHALL hold nothing back. An orphan's relaunch SHALL NOT be held back on the disk. An entry the disk holds back SHALL be listed in the plan's next section with the reason `waits for disk`. When `clm run` or `clm draft` exits 3 because another session took the place meanwhile, the step SHALL launch nothing more that step and print no action for that run.

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

### Requirement: build-wait <!-- base: e30160925a731d1d -->
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

### Requirement: humans-queue <!-- base: b7a7e8fadd37b194 -->
The plan SHALL hold the human's queue, each entry with what the human does about it and since when, oldest first: the fold batches waiting for approval (`say "approve <batch>"`); the `question` and `human-task` items on `origin/main` that block requirements, with what they block (`answer backlog/<slug>`), since the commit that added them; the escalations the loop couldn't land; the red results of the loop's verify runs no item has taken in (`take in with /clm:triage`); the work `clm wrap-up` reports isn't safe and its ledger findings; the migrations `clm upgrade` lists (`run /clm:upgrade`); the conditions only the human can clear (`free disk, then say so`); and, when the disk holds back ready work (`plan-order`) while no run of the loop is planned or running, so no land will free space, the disk (`free disk; the loop goes on once a build fits`). It SHALL NOT call the host, so intake isn't in it.

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
