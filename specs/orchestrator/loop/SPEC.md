---
title: Build loop
status: unbuilt
owners: ["@clunny"]
depends_on: [orchestrator, orchestrator.run, orchestrator.triage, orchestrator.checkpoint, verify, agent-harness, view, ledger, upgrade, config, vcs, model]
paths: ["crates/clm-loop/**"]
successor: ""
serves: [capability/build-from-spec, architecture/agent-harness-adapters, architecture/git-is-truth]
---

## Contract
`clm build` runs the clone's build loop (`build-loop-in-clm`): it decides every step of draining the queue continuously (what to launch, in what order, on which model, how to handle a run's return, when to fold, archive, verify and escalate) and prints the actions the session must take with the agent harness's subagents. The session carries out the actions and reports what comes back; it decides nothing. There is one loop per clone, held by a lease, and its state is in git and the loop file, so any session can resume it.

- `clm build start --session-model <alias> [--take-over]` begins or adopts the loop and prints the first actions.
- `clm build step` prints the actions to take now.
- `clm build launched <name> <task-id>` records the agent harness's task ID for a run the session launched.
- `clm build returned <name> --result <line> [--cause <cause> [--until <time>]]` records a run's return and prints the actions to take now.
- `clm build cleared <condition>` records that the human cleared a condition (`disk`) and prints the actions to take now.
- `clm build wait [--interval <minutes>]` waits until a step would act, renewing the lease, and prints why it woke.
- `clm build stop --drain|--now` records a request to drain or to stop the loop.

An action is one line on stdout: `launch <name> <kind> <model> <prompt-file>`, `resume <name> <task-id> <message-file>`, `stop <name> <task-id>`, `run <name> <command>`, `wait`, `say <file>` or `end`. A run's `<name>` is `build:<component>`, `draft:<change-id>`, `fold:<batch>`, `archive:<id>`, `land:<batch>` or `verify`; its `<kind>` is `build`, `draft`, `fold` or `archive`. A `--cause` is the session's reading of why a run that didn't land ended: `limit` (with `--until <time>` when the error says when it lifts), `disk`, `outage`, `tool`, `work`, `context`, `blocked`, `stopped` (the loop stopped it), or `running` for a run whose task the agent harness still lists as running background work. Every command fetches `origin` first, except `stop` and `launched`, and exits 0 on success, 1 when the work isn't possible (with the reason on stderr), and 2 when git fails.

As a library it offers `plan(snapshot) -> view::Queue`, a pure function, and `snapshot(root)`, which gathers the snapshot from the clone without fetching, so `view.repo` renders the same plan the loop acts on (`views-rendered-by-clm`).

## Design
Crate `clm-loop`, at `crates/clm-loop/`, dispatched by `crates/clm` as `clm build`, which passes its own executable as the program the loop runs `clm land` with, as `clm runner` is passed it (`orchestrator.runner`). It calls the libraries of the components it depends on and holds no rule they hold: `orchestrator`'s queue and places, `orchestrator.run`'s `run` and `finish`, `orchestrator.triage`'s `triage`, `draft`, `fold` and `finish`, `orchestrator.checkpoint`'s `checkpoint`, `verify`'s due check, `agent-harness`'s abilities, `ledger`'s approvals and `upgrade`'s pending migrations. Its git calls go through `clm-git` (`git-plumbing-layer`).

**Why in `clm`** (`agent-harness-adapters`, `deterministic-enforcement`). The skill `build` used to hold the loop's rules as prose a model followed: return handling, retries, the order of drafts and builds, the fold, the archive and the watch. A model following prose got several of them wrong in practice, and its state lived only in its context, which compaction loses. Here each rule is code with a test, and the one judgement left to the session is the cause of a stop, which only a model can read from a subagent's output: `clm` keeps the counts and decides.

**The plan is recomputed** (`git-is-truth`). Each step computes the plan afresh from a snapshot: `origin/main`'s queue (`queue_from_refs`) with effective priorities, `clm triage`'s groups, the complete changes, whether `clm verify` is due, the places, the loop file, the agent harness's abilities, the human's items and the time. `plan` is a pure function of it, so the scheduler is tested on snapshots built in the test with no git, and `snapshot` is the thin layer that reads them. The loop file holds only what git can't tell: the lease, the runs, the history, the drafted items, the fold list and batches, the conditions and the stop request. An explicit queue kept in the file was rejected: it goes stale whenever another clone lands, claims or files an item, so it would need recomputing anyway, plus reconciling.

