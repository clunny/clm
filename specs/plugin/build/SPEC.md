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
The skill `build`, `plugin/skills/build/SKILL.md`: drain `clm next`, building each component in a background subagent in parallel up to the clone's `max_builds`, draft each bug group `clm triage` prints in a background subagent sharing those places, and archive the changes that complete; it ends with `clm wrap-up` and a summary that says whether the session can be ended. The rules it shares with other skills are `plugin`'s; `plugin`'s Contract says what each asks of this file.

## Design
`build` loops: run `clm triage` and draft each bug group it prints; run `clm next`; build each component it lists; repeat until neither has new work and no build or draft is left (`builds-in-subagents`, `build-drafts-bugs`). `clm next` prints at most as many components as the clone has free places (`build-places`). `clm run` exits 3 when another session's builds or drafts on the same clone hold the places: `build` launches nothing for that component and runs `clm next` again once one of its own builds returns, or, with none running, tells the human the cap is reached and by how many places, naming `max_builds` and `clm.local.toml`, and stops.

Whenever a build's subagent returns, landed or stopped, `build` runs `clm run --finish <component>` in its own checkout before anything else for that component, unless it resumes the build (below): it frees the build's place and, once the branch has landed, removes the build's worktree, whose `target/` `clm land` already deleted, so disk stays bounded by the cap rather than growing with the queue. It runs it through `clm` rather than `git worktree remove` because the worktree is another worktree, where a worktree-isolated session may not run git. A stopped build's claim and branch stay, so the retry below resumes them. Before any of that, `build` judges from why a build stopped whether to resume that same subagent or to build the component again (`resume-sound-builds`). A build whose work is sound and whose stop came from outside the work is resumed: a usage or rate limit, a full disk, a network outage, a tool or host failure that has passed. Its claim, worktree and context are intact, so building it again would throw that context away (what it tried, the review findings it weighed), compile the workspace again from an empty `target/` and restart the build from its last push. It keeps its place and counts as one of the session's builds still running, and is resumed with `SendMessage` once the cause has cleared (`build-resumes-sound-builds`), each time a usage or rate limit or a full disk stops it: building it again would meet the same limit or disk, so it would only repeat the work, and the limit's reset or the human's word that the disk is freed bounds each resume. When the same outside cause stops builds again, `build` acts on what brings it back where it can, such as parallel builds and their land checks filling the disk or reaching a limit, by resuming those builds one at a time and launching no new build until they have returned, and tells the human it recurred. A stop whose cause could lie in the work itself, such as a check or test that keeps failing, and one resumed at once, a network outage or a tool or host failure that had already passed, which nothing bounds but the retry and which may be the work's when it keeps coming back, is resumed at most once for the same reason; its second such stop is the retry's first. A build that went wrong in its approach or ran out of context is built again as below, from a fresh context. The resumed build returns like any other and is finished then. When it can't be resumed, it counts as a stop: `build` runs `clm run --finish <component>` and builds the component again as below.

