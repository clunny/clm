---
title: Build skill
status: active
owners: ["@clunny"]
depends_on: []
paths: ["plugin/skills/build/**"]
successor: ""
serves: [architecture/method-in-the-plugin, capability/work-from-any-agent-harness]
---

## Contract
The skill `build`, `plugin/skills/build/SKILL.md`: run the clone's build loop (`orchestrator.loop`) by carrying out the actions `clm build` prints, launching each run as a background subagent in its own worktree on the model its action names, reporting each return, and asking the human only what the loop puts in the human's queue that needs this conversation, the approval of a fold batch. The loop decides every launch, order, retry, fold, archive and verify; the skill decides none. The rules it shares with other skills are `plugin`'s; `plugin`'s Contract says what each asks of this file.

## Design
`build` is the agent harness's half of the build loop (`build-loop-in-clm`, `agent-harness-adapters`): `clm build` holds every rule, and the skill only does what `clm` can't, launching subagents, resuming and stopping them, reading why one stopped, and talking to the human. The skill used to hold the loop's rules as prose (the order of drafts and builds, return handling, resume against rebuild, the fold, the archive, the wrap-up and the watch), and a model following them got several wrong and lost their state to compaction; those requirements moved into `orchestrator.loop` as code.

Each action line maps to one tool call: `launch` to the Agent tool as a background `general-purpose` subagent with `isolation: "worktree"` and the line's model, its prompt the file's text unchanged, then `clm build launched` with the task ID; `resume` to `SendMessage` with the file's text; `stop` to `TaskStop`; `run` to a background Bash command whose last line is reported with `clm build returned`; `wait` to `clm build wait` as a background Bash command; `say` to showing the file's text once; `end` to ending the loop. Deferred tools are loaded with `ToolSearch` first. A subagent's return is reported as it stands, its last line unchanged, with the cause the session reads from its output: reading a stop's cause is the one judgement only a model can make, and `clm` keeps the counts.

Nothing the human must see is kept only in the chat (`build-keeps-the-human-in-sight`): the loop puts it in the human's queue, which `clm view queue` and the panel's Queue tab show until it is resolved. The approval of a fold batch is still asked in the conversation, since `clm approve` must be the human's yes through the guard (`approval-policy`, `humans-review-intent`).

**Tests** (`tests-count-where-cited`). The tests of this component's requirements sit in `crates/clm/tests/plugin_build.rs`; `plugin`'s tests of the shared rules read this component's file too.

## Requirements

### Requirement: skill-named-build
The plugin SHALL ship the skill that builds the approved specs as `skills/build/SKILL.md` and no `skills/implement/` directory, its heading and its description saying build. The description SHALL contain `approved specs` and `not compile`, so that nobody takes `/clm:build` for a compiler. No file under `plugin/` or `docs/guide/`, nor `CLAUDE.md`, nor `AGENTS.md` when it exists, nor `README.md`, SHALL name `/clm:implement` or `skills/implement/`, so the rule still covers this repository's instructions once `upgrade/migration-agents-md` has moved them from `CLAUDE.md` to `AGENTS.md`.

#### Scenario: the skill and its description
- GIVEN the plugin
- WHEN the tests run
- THEN `skills/build/SKILL.md` exists with a `description` holding `approved specs` and `not compile`, and `skills/implement/` doesn't exist

#### Scenario: an old mention left behind
- GIVEN a guide page or a skill that still says `/clm:implement`
- WHEN the tests run
- THEN the test fails, naming the file and the line

#### Scenario: the repository's instructions are read
- GIVEN the repository
- WHEN the tests run
- THEN the files they check for `/clm:implement` include `CLAUDE.md`, `README.md`, and `AGENTS.md` when it exists