**The loop file** is `<git common dir>/clm/loop`, beside `clm/builds/` (`build-places`), so every worktree of the clone shares it, written whole through a temporary file and a rename under the places' lock (`<git common dir>/clm/builds.lock`), so a step and a `clm run` or `clm draft` started at once never interleave. Its format is the builder's, with a `version` the loader checks. A file that can't be read or parsed is set aside as `loop.damaged` and the loop starts afresh, which loses only the history, the drafted set and the conditions: claims, branches and places are in git and the places, and a step finds them again.

**Steps.** A step does the housekeeping, computes the plan, prepares each run the plan launches (claiming and taking its place through `orchestrator.run` or `orchestrator.triage`, which write its prompt file), records each action in the loop file, and prints the actions, all under one fetch. A run's state is `planned` from the step that prints its `launch` until `launched` reports it, `running` until `returned`, then `returned`; a planned run's `launch` is printed again, unchanged, by every step until it is reported, and it is never prepared twice. So a session that runs `step` twice, or loses an action line to compaction, launches each run once.

**The lease.** The holder renews it with every command it runs and every interval while `clm build wait` runs; it is live for 15 minutes after the last renewal. 15 minutes is a constant, not a setting (`yagni`): it only has to outlast the wait's interval. A session that dies takes its subagents and its `clm build wait` with it, so the lease lapses and the next `start` adopts the loop. Runs the file lists as planned or running belong to the session that is gone: *orphans*, whose claims, branches and places remain, which the next step relaunches as `clm run` and `clm draft` resume a branch. `--take-over` is the human's word that a live lease's session is gone; since that session's subagents may still be running when it isn't, an orphan whose branch the last fetch saw advance within the last 15 minutes is relaunched only once it has stayed still for 15 minutes, so two subagents never push over one branch.

**Escalations** need no subagent. A `question` item is a file the loop writes itself: it makes the commit on `origin/main` with `git commit-tree`, as a claim is made, pushes it as `loop/<slug>` and lands that branch with `clm land` from the main worktree. A branch outside `impl/`, `spec/`, `propose/` and `autosync/` that only adds a backlog item lands on checks as a small change (`small-change-no-claim`, `gate/backlog-additions`), so `gate` needs no rule for it; its `/` keeps it out of the claim namespaces (`nested-branches-not-claims`).

**Waiting.** `clm build wait` reads the loop file and the places every 5 seconds, which needs no network, so a stop request or a place freed on the clone wakes it at once, and fetches `origin` once per interval for work landed elsewhere. It replaces `clm next --wait`, which only the skill ran, and differs from it in waking on everything a step would launch, drafts, folds and verify runs included, while runs are still going, so no place stays idle while work is ready.

**Tests** start the commands as processes on fixture repositories with a fake `clm` for `land`, and test `plan` and the return rules on snapshots built in the test.

## Requirements

### Requirement: loop-file
`clm-loop` SHALL keep the loop's state in `<git common dir>/clm/loop`, written whole through a temporary file and a rename while holding the lock `<git common dir>/clm/builds.lock`, holding: the lease's last renewal; each run with its name, kind, model, state (`planned`, `running`, `returned`), task ID once reported, the time it started and its round; each return's result and cause; the backlog items the loop's drafts took; the changes the loop's drafts left for review and the fold batches waiting for approval; the conditions (`disk`, and `limit` with the time it lifts); and the stop request. A file it can't read or parse SHALL be renamed `loop.damaged` and the loop SHALL start afresh, saying so on stderr.

#### Scenario: state survives the session
- GIVEN a loop that launched `build:a` and recorded its task ID, and the session ended
- WHEN a new session runs `clm build start --session-model opus` after the lease lapsed
- THEN the loop file still lists `build:a`, and the first step relaunches it as an orphan

#### Scenario: damaged file
- GIVEN `<git common dir>/clm/loop` holding bytes that don't parse
- WHEN `clm build start --session-model opus` runs
- THEN `loop.damaged` holds those bytes, stderr says the loop started afresh, and it exits 0