`build` takes the queue's state from `clm next`, which fetches `origin` and reads `origin/main`, never from `clm status`, `clm view` or the state the session-start hook printed, which describe the session's checkout as of its last fetch and may be an old commit; when they disagree, `clm next` is right, and the checkout line (`view`'s `views-name-their-commit`) says why. It never pulls, merges or syncs the session's own checkout to bring them up to date, since that checkout may hold the human's work in progress. For the same reason its closing summary takes the requirements each `question` and `human-task` item blocks from `clm pending --at origin/main` (`view.repo/pending-names-its-commit`), never from `clm pending` alone, which describes the checkout. Before its first build it tells the human once that builds run under the session's permission mode, so in manual mode they pause for the human's answers, while auto or bypass lets them run alone; it never refuses a mode. To build a component it runs `clm run <component>` and, when that exits 0, passes its stdout to the Agent tool as a background `general-purpose` subagent with `isolation: "worktree"`, one per component; the build opens its pull request, lands its own branch with `clm land`, and when the subagent returns, its closing line gives the land (`landed <sha>`) or why it stopped (`stopped: <reason>`). A component that writes a protected path is built like the rest: Claude Code asks the human before the write. When a build stops without landing, `build` builds the component again the same way, which resumes on the component's branch, starts with `clm rebase` and lands it (`versioned-markers`); it retries a component until it lands or stops twice in a row for the same reason, then reports to the human. It doesn't do the component's work in its own context. A component whose branch landed may be listed again at once: its spec may have moved, or its last run may have stopped at its budget with work left pending, and either is just the next run. At the end it runs `clm wrap-up`, and when it prints any `complete <id>` line, archives those changes on one branch (`archive-branches`): on a branch `impl/archive/<id>` cut from `origin/main` in a worktree of its own (`session-worktrees`), `<id>` the first change the checkpoint lists `complete`, it runs `clm rebase`, which archives every complete change and deletes the items they resolve (`sync/archive-complete`), pushes, opens the pull request and lands it with `clm land`. The name has a `/` after `impl/`, which no component ID has, so the queue and `clm status` never read it as a claim (`orchestrator`). When the push is refused because `impl/archive/<id>` already exists on `origin`, another session is archiving the same changes: `build` deletes its local branch and leaves the archive to that session. It ends with the same kind of summary as `propose`, including the spec branches waiting for approval.

**Bugs** (`build-drafts-bugs`). Triage's bug path asks the human only to pick *Bugs* from its menu and, for a draft that needs review, to approve it. Running `/clm:build` is already a yes to unattended work, so `build` drafts the bug groups itself, exactly as `triage` drafts a picked `group` line (`plugin.triage`): `clm draft <slug>...` claims the group and takes a place (`orchestrator.triage/draft-takes-place`), and its prompt goes to a background worktree subagent on the session's own model, since a draft designs (`model-per-launch`). Drafts go before builds in a round: a bug's spec fix can make its component pending, which the same drain then builds, and a draft is usually short. A draft that lands on checks lands itself; one whose gate prints `spec review` stays on its branch, which `clm triage` then lists as a `designed` line, so the human approves it in the next `/clm:triage` and `build` asks no approval mid-drain (`humans-review-intent`). `build` takes only `group` lines: `human`, `idea`, `deferred` and `workaround` lines need the human, and stay `triage`'s. An item a draft of the session took is not drafted again in that session, since a draft that leaves an item releases its claim and the next `clm triage` would print it again. The defects the builds report are bug items too, so `build` needs no step of its own to fix them: once a build lands, they are on `origin/main` and the next round drafts them. No setting turns this off; a project that wants a human to read every bug first runs `/clm:triage`, whose drafts are the same. `clm next --wait` wakes for builds only; a bug filed while the session watches is drafted at the next drain.

**Its worktrees** (`session-worktrees`). `build` first removes a leftover `.claude/worktrees/archive` whose branch has landed; when another session's `impl/archive/<id>` is still unlanded, it leaves archiving to that branch, whose `clm rebase` archives every complete change, and says so. After landing and removing its archive worktree it runs `clm wrap-up` again before its summary. In a session isolated under `.claude/worktrees/`, `build` cuts its archive branch in the checkout it has, with `git switch -c impl/archive/<id> origin/main`; it removes no worktree, the leftover `.claude/worktrees/archive` included, which it can't reach, and once the branch has landed, or its push is refused, it switches the checkout off it with `git switch --detach origin/main`, before deleting a refused branch with `git branch -D` and before `clm wrap-up`.

**Tests** (`tests-count-where-cited`). The tests of this component's requirements sit in `crates/clm/tests/plugin_build.rs`, a test file of its own beside `plugin.rs`: its build moves each test that cites one of its requirements out of `plugin.rs` into that file, with copies of the helpers those tests use and the `clm:reads` markers for the files they read (`doc-readers-marked`), so its code count holds its own tests rather than the whole of `plugin.rs`. `plugin`'s tests of the shared rules read this component's file too; a build that changes the file keeps them passing.

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
`skills/build/SKILL.md` SHALL tell Claude to launch builds only for the components the latest `clm next` printed, which are at most the clone's free places, and to run `clm next` again after each build returns; to treat `clm run` exiting 3 as no free place, launching nothing for that component; to treat a `clm next` that prints nothing and writes a `no free build place` line to stderr (`orchestrator/next`) as no free place too, never as an empty queue, so that it never tells the human nothing is left to build; to run `clm run --finish <component>` whenever a build's subagent returns, whether it landed or stopped, before retrying or launching anything else for that component, but not for a build it resumes instead (`build-resumes-sound-builds`), such as one a usage or rate limit cut off; to pass each `left <path>: ...` line that `clm run --finish` prints to the human as it stands, a worktree it couldn't remove, and go on, leaving the directory to them; and, when no place is free, whichever of the two said so, and none of its own builds is running, to tell the human the cap is reached and by how many places, naming `max_builds` and `clm.local.toml`, and that other sessions' builds or drafts on the clone may hold the places, and then to run step 5 as at any drain, its summary saying the cap is reached in place of an empty queue, so the wrap-up runs and the watch offer (`build-watches`), whose `clm next --wait` waits for a place to free as it waits for work, follows (`next-wait`). It SHALL NOT tell Claude to run `git worktree remove` or `git worktree prune` on a build's worktree, or name `CARGO_TARGET_DIR`.

#### Scenario: build finishes builds
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm run --finish <component>`, `exits 3`, `no free build place`, `max_builds`, `clm.local.toml` and `left <path>`, and doesn't contain `CARGO_TARGET_DIR` or `git worktree prune`

#### Scenario: build without finishing
- GIVEN an `build` `SKILL.md` that doesn't mention `clm run --finish`
- WHEN the test's check runs on it
- THEN it reports `clm run --finish`

#### Scenario: full cap read as an empty queue
- GIVEN an `build` `SKILL.md` that doesn't mention `no free build place`
- WHEN the test's check runs on it
- THEN it reports `no free build place`

#### Needs
- orchestrator.run/run-finish
- orchestrator/next

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
`skills/build/SKILL.md` SHALL tell Claude to judge, when a build's subagent returns `stopped: <reason>`, or a usage or rate limit error from the API in place of a closing line of its own, whether to resume that same subagent or build the component again, by why it stopped (`resume-sound-builds`), a rule that comes before the rule for a subagent that ends without a closing line and before step 3's retry. It SHALL give the criteria: resume a build whose work is sound and whose stop came from outside the work, such as a usage or rate limit, a full disk, a network outage, or a tool or host failure that has passed; resume a build stopped by a usage or rate limit or a full disk each time that cause stops it and clears, never building it again for it, since the limit's reset or the human's word that the disk is freed bounds each resume; resume at most once for the same reason a build whose stop could lie in the work itself, such as a check or test that keeps failing, or that it resumes at once, a network outage or a tool or host failure that had already passed when it stopped, since such a cause that keeps coming back may be the work's, so that its second such stop is the retry's first; build again, with step 3's retry, a build that went wrong in its approach or ran out of context; and neither resume nor build again one that stopped blocked on a backlog item or a spec change. When the same outside cause stops builds again, it SHALL tell Claude to tell the human it recurred and to act on what brings it back where the session can, such as parallel builds and their land checks filling the disk or reaching a limit, by resuming the builds it stopped one at a time and launching no new build until they have returned. For a build it resumes, it SHALL tell Claude not to run `clm run --finish <component>` and to count it as one of its own builds still running, for the cap and for when to run `clm next` again; to tell the human why it stopped and, when only the human can clear the cause (such as freeing disk), what to clear; to resume it once the cause has cleared, after the limit has reset or on the session's first turn after the human says they cleared it (such as the human's next message), or at once for a failure that has already passed, with `SendMessage`, loading the tool with `ToolSearch` first when it is deferred, telling it why it stopped, that the cause has cleared, to carry on where it was in its own worktree, and to run again any review or check subagent of its own that the stop killed, since that one will never report back; to handle the resumed build like any other when it returns; and, when `SendMessage` returns an error or the subagent no longer exists, to count it as a stop, run `clm run --finish <component>` and build the component again with step 3's retry, never retrying the resume itself.

#### Scenario: build resumes a sound build
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `usage or rate limit`, `full disk`, `SendMessage`, `at most once`, `one at a time` and `resume-sound-builds`

#### Scenario: build without the resume
- GIVEN a `build` `SKILL.md` that doesn't mention `SendMessage`
- WHEN the test's check runs on it
- THEN it reports `SendMessage`

#### Scenario: build that resumes only limited builds
- GIVEN a `build` `SKILL.md` that doesn't mention `full disk`
- WHEN the test's check runs on it
- THEN it reports `full disk`

#### Scenario: build that rebuilds a recurring outside stop
- GIVEN a `build` `SKILL.md` that doesn't mention `one at a time`
- WHEN the test's check runs on it
- THEN it reports `one at a time`

#### Serves
- capability/build-from-spec
- capability/work-concurrently

### Requirement: build-follows-agent-harness
`skills/build/SKILL.md` SHALL hold `plugin/skills-follow-agent-harness`'s rule for its builds: it SHALL tell the session to run `clm agent-harness` before its first launch; where `isolated-worktrees` is `no`, to add each build's worktree with `git worktree add .claude/worktrees/agent-<component> origin/main` and tell the build, before the launcher `clm run` printed, to work in that directory, so `clm run --finish` and `clm wrap-up` tidy it as they tidy a subagent's `agent-*` worktree; where `subagents` is `no`, to build the components one at a time in the session itself, each in such a worktree; and, where `model-choice` is `no`, to launch with the session's own model and pass its name as `--review-model`.

#### Scenario: builds under Codex
- GIVEN `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it names `clm agent-harness`, `isolated-worktrees` and `.claude/worktrees/agent-<component>`



#### Needs
- clm/agent-harness-command

#### Serves
- capability/work-from-any-agent-harness

### Requirement: build-drafts-bugs
`skills/build/SKILL.md` SHALL tell Claude, in each round of its drain and before launching builds, to run `clm triage` and, for each `group` line it prints whose items no draft of this session has taken, in `clm triage`'s order and while a place is free, to run `clm draft <slug>...` with the line's items and, only when it exits 0, pass its stdout, unchanged, to the Agent tool as a background `general-purpose` subagent with `isolation: "worktree"` and `model` set to the session's own model; when `clm draft` exits 3, to launch nothing more for drafts that round. It SHALL tell Claude to handle a draft's return as `skills/triage/SKILL.md` does, by its closing line: a `waiting: <command>` line, or background work Claude Code still marks as running, holds the draft as still running unless the line is `landed <sha>` or `proposed <id> review` (`background-work-stopped`); otherwise to run `clm draft --finish <id>` and read `landed <sha>` as landed, `proposed <id> review` as a change left on its branch for review, and `stopped: <reason>` as a stop it tells the human and never retries; and to keep each `left <slug>: <reason>` line for the summary. It SHALL tell Claude to draft only `group` lines, never to pass `--answers`, `--fold`, or a `human`, `idea`, `deferred` or `workaround` line to `clm draft`, never to land or approve a change a draft left for review, and, in the summary, to list each such change with its branch and say that `/clm:triage` puts it up for approval. It SHALL tell Claude to end the drain only when `clm next` prints nothing with no `no free build place` line, `clm triage` prints no `group` line whose items no draft of this session has taken, and none of its own builds or drafts is left; and, after its archive branch lands, to run `clm triage` again and go back to the drain when it prints such a line.

#### Scenario: the shipped build skill
- GIVEN the plugin's `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm triage`, `clm draft <slug>...`, `clm draft --finish <id>`, `proposed <id> review`, `/clm:triage` and `build-drafts-bugs`

#### Scenario: build drafting what needs the human
- GIVEN a `build` `SKILL.md` that doesn't say it drafts only `group` lines
- WHEN the test's check runs on it
- THEN it reports `group`

#### Scenario: build with no draft step
- GIVEN a `build` `SKILL.md` that doesn't mention `clm draft`
- WHEN the test's check runs on it
- THEN it reports `clm draft`

#### Serves
- capability/build-from-spec
- capability/triage-unattended

#### Needs
- orchestrator.triage/draft-takes-place