### Requirement: build-runs-the-loop
`skills/build/SKILL.md` SHALL tell Claude to say once, before starting, that runs launch under the session's permission mode and pause for the human's answers in manual mode, never refusing a mode; to run `clm build start --session-model <alias>`, `<alias>` the alias of the session's own model; and then to carry out each action every `clm build` command prints, in order: `launch` as a background subagent with `isolation: "worktree"` and the line's model, its prompt the named file's text unchanged, followed by `clm build launched <name> <task-id>`; `resume` with `SendMessage`, reporting `clm build returned <name> --result none --cause tool` when it can't be delivered; `stop` with `TaskStop`, or, where the agent harness can't stop a subagent, by waiting for it to return and reporting it with `--cause stopped`; `run` as a background Bash command; `wait` as `clm build wait` in the background; `say` by showing the file's text to the human; and `end` by ending. It SHALL tell Claude to report every subagent's or background command's return with `clm build returned <name> --result <last line>` as it stands, and, when `clm build wait` exits, to run `clm build step`. It SHALL tell Claude never to decide a launch, an order, a retry or a resume itself, never to do a run's work in its own context unless the `launch` line says to run it in the session, and never to run a command aimed at a run's worktree. When `clm build start` refuses because another session holds the loop, it SHALL tell Claude to show the human `clm view queue` and to run `clm build start --take-over` only when the human says that session is gone. When the human asks to drain or stop the loop, it SHALL tell Claude to run `clm build stop --drain` or `clm build stop --now`, and when the human says they freed the disk, `clm build cleared disk`. When the human gives up a component for good, it SHALL tell Claude to release its claim with `clm branch delete`. It SHALL tell Claude to report each CLM defect it meets with `clm defect` when it happens, naming with `--component` the component a fix would lie in, since the session opens no pull request of its own and the loop drafts reported defects with the backlog's other bugs (`plugin/defect-check-before-pr`); it SHALL have no step holding `fix reported defects`. It SHALL NOT name `clm next --wait`, `clm run`, `clm draft` or `clm wrap-up`.

#### Scenario: the shipped skill
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm build start --session-model`, `clm build launched`, `clm build cleared disk`, `clm defect`, `clm build returned`, `clm build wait`, `clm build step`, `clm build stop --drain`, `clm build stop --now`, `--take-over` and `isolation: "worktree"`, and none of `clm next --wait`, `clm run `, `clm draft ` or `clm wrap-up`

#### Scenario: a skill that schedules
- GIVEN a `skills/build/SKILL.md` that tells Claude to run `clm next` and launch what it prints
- WHEN the test's check runs on it
- THEN it reports `clm next`

#### Serves
- capability/build-from-spec
- architecture/agent-harness-adapters

#### Needs
- orchestrator.loop/stop-causes
- orchestrator.loop/step-actions
- orchestrator.loop/loop-start
- orchestrator.loop/build-wait
- orchestrator.loop/build-stop

### Requirement: build-reads-the-cause
`skills/build/SKILL.md` SHALL tell Claude, for a return that isn't `landed <sha>`, `proposed <id> review <url>` or `waiting: <command>`, to pass with `--cause` its reading of why the run ended: `limit` for a usage or rate limit error (with `--until <time>` when the error says when it lifts), `disk` for a full disk, `outage` for a network outage, `tool` for a tool or host failure, `work` for a check or test that fails or an approach that went wrong, `context` for a run that ran out of context, `blocked` for a run that stopped on a backlog item or a spec change, `stopped` for a run the loop told it to stop; and `--cause running` with a `landed <sha>` line when the agent harness still lists the run as running background work. It SHALL say that the cause is the session's one judgement and that `clm build` decides what follows from it.

#### Scenario: the shipped skill
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it names each of the causes `limit`, `disk`, `outage`, `tool`, `work`, `context`, `blocked`, `stopped` and `running`, and `--until`

#### Serves
- capability/build-from-spec

#### Needs
- orchestrator.loop/stop-causes

### Requirement: build-approves-in-session
`skills/build/SKILL.md` SHALL tell Claude, when the loop's human queue (`clm view queue`) holds a fold batch waiting for approval, to ask the human once in the conversation whether they approve the intent of every folded change, showing the pull request's link and each change's `## Intent`, `## Decisions` and `## Open questions`; on a yes, to run `clm approve spec/<batch>`, which the guard has them confirm, and then `clm build approved <batch>`; never to approve or land a change otherwise (`approval-policy`, `humans-review-intent`). It SHALL keep `plugin`'s shared rules: never push `main` by any means but `clm land`, and run `clm approve` only after the human says yes in this conversation, through the guard's confirmation.

#### Scenario: the shipped skill
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm approve spec/<batch>`, `clm build approved`, `## Intent`, `## Decisions` and `## Open questions`

#### Serves
- capability/approve-intent
- architecture/approval-policy

#### Needs
- orchestrator.loop/plan-fold

### Requirement: build-keeps-the-human-in-sight
`skills/build/SKILL.md` SHALL tell Claude that what needs the human is in the loop's queue, which `clm view queue` and the CLM panel's Queue tab show until it is resolved, and so to point the human at it rather than keep a question, a stop or a condition only in the chat; when the human asks what is waiting on them, to answer from `clm view queue`.

#### Scenario: the shipped skill
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm view queue` and `Queue`

#### Serves
- capability/build-from-spec
- capability/watch-beside-chat

#### Needs
- orchestrator.loop/humans-queue
- view.repo/view-queue
- plugin.panel/queue-tab