### Requirement: loop-start
`clm build start --session-model <alias>` SHALL take the loop's lease when there is no loop or its lease lapsed (no renewal for 15 minutes), record the session's model, clear a stop request, mark every run listed as `planned` or `running` an orphan, and print the first step's actions. When the lease is live it SHALL refuse with exit 1, printing when it was last renewed and that `clm view queue` shows the loop, unless `--take-over` is given. An orphan SHALL be relaunched by the next step as a resume of its branch, counting as no stop.

#### Scenario: a live loop
- GIVEN a loop whose lease was renewed 2 minutes ago
- WHEN `clm build start --session-model opus` runs
- THEN it exits 1, stderr names the renewal and `clm view queue`, and the loop file is unchanged

#### Scenario: adopting a lapsed loop
- GIVEN a loop whose lease was renewed 20 minutes ago, listing `build:a` as running with a live claim `origin/impl/a`
- WHEN `clm build start --session-model opus` runs
- THEN it prints `launch build:a build sonnet <prompt-file>` for a prompt that resumes `impl/a`, and the history records no stop for `a`

### Requirement: take-over-holds-moving-orphans
`clm build start --take-over` SHALL take a live lease, and an orphan it finds whose branch on `origin` advanced within the 15 minutes before the fetch SHALL NOT be relaunched until the branch has stayed at the same commit for 15 minutes; until then the plan SHALL list it as running, held by a session taken over.

#### Scenario: a branch still moving
- GIVEN a live lease, and `origin/impl/a` pushed 3 minutes ago for a run `build:a`
- WHEN `clm build start --session-model opus --take-over` runs, and then `clm build step` runs 5 minutes later
- THEN neither prints a `launch` for `build:a`, and `clm view queue` lists it under Running as held by a session taken over

### Requirement: step-actions
`clm build step` SHALL fetch `origin`, do the housekeeping (`step-housekeeping`), compute the plan, prepare each run the plan launches, record it as `planned`, and print the actions, one per line, in the forms the Contract gives, the `launch` lines in the plan's order. The prompt file a `launch` names SHALL be the loop's own, under `.clm/prompts/loop/`, holding the line `loop run <name>` first and then, unchanged, the launcher `orchestrator.run`'s `run` or `orchestrator.triage`'s `draft` or `fold` gave for it (or the archive prompt, `plan-archive`), so the panel finds which entry a subagent belongs to (`plugin.panel/build-rows`). A run still `planned` SHALL be printed again unchanged by every step until `clm build launched` reports it, and SHALL NOT be prepared again. When the loop has nothing to launch, resume or stop and no `clm build wait` of the holder is running, it SHALL print `wait`. Every `clm build` command SHALL renew the lease.

The build of this requirement updates the capability test `capability/build-from-spec` (`crates/clm/tests/capability/build_from_spec.rs`) to drain its queue through `clm build start`, `step`, `launched` and `returned` rather than `clm run`, and adds its scenario *resume in a new session* (`capability-tests-follow-rules`).

#### Scenario: an unreported launch
- GIVEN a step that printed `launch build:a build sonnet <file>`, and no `launched` for it
- WHEN `clm build step` runs again
- THEN it prints the same line, and `a` was claimed once and holds one place

#### Scenario: launched
- GIVEN a planned run `build:a`
- WHEN `clm build launched build:a t-17` runs
- THEN the loop file lists `build:a` as running with task ID `t-17`

### Requirement: return-rules
`clm build returned <name> --result <line>` SHALL, by `<line>`, the run's last line as it stands: for `waiting: <command>`, leave the run running and free nothing; for `landed <sha>` while the run's task is still running background work (`--cause running`), print `stop <name> <task-id>` and then finish it as landed; otherwise finish the run as `orchestrator.run`'s `finish` or `orchestrator.triage`'s `finish` does, and, for a run whose line is neither `landed <sha>`, `proposed <id> review <url>` nor `stopped: <reason>`, count it as landed exactly when the finish reports the branch landed (`unclosed-build-reads-finish`). It SHALL pass each `left <path>: ...` line the finish prints to a `say`. A `proposed <id> review <url>` from a draft SHALL add `<id>` to the fold list.

