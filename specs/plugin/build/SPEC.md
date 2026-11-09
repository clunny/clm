---
title: Build skill
status: unbuilt
owners: ["@clunny"]
depends_on: []
paths: ["plugin/skills/build/**"]
successor: ""
serves: [architecture/method-in-the-plugin]
---

## Contract
The skill `build`, `plugin/skills/build/SKILL.md`: drain `clm next`, building each component in a background subagent in parallel up to the clone's `max_builds`, and archive the changes that complete; it ends with `clm wrap-up` and a summary that says whether the session can be ended. The rules it shares with other skills are `plugin`'s.

## Design
`build` loops: run `clm next`; build each component it lists; repeat until `clm next` is empty and no build is left (`builds-in-subagents`). `clm next` prints at most as many components as the clone has free places (`build-places`). `clm run` exits 3 when another session's builds or drafts on the same clone hold the places: `build` launches nothing for that component and runs `clm next` again once one of its own builds returns, or, with none running, tells the human the cap is reached and by how many places, naming `max_builds` and `clm.local.toml`, and stops.

Whenever a build's subagent returns, landed or stopped, `build` runs `clm run --finish <component>` in its own checkout before anything else for that component, unless it resumes the build (below): it frees the build's place and, once the branch has landed, removes the build's worktree, whose `target/` `clm land` already deleted, so disk stays bounded by the cap rather than growing with the queue. It runs it through `clm` rather than `git worktree remove` because the worktree is another worktree, where a worktree-isolated session may not run git. A stopped build's claim and branch stay, so the retry below resumes them. Before any of that, `build` judges from why a build stopped whether to resume that same subagent or to build the component again (`resume-sound-builds`). A build whose work is sound and whose stop came from outside the work is resumed: a usage or rate limit, a full disk, a network outage, a tool or host failure that has passed. Its claim, worktree and context are intact, so building it again would throw that context away (what it tried, the review findings it weighed), compile the workspace again from an empty `target/` and restart the build from its last push. It keeps its place and counts as one of the session's builds still running, and is resumed with `SendMessage` once the cause has cleared, at most once for the same reason (`build-resumes-sound-builds`). A build that went wrong in its approach, ran out of context, or stopped again for the reason it was resumed for is built again as below, from a fresh context. The resumed build returns like any other and is finished then. When it can't be resumed, it counts as a stop: `build` runs `clm run --finish <component>` and builds the component again as below.