#### Scenario: waiting
- GIVEN a running `build:a`
- WHEN `clm build returned build:a --result 'waiting: clm land impl/a'` runs
- THEN `build:a` is still running and its place still counts

#### Scenario: landed with a task left running
- GIVEN a running `build:a` with task ID `t-17`
- WHEN `clm build returned build:a --result 'landed 1a2b3c4' --cause running` runs
- THEN it prints `stop build:a t-17` first, and the place for `a` is freed

#### Scenario: no closing line, landed
- GIVEN a running `build:a` whose `impl/a` is an ancestor of `origin/main`
- WHEN `clm build returned build:a --result none --cause context` runs
- THEN the run is returned as landed and no stop is recorded

### Requirement: stop-causes
For a run that returned without landing, `clm build returned` SHALL decide by `--cause`: `limit` and `disk` SHALL resume the same subagent each time they recur, once the condition clears (`--until <time>` passes for `limit`; `clm build cleared disk` for `disk`), recording the condition meanwhile; `outage` and `tool` SHALL resume at once, the first time for that run; `work` SHALL resume at once the first time; `context`, a second `work`, and a second `outage` or `tool` SHALL rebuild it with a new subagent on the session's model, after finishing it; `blocked` SHALL relaunch nothing. A resume SHALL be a `resume <name> <task-id> <message-file>` action whose message says why the run stopped, that the cause has cleared, to carry on in its own worktree, and to run again any review or check subagent of its own the stop killed. A resume reported as failed (`clm build returned <name> --result none --cause tool` for a run the loop resumed and that never ran) SHALL rebuild it.

#### Scenario: a limit
- GIVEN a running `build:a`
- WHEN `clm build returned build:a --result none --cause limit --until 2026-11-13T14:00:00Z` runs at 13:00, and `clm build step` runs at 14:01
- THEN the first prints no action for `build:a` and keeps its place, and the second prints `resume build:a <task-id> <message-file>`

#### Scenario: a second work stop
- GIVEN `build:a` resumed once after a `work` stop
- WHEN `clm build returned build:a --result 'stopped: check fails' --cause work` runs
- THEN it finishes `build:a` and prints `launch build:a build opus <prompt-file>` with round 2

### Requirement: repeated-stops-escalate
When a component or draft stops twice in a row for the same cause, other than `limit`, `disk` and `blocked`, the loop SHALL launch it no more and SHALL file a `kind: question` backlog item on it, whose `components` name the component (or the draft's items' components), whose `blocks` name the component's pending requirements (none for a draft), and whose body names the two stops with their results. It SHALL file it by committing the item onto `origin/main` with `git commit-tree`, pushing that commit as `loop/<slug>`, and landing the branch with `clm land loop/<slug>` run from the main worktree, then deleting the branch. A land that fails SHALL keep the escalation in the loop file, listed in the human's queue, and the next step SHALL try again.

#### Scenario: escalated
- GIVEN `build:a` stopped with cause `work`, rebuilt, and stopped with cause `work` again, `a` pending `a/x`
- WHEN `clm build returned` records the second stop
- THEN a `kind: question` item on `a` whose `blocks` names `a/x` is on `origin/main`, no `loop/*` branch is left on `origin`, and the next step launches nothing for `a`

### Requirement: plan-order
The plan SHALL launch, up to the clone's free places, in one list sorted by priority (`P1` first): each component `clm next` gives, at its effective priority (`priority-orders-components`), and each `group` line of `clm triage` none of whose items a draft of the loop took, at its priority; at equal priority a draft SHALL come before a build, and otherwise each keeps its own command's order. A build SHALL launch on the build model `clm agent-harness` names and a draft on the session's model (`model-per-launch`). An item a draft of the loop took SHALL never be drafted again by the loop. It SHALL launch nothing new while a drain or a stop is requested.

#### Scenario: a P1 build before P4 drafts
- GIVEN `max_builds` 3, three `group` lines at `P4`, and component `a` ready at `P1`
- WHEN `clm build step` runs
- THEN its first `launch` is `build:a`, followed by two drafts

#### Scenario: a draft before a build at equal priority
- GIVEN `max_builds` 2, component `a` ready at `P3` and a `group` line at `P3`
- WHEN `clm build step` runs
- THEN it launches the draft first and then `build:a`

### Requirement: plan-fold
Once the fold list isn't empty and no draft of the loop is running, the plan SHALL launch one fold, `fold:<batch>`, with the prompt `orchestrator.triage`'s `fold` writes for the fold list's IDs in the order their `group` lines came, on the build model, taking no place. Its `proposed <batch> review <url>` SHALL put the batch in the human's queue. When the ledger holds the human's approval of the batch's current diff, as `clm land` judges it (`land.approval`), the plan SHALL print `run land:<batch> clm land spec/<batch>`, and, once it has landed, finish each folded draft.

#### Scenario: fold while builds run
- GIVEN drafts `x` and `y` returned `proposed x review <url>` and `proposed y review <url>`, no draft running, and `build:a` running
- WHEN `clm build step` runs
- THEN it prints a `launch fold:<batch> fold sonnet <prompt-file>` for `x` then `y`

#### Scenario: approved batch
- GIVEN a fold batch `b` in the human's queue, and an `approved` record for `spec/b`'s current fingerprint
- WHEN `clm build step` runs
- THEN it prints `run land:b clm land spec/b`

### Requirement: plan-archive
When `origin/main` holds complete changes that aren't archived and no `impl/archive/*` branch is live on `origin`, the plan SHALL launch `archive:<id>`, `<id>` the first such change, on the build model, taking no place, with a prompt that cuts `impl/archive/<id>` from `origin/main` in its own worktree, runs `clm rebase` until its last line is `rebase ok`, pushes with `clm branch push`, leaves the archive to the other session when the push is refused because the branch exists (`archive-branches`), opens its pull request and lands it with `clm land`.

#### Scenario: complete change
- GIVEN change `c` complete on `origin/main` and no `impl/archive/*` on `origin`
- WHEN `clm build step` runs
- THEN it prints `launch archive:c archive sonnet <prompt-file>`, whose prompt names `impl/archive/c`

### Requirement: plan-verify
After a run of the loop landed, and when the loop becomes idle (no run running and nothing to launch), the plan SHALL ask `verify` whether a run is due, with the `work-done` event when idle, and, when one is, print `run verify clm verify --due` (with `--event work-done` when idle). The run's last line SHALL be reported with `clm build returned verify --result <line>`. A `verify failed` line SHALL put the red result in the human's queue and SHALL NOT draft it, since the human takes a slow test's break in (`slow-tests-after-land`).

#### Scenario: idle and due
- GIVEN `[verify] triggers = ["queue-drained"]`, one land since the last `verified` record, and nothing running or ready
- WHEN `clm build step` runs
- THEN it prints `run verify clm verify --due --event work-done`

### Requirement: step-housekeeping
Every step SHALL do the removals `clm wrap-up` does (`orchestrator.checkpoint/checkpoint`): remove landed builds' worktrees under `.claude/worktrees/` and landed branches, and free the places that don't count, passing each `left <path>: ...` line to a `say` once. What `clm wrap-up` reports and doesn't change (unsafe work, ledger findings) SHALL go in the human's queue.

#### Scenario: landed leftover
- GIVEN a landed `impl/a` whose worktree `.claude/worktrees/agent-a` is clean
- WHEN `clm build step` runs
- THEN the worktree and the local and remote `impl/a` are gone

### Requirement: loop-follows-agent-harness
The loop SHALL read the agent harness's abilities (`agent-harness/abilities`) and follow them (`skills-follow-agent-harness`): where `subagents` is `no`, it SHALL launch one run at a time, its `launch` lines saying to run the prompt in the session itself; where `isolated-worktrees` is `no`, it SHALL add the run's worktree at `.claude/worktrees/agent-<name>` from `origin/main`, `<name>` with `:` as `-`, before printing the `launch`, and its prompt SHALL say to work there; where `model-choice` is `no`, every run SHALL be on the session's model.

#### Scenario: no subagents
- GIVEN an agent harness whose `subagents` is `no`, and components `a` and `b` ready with `max_builds` 3
- WHEN `clm build step` runs
- THEN it prints one `launch`, for `build:a`, and none for `b` until `build:a` returns