`build` takes the queue's state from `clm next`, which fetches `origin` and reads `origin/main`, never from `clm status`, `clm view` or the state the session-start hook printed, which describe the session's checkout as of its last fetch and may be an old commit; when they disagree, `clm next` is right, and the checkout line (`view`'s `views-name-their-commit`) says why. It never pulls, merges or syncs the session's own checkout to bring them up to date, since that checkout may hold the human's work in progress. For the same reason its closing summary takes the requirements each `question` and `human-task` item blocks from `clm pending --at origin/main` (`view.repo/pending-names-its-commit`), never from `clm pending` alone, which describes the checkout. Before its first build it tells the human once that builds run under the session's permission mode, so in manual mode they pause for the human's answers, while auto or bypass lets them run alone; it never refuses a mode. To build a component it runs `clm run <component>` and, when that exits 0, passes its stdout to the Agent tool as a background `general-purpose` subagent with `isolation: "worktree"`, one per component; the build opens its pull request, lands its own branch with `clm land`, and when the subagent returns, its closing line gives the land (`landed <sha>`) or why it stopped (`stopped: <reason>`). A component that writes a protected path is built like the rest: Claude Code asks the human before the write. When a build stops without landing, `build` builds the component again the same way, which resumes on the component's branch, starts with `clm rebase` and lands it (`versioned-markers`); it retries a component until it lands or stops twice in a row for the same reason, then reports to the human. It doesn't do the component's work in its own context. A component whose branch landed may be listed again at once: its spec may have moved, or its last run may have stopped at its budget with work left pending, and either is just the next run. At the end it runs `clm wrap-up`, and when it prints any `complete <id>` line, archives those changes on one branch (`archive-branches`): on a branch `impl/archive/<id>` cut from `origin/main` in a worktree of its own (`session-worktrees`), `<id>` the first change the checkpoint lists `complete`, it runs `clm rebase`, which archives every complete change and deletes the items they resolve (`sync/archive-complete`), pushes, opens the pull request and lands it with `clm land`. The name has a `/` after `impl/`, which no component ID has, so the queue and `clm status` never read it as a claim (`orchestrator`). When the push is refused because `impl/archive/<id>` already exists on `origin`, another session is archiving the same changes: `build` deletes its local branch and leaves the archive to that session. It ends with the same kind of summary as `propose`, including the spec branches waiting for approval.

## Requirements

### Requirement: skill-named-build
The plugin SHALL ship the skill that builds the approved specs as `skills/build/SKILL.md` and no `skills/implement/` directory, its heading and its description saying build. The description SHALL contain `approved specs` and `not compile`, so that nobody takes `/clm:build` for a compiler. No file under `plugin/` or `docs/guide/`, nor `CLAUDE.md`, nor `README.md`, SHALL name `/clm:implement` or `skills/implement/`.

#### Scenario: the skill and its description
- GIVEN the plugin
- WHEN the tests run
- THEN `skills/build/SKILL.md` exists with a `description` holding `approved specs` and `not compile`, and `skills/implement/` doesn't exist

#### Scenario: an old mention left behind
- GIVEN a guide page or a skill that still says `/clm:implement`
- WHEN the tests run
- THEN the test fails, naming the file and the line

### Requirement: build-subagents
`skills/build/SKILL.md` SHALL tell Claude to say once, before the first build, that builds run under the session's permission mode and pause for the human's answers in manual mode, and never to refuse a mode. It SHALL tell Claude to build each component `clm next` prints by running `clm run <component>` and, only when it exits 0, passing its stdout to the Agent tool as a background subagent with `isolation: "worktree"`; when the subagent returns, to read its closing `landed <sha>` line, or tell the human the `stopped:` reason; to build a component again the same way when it stopped without landing and isn't one it resumes (`build-resumes-sound-builds`), until it lands or stops twice for the same reason; and never to do a component's work in its own context. It SHALL NOT name `clm next --interactive`, `.clm/logs`, `clm watch` or `PR #<number>`.

#### Scenario: build launches subagents
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm run <component>`, `Agent`, `background`, `isolation: "worktree"`, `manual`, `landed <sha>` and `stopped:`

#### Scenario: build without isolation
- GIVEN an `build` `SKILL.md` that doesn't mention `isolation`
- WHEN the test's check runs on it
- THEN it reports `isolation`

#### Scenario: build watching
- GIVEN an `build` `SKILL.md` that mentions `clm watch`
- WHEN the test's check runs on it
- THEN it reports `clm watch`

#### Serves
- capability/build-from-spec
- capability/work-concurrently

### Requirement: build-archive-branch
`skills/build/SKILL.md` SHALL tell Claude to archive the changes `clm wrap-up` lists `complete` on one branch `impl/archive/<id>`, `<id>` the first change it lists, cut from `origin/main`, and, when pushing that branch is refused because it already exists on `origin`, to delete the local branch and leave the archive to the session that pushed it (`archive-branches`). It SHALL NOT name a branch `impl/archive` without `/<id>`, which the queue would read as a claim on a component `archive`.

#### Scenario: nested archive branch
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `impl/archive/<id>`

#### Scenario: archive branch read as a claim
- GIVEN an `build` `SKILL.md` that names `impl/archive` followed by a space or a backtick
- WHEN the test's check runs on it
- THEN it reports `impl/archive/<id>`

#### Needs
- clm/renamed-commands

#### Serves
- capability/wrap-up-session
- capability/work-concurrently

### Requirement: build-reads-origin
`skills/build/SKILL.md` SHALL tell Claude to take the queue's state from `clm next`, which fetches and reads `origin/main`, and not from `clm status`, `clm view` or the session-start state, which describe the session's checkout as of its last fetch; to trust `clm next` when they disagree; and never to pull, merge or sync the session's own checkout to bring them up to date (`views-name-their-commit`). It SHALL tell Claude to learn what a build has done from its branch on `origin`, `origin/impl/<component>`, which the build pushes at each sync point, and from `clm wrap-up`, never from a command aimed at the build's worktree, such as `git -C <worktree>`, which Claude Code's worktree isolation refuses. It SHALL tell Claude to take the requirements its closing summary lists under each `question` and `human-task` backlog item from `clm pending --at origin/main`, run after `clm next` has fetched, and not from `clm pending` without `--at`, which describes the checkout.

#### Scenario: build reads origin
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `origin/main`, `clm status`, `views-name-their-commit`, `origin/impl/<component>` and `clm pending --at origin/main`

#### Scenario: build without the rule
- GIVEN an `build` `SKILL.md` that doesn't mention `views-name-their-commit`
- WHEN the test's check runs on it
- THEN it reports `views-name-their-commit`

#### Scenario: build inspecting a build's worktree
- GIVEN an `build` `SKILL.md` that doesn't mention `origin/impl/<component>`
- WHEN the test's check runs on it
- THEN it reports `origin/impl/<component>`

#### Scenario: summary reading the checkout
- GIVEN a `build` `SKILL.md` that doesn't mention `clm pending --at origin/main`
- WHEN the test's check runs on it
- THEN it reports `clm pending --at origin/main`

#### Needs
- clm/renamed-commands
- view.repo/pending-names-its-commit

#### Serves
- capability/build-from-spec
- architecture/git-is-truth

### Requirement: build-finishes-builds
`skills/build/SKILL.md` SHALL tell Claude to launch builds only for the components the latest `clm next` printed, which are at most the clone's free places, and to run `clm next` again after each build returns; to treat `clm run` exiting 3 as no free place, launching nothing for that component; to run `clm run --finish <component>` whenever a build's subagent returns, whether it landed or stopped, before retrying or launching anything else for that component, but not for a build it resumes instead (`build-resumes-sound-builds`), such as one a usage or rate limit cut off; to pass each `left <path>: ...` line that `clm run --finish` prints to the human as it stands, a worktree it couldn't remove, and go on, leaving the directory to them; and, when no place is free and none of its own builds is running, to tell the human the cap is reached, naming `max_builds` and `clm.local.toml`. It SHALL NOT tell Claude to run `git worktree remove` or `git worktree prune` on a build's worktree, or name `CARGO_TARGET_DIR`.

#### Scenario: build finishes builds
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm run --finish <component>`, `exits 3`, `max_builds`, `clm.local.toml` and `left <path>`, and doesn't contain `CARGO_TARGET_DIR` or `git worktree prune`

#### Scenario: build without finishing
- GIVEN an `build` `SKILL.md` that doesn't mention `clm run --finish`
- WHEN the test's check runs on it
- THEN it reports `clm run --finish`

#### Needs
- orchestrator.run/run-finish

#### Serves
- capability/build-from-spec
- capability/work-concurrently

### Requirement: build-watches
`skills/build/SKILL.md` SHALL tell Claude, once step 5 has ended with its summary, `clm next` printing nothing and none of its own builds left, to offer the human once per session to watch for new work (`next-wait`). On a yes it SHALL tell Claude to run `clm next --wait` as a background Bash command (`run_in_background`), which wakes the session when it exits; when it exits 0, to start again at step 1, taking the queue from a fresh `clm next`, without repeating the notice given before the first build; and when it exits non-zero, to tell the human what it printed and stop watching. It SHALL tell Claude that the watch, once on, stays on for the rest of the session: after each later drain it runs step 5 again, its summary saying the session is watching and that the human can say to stop, and runs `clm next --wait` again without asking; it answers the human's messages meanwhile and keeps watching; and when the human says to stop, it stops the background command (`TaskStop`) and runs nothing more. It SHALL NOT make the watch a setting that outlives the session.

#### Scenario: build offers to watch
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm next --wait`, `run_in_background`, `step 1`, `TaskStop` and `next-wait`

#### Scenario: build without the wait
- GIVEN an `build` `SKILL.md` that doesn't mention `clm next --wait`
- WHEN the test's check runs on it
- THEN it reports `clm next --wait`

#### Serves
- capability/build-from-spec

#### Needs
- orchestrator/next-wait

### Requirement: build-reports-workarounds
`skills/build/SKILL.md` SHALL tell Claude to list in its summary each `workaround` item and each `reconsider-<component>-<slug>` question the builds filed, and to point at `/clm:health` for the whole picture.

#### Scenario: build without the pointer
- GIVEN a `skills/build/SKILL.md` that doesn't mention `reconsider-<component>-<slug>` and `/clm:health`
- WHEN the tests run
- THEN a test fails naming the skill and the missing text

#### Serves
- capability/know-health

#### Needs
- model/parse-backlog

### Requirement: build-waiting-builds
`skills/build/SKILL.md` SHALL tell Claude that a build whose subagent's result ends with a `waiting: <command>` line, or that Claude Code marks as still running background work of its own, hasn't returned, whatever its last line says, a `stopped:` line included, unless that line is `landed <sha>`: the build is waiting for a command it started, such as its own `clm land`, and finishing it would free its place and remove its worktree while that command runs (`orchestrator.run/background-work-stopped`). This rule SHALL come before the rule for a subagent that ends without a closing line. For such a build it SHALL tell Claude not to run `clm run --finish <component>`, not to tell the human it stopped and not to build the component again; to count it as one of its own builds still running, for the cap and for when to run `clm next` again; and to handle the subagent's next result, which arrives once that command has ended, as the build's return.

#### Scenario: build waits for a waiting build
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `waiting: <command>`, `whatever its last line says` and `background-work-stopped`

#### Scenario: build without the wait for a waiting build
- GIVEN an `build` `SKILL.md` that doesn't mention `waiting: <command>`
- WHEN the test's check runs on it
- THEN it reports `waiting: <command>`

#### Serves
- capability/build-from-spec
- capability/work-concurrently

### Requirement: build-stops-landed-builds
`skills/build/SKILL.md` SHALL tell Claude that a build whose subagent's result ends with `landed <sha>` while Claude Code marks it as still running background work of its own has returned, the exception the waiting rule (`build-waiting-builds`) makes for `landed <sha>`: its land has ended, and what still runs is a leftover task the build failed to stop, against `orchestrator.run/background-work-stopped`. It SHALL tell Claude to stop that subagent with `TaskStop`, by the task ID its notifications carry, loading the tool with `ToolSearch` first when it is deferred, before it runs `clm run --finish <component>` for the build, so the leftover task neither runs on in the worktree the finish removes nor wakes the session later; when `TaskStop` fails because the subagent has already ended, to go on; and then to read the build's `landed <sha>` line as for any landed build. This rule SHALL be in the step that reads a build's return, right after the waiting rule and before the rule for a build a usage or rate limit cut off.

#### Scenario: build stops a landed build's leftover task
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `leftover task`, and the sentence that contains it also contains `landed <sha>`, and a sentence after it contains `TaskStop` and `clm run --finish <component>`

#### Scenario: build without stopping a landed build's leftover task
- GIVEN an `build` `SKILL.md` that doesn't mention `leftover task`
- WHEN the test's check runs on it
- THEN it reports `leftover task`

#### Serves
- capability/build-from-spec
- capability/work-concurrently

### Requirement: build-says-what-is-ready
`skills/build/SKILL.md` SHALL say, where it tells Claude to run `clm next`, that `clm next` prints the unclaimed components with a pending requirement that no `#### Needs` and no blocking backlog item holds back, and that a component's `depends_on` doesn't order builds, naming `needs-alone-order` and `component-reconciliation`. It SHALL NOT say that `clm next` waits for a component's dependencies to be settled or built, nor that a landed component settles the components that depend on it (a landed component may only free the requirements whose `#### Needs` name its requirements), since the queue doesn't order by `depends_on` (`orchestrator/next`), so a session following the skill never waits on, or explains the queue by, an order CLM doesn't apply.

#### Scenario: the shipped build skill
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `needs-alone-order`, `component-reconciliation`, `#### Needs` and `depends_on`, and contains neither `dependencies are all settled` nor `settles the components that depend on it`

#### Scenario: a build skill that orders by dependencies
- GIVEN a `build` `SKILL.md` that says `clm next` prints the components whose dependencies are all settled
- WHEN the test's check runs on it
- THEN it reports `dependencies are all settled`

#### Serves
- capability/build-from-spec

### Requirement: build-unclosed-builds
`skills/build/SKILL.md` SHALL tell Claude that a build whose subagent ends without a closing line of its own, neither `landed <sha>` nor `stopped: <reason>` (for example out of context, or after an earlier result that ended with `waiting: <command>`), and isn't one the waiting rule (`build-waiting-builds`) or the resume rule (`build-resumes-sound-builds`) holds back, is judged by what `clm run --finish <component>` printed for it, which it runs for that build as for any other return (`unclosed-build-reads-finish`): when that output holds the line `landed impl/<component>`, the build landed, so Claude tells the human that it landed without its closing line, doesn't count it as a stop and doesn't build it again for that, and runs `clm next` again as after any landed build; otherwise it counts as a stop, retried as a stopped build is. This rule SHALL be in the step that reads a build's return, where the skill says what a subagent that ends without either line counts as. It SHALL NOT tell Claude to judge such a build by reading `origin` with a `git` command of its own.

#### Scenario: build reads the finish of an unclosed build
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `landed impl/<component>` and `unclosed-build-reads-finish`, and the sentence that contains `landed impl/<component>` also contains `clm run --finish <component>`

#### Scenario: build that counts every unclosed build as a stop
- GIVEN a `build` `SKILL.md` that doesn't mention `landed impl/<component>`
- WHEN the test's check runs on it
- THEN it reports `landed impl/<component>`

#### Needs
- orchestrator.run/run-finish

#### Serves
- capability/build-from-spec
- capability/work-concurrently

### Requirement: build-resumes-sound-builds
`skills/build/SKILL.md` SHALL tell Claude to judge, when a build's subagent returns `stopped: <reason>`, or a usage or rate limit error from the API in place of a closing line of its own, whether to resume that same subagent or build the component again, by why it stopped (`resume-sound-builds`), a rule that comes before the rule for a subagent that ends without a closing line and before step 3's retry. It SHALL give the criteria: resume a build whose work is sound and whose stop came from outside the work, such as a usage or rate limit, a full disk, a network outage, or a tool or host failure that has passed; build again, with step 3's retry, a build that went wrong in its approach, ran out of context, or stopped for the same reason it was already resumed for, so that a build is resumed at most once for the same reason and its second such stop is the retry's first; and neither resume nor build again one that stopped blocked on a backlog item or a spec change. For a build it resumes, it SHALL tell Claude not to run `clm run --finish <component>` and to count it as one of its own builds still running, for the cap and for when to run `clm next` again; to tell the human why it stopped and, when only the human can clear the cause (such as freeing disk), what to clear; to resume it once the cause has cleared, after the limit has reset or on the session's first turn after the human says they cleared it (such as the human's next message), or at once for a failure that has already passed, with `SendMessage`, loading the tool with `ToolSearch` first when it is deferred, telling it why it stopped, that the cause has cleared, to carry on where it was in its own worktree, and to run again any review or check subagent of its own that the stop killed, since that one will never report back; to handle the resumed build like any other when it returns; and, when `SendMessage` returns an error or the subagent no longer exists, to count it as a stop, run `clm run --finish <component>` and build the component again with step 3's retry, never retrying the resume itself.

#### Scenario: build resumes a sound build
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `usage or rate limit`, `full disk`, `SendMessage`, `at most once` and `resume-sound-builds`

#### Scenario: build without the resume
- GIVEN a `build` `SKILL.md` that doesn't mention `SendMessage`
- WHEN the test's check runs on it
- THEN it reports `SendMessage`

#### Scenario: build that resumes only limited builds
- GIVEN a `build` `SKILL.md` that doesn't mention `full disk`
- WHEN the test's check runs on it
- THEN it reports `full disk`

#### Serves
- capability/build-from-spec
- capability/work-concurrently