### Requirement: build-wait
`clm build wait [--interval <minutes>]` (a positive decimal number, default 5) SHALL renew the lease every interval and exit 0, printing one line naming why, as soon as a step would print an action: a place frees while work is ready, new work is on `origin`, a fold is due, a condition clears or its time passes, a draft becomes possible, or a stop is requested. It SHALL read the loop file and the places every 5 seconds and fetch `origin` once per interval. When a fetch fails it SHALL print `clm build wait: fetch failed: <reason>` on stderr and go on. An `--interval` that isn't a positive number SHALL exit 1 before waiting.

#### Scenario: a place frees
- GIVEN `max_builds` 1, component `a` ready, and the one place held by `build:b`
- WHEN `clm build wait` runs and `build:b`'s place is freed
- THEN within 10 seconds it prints a line naming the freed place and exits 0

#### Scenario: a stop request
- GIVEN `clm build wait --interval 5` running
- WHEN `clm build stop --drain` runs
- THEN within 10 seconds it prints a line naming the drain and exits 0

### Requirement: build-stop
`clm build stop --drain` SHALL record a drain: no step launches anything new, running runs finish and are handled as usual, the fold list is folded, and once nothing runs the step prints the summary's `say` and `end`. `clm build stop --now` SHALL record a stop: the next step prints `stop <name> <task-id>` for every running run, each then returned with cause `stopped`, which keeps its claim, branch and place for the next `start` to relaunch, and then the summary's `say` and `end`. Neither SHALL fetch.

#### Scenario: stop now
- GIVEN running `build:a` (task `t-1`) and `draft:c` (task `t-2`)
- WHEN `clm build stop --now` runs and then `clm build step`
- THEN the step prints `stop build:a t-1` and `stop draft:c t-2`, and after both are returned with `--cause stopped` the next step prints a `say` and `end`, and `origin/impl/a` still exists

### Requirement: end-summary
The summary a drain or a stop ends with SHALL say what landed since `start`, what is still running or held and on what, the human's queue as it stands, each `workaround` item and each `reconsider-<component>-<slug>` question the loop's runs filed, pointing at `/clm:health`, and whether the session can end safely, the unsafe work `clm wrap-up` would report standing in for that line when there is any.

#### Scenario: summary with a workaround
- GIVEN a drain in which `build:a` landed and filed `backlog/workaround-a-x-1.md`
- WHEN the last run returns
- THEN the summary names `a` as landed, names `workaround-a-x-1` and `/clm:health`, and says the session can end

### Requirement: humans-queue
The plan SHALL hold the human's queue, each entry with what the human does about it, oldest first: the fold batches waiting for approval (`say "approve <batch>"`); `question` and `human-task` items that block requirements, with what they block (`answer backlog/<slug>`); the escalations the loop couldn't land; red `verified` results no item has taken in (`take in with /clm:triage`); unsafe work and ledger findings as `orchestrator.checkpoint`'s report gives them; pending upgrade migrations (`run /clm:upgrade`); and the conditions only the human can clear (`free disk, then say so`). It SHALL NOT call the host, so intake isn't in it.

#### Scenario: escalated build in the queue
- GIVEN an item `backlog/a-stops-on-work.md` the loop filed, blocking `a/x`
- WHEN the plan is computed
- THEN the human's queue lists `a-stops-on-work` with `answer backlog/a-stops-on-work` and `a/x`

### Requirement: plan-snapshot
`clm-loop` SHALL offer `plan(snapshot)`, a pure function returning `view`'s queue model with the sections Running, Next up, Waiting on you and Waiting on work in the order and with the reasons the steps act on, and `snapshot(root)`, which reads the snapshot from the local refs, the places and the loop file without fetching; `clm build step` SHALL act on `plan` of the snapshot it reads after its fetch, so the queue view and the loop never disagree on the same refs.

#### Scenario: view and step agree
- GIVEN a fetched clone with `a` ready at `P2` and a `group` line at `P4`, `max_builds` 1
- WHEN `plan(snapshot(root))` is computed and `clm build step` runs on the same refs
- THEN the plan's Next up starts with `build:a` and the step's first `launch` is `build:a`
