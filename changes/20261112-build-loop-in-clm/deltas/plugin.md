---
component: plugin
base: { contract: "81e3204bb14df39d", design: "2dfc469c96b54996" }
---

## MODIFIED Contract
The plugin `clm`, one set of files packaged for Claude Code and for Codex (`agent-harness-adapters`), each installed from this repo's marketplace for that agent harness. It provides, on both:
- the `clm` command on the shell tool's PATH;
- `clm defect`, which reports a defect in CLM itself as `config`'s `defects` setting says: as a backlog item in CLM's own repository, or as an issue filed through the host that the human approves (`defect-reports`, `host-adapters`); its rules are `plugin.hooks`';
- hooks that call `clm hook <name> --agent-harness <agent harness>`, whose rules are `plugin.hooks`': to show the repo's state at session start, with the rule for reporting CLM defects, to have the human confirm `clm approve`, to block cutting releases and pushing to `main`, to have the human approve each defect issue, and to autosync spec work at the end of each turn;
- the skills `propose` (`plugin.propose`), `build` (`plugin.build`), `triage` (`plugin.triage`), `init` (`plugin.init`) and `upgrade` (`plugin.upgrade`), each its child's;
- skill `contribution` (take in a contributor's pull request or issue on the maintainer's side: judge its files, show it to the human, close it or land it on their yes, and credit its author; `clm guide contributions` says why);
- skill `help` (answer a person's questions about CLM, read-only, from what the installed `clm` carries: the guide, `clm guide constraints` and, in a repository under CLM, its state; it points at the skill that acts and changes nothing);
- skill `health` (run `clm health`, summarise its sections for the human, launch `spec-slimmer` on each component its bloat signals mark, and list what to take to `propose`: the reconsider candidates and the cuts; it changes nothing itself);
- skill `configure` (review CLM's settings or change one);
- skill `verify` (verify `origin/main` as it is now, triggers aside, with `clm verify --commit origin/main`; on a red result show the failing step and its last lines, re-run the failing tests when it can, and offer to take the failure in at `P1`);
- the subagents `spec-auditor`, `architecture-auditor`, `spec-slimmer` and `issue-verifier`, which are `plugin.agents`'.

The rules several skills share are this component's, though the files they constrain are its children's. What each asks of the skills:
- `skills`: `propose` and `build` never push `main` by any means but `clm land`, and run `clm approve` only after the human's yes, through the guard; no skill or agent names `clm hash`, only `clm hashes`;
- `builder-check` and `architecture-audit`: `propose` runs a builder check and, beside it, the `architecture-auditor` before its pull request; `build`'s loop keeps the `question` and `human-task` items that block what it leaves in the human's queue (`orchestrator.loop/humans-queue`); `contribution` launches the auditor too;
- `session-worktrees`: `propose` and `init` cut each branch in a worktree of its own under `.claude/worktrees/`, never switching the checkout the session started in, except in a session already isolated there; `triage` cuts its archive branch the same way, a rule its own requirement states (`plugin.triage/triage-archives-complete`);
- `defect-check-before-pr`: `propose` reviews the session for CLM defects before any pull request and fixes the reported ones once its branches have landed; `build`, which opens no pull request, reports each when it happens, and its loop drafts them with the backlog's other bugs (`orchestrator.loop/plan-order`);
- `recommend-the-right-design`: `triage` and `propose` read the specs and code before recommending, and recommend the correct design over the convenient one;
- `skills-write-serves` and `skills-write-enforcement`: `propose` and `init` give components `serves`, and constraints `#### Enforced by` or `#### Untested`;
- `launch-models`: every subagent a skill launches has its model named;
- `skills-follow-agent-harness`: every skill reads `clm agent-harness` and follows what the agent harness can do, in words no single agent harness owns;
- `skills-push-with-clm`: no skill says `git push`; pushes and remote deletes go through `clm branch push` and `clm branch delete`;
- `unattended-permissions-check` and `skills-run-verify`: `build` and `triage` check the allow rules before their first launch; `triage` runs `clm verify --due --event work-done` before `clm wrap-up`, and `build`'s loop runs `clm verify` when the project's triggers say (`orchestrator.loop/plan-verify`);
- `wrap-up-left-lines`: a skill that runs `clm wrap-up` passes each `left` line to the human and never prunes a worktree itself;
- `method-pointers`: no skill or agent names `docs/guide/` but `propose`'s two pages, each `clm guide <page>` named is a page, and `init` explains graph shape before the first component map;
- `renamed-commands`, `says-architectural-constraint`, `skills-say-new-words` and `skills-say-component`: every file under `plugin/` uses the current command names, terms and words.

This component's build never edits a child's file (`path-ownership`). A change to a shared rule adds a requirement to each child whose file must change, and the shared rule's `#### Needs` names them, so it waits until each child has built its part (`dependency-work-is-a-requirement`, `requirement-needs`).

The CLM panel, a second plugin `clm-panel` in the Claude Code marketplace, is `plugin.panel`; it is Claude Code's alone.

The plugin's version is the `clm` version it runs, in both packages. Users update with `claude plugin update clm`, or Codex's own plugin update, and the new version's shim downloads its binary on first use.

An agent of the generic tier, which has no package, gets CLM from the `clm` binary and the project's `AGENTS.md` alone (`platform`).

The skills carry CLM's method to every adopter (`method-in-the-plugin`): what they don't say themselves, they point at as a page of the guide that `clm guide <page>` prints, never as a path in this repository.

## MODIFIED Design
The plugin is `plugin/` in this repository: `.claude-plugin/plugin.json` (`clm`, versioned with the binary), `bin/clm` (the shim), `hooks/hooks.json`, the skills and agents the Contract lists, and `commands/view.md`. The skills `propose`, `build`, `triage`, `init` and `upgrade` and the four subagents are children's, one per file (`nested-splits`); this component keeps the files no child owns and the rules the skills share. The last makes `/clm:view` exist: with `clm-panel` installed, its hooks module answers the command itself (`plugin.panel`); without it, the file has the model run `clm status` or `clm view <name>` through the Bash tool. The `clm` CLI holds every rule; the plugin only runs it (`plugin-plus-rust-cli`, `hook-logic-in-clm`). The rules behind `clm hook` and `clm defect` are `plugin.hooks`'; the panel is `plugin.panel`.

**The shim** (`binary-from-releases`). `bin/clm` resolves the binary for the plugin's version: `${XDG_CACHE_HOME:-~/.cache}/clm/<version>/clm`, downloading `clm-<target>.tar.gz` from the matching GitHub release and verifying its `.sha256` when it is missing, then runs it with the arguments it was given. `$CLM_BIN` overrides it, for developing against a local build. It is a POSIX `sh` script and uses `curl`, `tar` and `sha256sum` or `shasum`: it downloads both files with `curl -fsSL` from `https://github.com/clunny/clm/releases/download/v<version>/`, the public repository's release, so no `gh`, sign-in or token is needed (`binary-from-releases`). A missing `curl` is a failed download. The guard fails open (`guard-fails-open`): when the binary `$CLM_BIN` names is missing or can't run, or `clm hook guard` crashes, the hook exits with neither 0 nor 2, which Claude Code doesn't treat as a block, and the shim adds no wrapper that exits 2, since a session whose every `Bash` call is blocked can't rebuild the binary that would unblock it.

**Hooks** (`hooks-wired`). What each hook does with Claude Code's event is `plugin.hooks`.

**Skills.** The skills follow the repo's change flow (`CLAUDE.md`). A spec branch either needs review or lands on checks, and nothing is reviewed after it lands (`yes-no-review`). There are no labels and no `gh` calls in the skills: pull requests are opened with `clm pr open` and bodies set with `clm pr body` (`host-adapters`).

**A worktree per session branch** (`session-worktrees`). A skill that cuts a branch in the human's session never switches the checkout the session started in, which other sessions on the same clone share: one session's `git switch` would move another's `HEAD` between its commands, so that session's next commits would land on the first session's branch, and autosync would push them to its `autosync/spec/<id>`, with nothing to warn either. The worktree is inside the session's project, so the session's `cd` into it persists between Bash calls, and the `Stop` hook's autosync, which finds the repository from the event's `cwd`, snapshots the worktree's branch. The checkout the session started in stays on its branch, usually `main`. When the branch has landed, the skill changes back to that checkout, removes the worktree, then runs `clm wrap-up` there, which deletes the landed branch. A worktree whose branch hasn't landed, such as a change waiting for approval, stays with its branch for the session that picks the work up: a session resuming `spec/<id>` changes into `.claude/worktrees/<id>` when it exists, and otherwise adds it for the existing branch with `git worktree add .claude/worktrees/<id> spec/<id>` (no `-b`), tracking `origin/spec/<id>` when only `origin` has it. The session writes its scratch files, such as the pull request body, under `.clm/`, which `clm run` and `clm draft` keep out of git, so a plain `git worktree remove` succeeds; when it refuses anyway, the skill reports the path and leaves the worktree, never forcing it. Wherever the session started, on `main` or another branch of the shared checkout, or in a worktree the human added, it adds the new worktree from `origin/main` and changes back to that same directory at the end.

A session whose checkout already lies under `.claude/worktrees/` (its `git rev-parse --show-toplevel` says so), in a directory the skills don't add, is the exception. Claude Code names a subagent's worktree `agent-<id>`, while the skills name theirs by change ID or `archive`; a human session started in one of the skills' worktrees is no more isolated than the checkout that made it, so it adds a worktree as any other session does. Claude Code isolates a subagent's session in its worktree, refusing every write and every git operation that targets another worktree, so such a session can't use a worktree it adds, even when the `git worktree add` itself goes through, and needs none, since a worktree there is one session's own and no other session's `HEAD` moves when it switches. Builds and drafts need none of this: they already run in the worktree the Agent tool's `isolation: "worktree"` gives them (`builds-in-subagents`).

**Shared steps** (`defect-check-before-pr`). `propose` resolves what `clm rebase` leaves it, as the runs of `build`'s loop do: a `spec changed` report by reading it and `clm pending <component>` and adapting the work, conflicted files by hand, stale deltas by rebasing them on `clm hashes`, check failures by fixing them. `propose` reviews the session for CLM defects before opening any pull request, and `build` reports each when it happens (`defect-check-before-pr`). Where `clm defect` writes to the backlog, `propose` then **fixes reported defects** once its branch has landed, before `clm wrap-up` and the summary (`defect-reports`), in the branch's worktree switched with `git switch --detach origin/main`; the fix branch's pull request lists the defect as fixed, and the landed pull request that reported it isn't edited. `build` has no such step: the defects its runs report are `kind: bug` items, which its loop drafts with the backlog's other bugs once they reach `origin/main` (`orchestrator.loop/plan-order`).

**The method** (`method-in-the-plugin`, `first-graph-teaches-shape`). An adopter's project has none of this repository's `docs/`, `ARCHITECTURE.md` rationale or specs, so the skills are where it learns the method. `clm guide <page>` prints a page from the binary the shim runs, offline and at the plugin's version (`guide-in-the-binary`), so a skill names it and not `docs/guide/...`, a path only this repository has, except the two pages a spec branch edits that `method-pointers` lets `propose` name. A test per skill keeps its pointers, and a test checks that each page a skill names is a file in `docs/guide/`.

Builds use superpowers as `unattended-superpowers` decides; their prompt is `orchestrator.run`'s `clm run`.

**No eval cases** (`cut-evals`). The plugin has no `evals/` directory. The skills' tests read their text, `claude plugin validate plugin` checks that Claude Code loads the plugin (`ci/plugin-validate`), and a skill's behaviour is judged where it is used, in sessions.

## MODIFIED Requirements
### Requirement: skills <!-- base: b943c3b9783b88fe -->
The plugin SHALL ship `skills/propose/SKILL.md` and `skills/build/SKILL.md`, each with a `description` in its frontmatter. Each SHALL tell Claude never to push `main` by any means but `clm land`, and to run `clm approve` only after the human says yes in the conversation, through the guard's confirmation. No skill or agent SHALL name `clm hash` (the word `hash` as the subcommand), which Claude Code's worktree isolation refuses: they SHALL name `clm hashes`. `propose` SHALL name `clm hashes`, `clm check`, `clm apply`, `clm rebase`, `clm gate`, `spec review`, `human_burdens`, `clm pr open`, `clm approve`, `clm land` and `clm wrap-up`, and the section `## Intent`, and SHALL NOT tell Claude to set `after` or `prefer_after`. It SHALL tell Claude that a branch whose `clm gate` prints `spec review` as its first line lands only after the human's `clm approve`, and that any other spec branch lands on checks (`yes-no-review`). No skill SHALL name `gh pr`, `gh api`, `spec-approved`, `spec-review`, `clm watch`, `clm review`, `clm significant`, `## Significant`, the field `` `gate` ``, `gate: `, or the review tiers `HARD`, `SOFT` and `AUTO`. `build` SHALL name `clm build start`, `clm build step`, `clm build launched`, `clm build returned`, `clm build stop` and `isolation`, since the build loop decides every step and archives the complete changes itself (`orchestrator.loop`). No `SKILL.md` SHALL hold `` `clm rebase` until it exits 0 ``: `propose` and `init` SHALL tell Claude to run `clm rebase` until its last line is `rebase ok`, and to read its outcome from that line, never from its exit code.

#### Scenario: skill without the rule
- GIVEN a `SKILL.md` that doesn't mention `clm approve`
- WHEN the tests run
- THEN a test fails naming the skill

#### Scenario: build without the loop
- GIVEN a `build` `SKILL.md` that doesn't mention `clm build returned`
- WHEN the tests run
- THEN a test fails naming the skill and `clm build returned`

#### Scenario: skill without land
- GIVEN a `propose` `SKILL.md` that doesn't mention `clm land`
- WHEN the tests run
- THEN a test fails naming the skill and `clm land`

#### Scenario: skill that orders changes
- GIVEN a `propose` `SKILL.md` that tells Claude to set `prefer_after`
- WHEN the tests run
- THEN a test fails naming the skill and `prefer_after`


#### Scenario: skill naming a label
- GIVEN a `propose` `SKILL.md` whose approval step says `gh api -X POST repos/{owner}/{repo}/issues/<pr>/labels -f 'labels[]=spec-approved'`
- WHEN the tests run
- THEN a test fails naming the skill and `gh api`

#### Scenario: skill naming a tier
- GIVEN a `propose` `SKILL.md` that says a `SOFT` change is reviewed with `clm review <id>` after it lands, and in a second run an `init` `SKILL.md` that writes `gate: HARD` into `change.md`
- WHEN the tests run
- THEN each time a test fails naming the skill and `SOFT` or `gate: `

#### Scenario: skill naming the old command
- GIVEN a `propose` `SKILL.md` that says to rebase stale deltas on `clm hash`
- WHEN the tests run
- THEN a test fails naming the skill and `clm hash`

#### Scenario: sync read by its exit code
- GIVEN a `propose` `SKILL.md`, and in a second run an `init` `SKILL.md`, whose step says to run `clm rebase` until it exits 0
- WHEN the tests run
- THEN each time a test fails naming the skill and `rebase ok`

#### Needs
- plugin.build/build-runs-the-loop
- clm/renamed-commands
- model/parse-changes
- model/parse-backlog

#### Serves
- capability/propose-intent
- capability/build-from-spec
- architecture/land-locally
- architecture/approval-policy

### Requirement: defect-check-before-pr <!-- base: abdfad5ca819bd51 -->
`skills/propose/SKILL.md` SHALL tell Claude, before opening any PR, to review the session for CLM defects neither reported nor fixed, report each with `clm defect` unless the session-start rule has it folded into the branch, naming with `--component` the component a fix would lie in, and list every defect of the session under `## CLM defects` in the PR body, each fixed one with its fix. `skills/propose/SKILL.md` SHALL also have a step, **fix reported defects**, for when `clm defect` writes to the backlog: once the session's branch has landed and before `clm wrap-up` and the summary, fix each small defect the session reported whose item is on `origin/main`, one at a time and one round only, with `clm claim <component> --fix backlog/<slug>` as `propose`'s code fixes do, in the branch's worktree and never in the checkout the session started in, unless that checkout is an isolated session's own under `.claude/worktrees/` (`session-worktrees`), where the fix is made in it; fold a defect met during such a fix when it lies in that fix's component and leave it filed otherwise; and when a claim is refused or a fix turns out large, leave the item filed, release a claim it took by deleting its branch on `origin`, and name the item in the summary. `skills/build/SKILL.md` SHALL tell Claude to report each CLM defect it meets with `clm defect` when it happens, naming with `--component` the component a fix would lie in, since the session opens no pull request of its own; it SHALL have no step holding `fix reported defects`: its loop drafts the defects reported to the backlog with the backlog's other bugs (`orchestrator.loop/plan-order`).

#### Scenario: skill without the check
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `## CLM defects`, and in a second run a `skills/build/SKILL.md` that doesn't mention `clm defect`
- WHEN the tests run
- THEN each time a test fails naming the skill and `## CLM defects` or `clm defect`

#### Scenario: skill without fixed defects
- GIVEN a `skills/propose/SKILL.md` whose defect check doesn't say `each fixed one with its fix`
- WHEN the tests run
- THEN a test fails naming the skill and `each fixed one with its fix`

#### Scenario: skill without the fix step
- GIVEN a `skills/propose/SKILL.md` with no paragraph that holds `fix reported defects`, `clm claim <component> --fix backlog/<slug>`, `origin/main` and `clm wrap-up`
- WHEN the tests run
- THEN a test fails naming the skill and `fix reported defects`

#### Scenario: build keeping its own fix step
- GIVEN a `skills/build/SKILL.md` with a paragraph that holds `fix reported defects`
- WHEN the tests run
- THEN a test fails naming the skill and `fix reported defects`

#### Needs
- plugin.build/build-runs-the-loop
- clm/renamed-commands

#### Serves
- capability/report-defects

### Requirement: session-worktrees <!-- base: 651acb1c56635e61 -->
`skills/propose/SKILL.md` SHALL tell Claude to cut `spec/<id>` with `git worktree add .claude/worktrees/<id> -b spec/<id> origin/main`, to change into that worktree and do all later work there, never switching the checkout the session started in onto another branch, and, once the branch has landed, to change back to that checkout and remove the worktree with `git worktree remove` before `clm wrap-up`. `skills/init/SKILL.md` SHALL tell Claude to cut its spec branch the same way, in `.claude/worktrees/<YYYYMMDD>-init`, after the bootstrap push (`session-worktrees`). `skills/build/SKILL.md` cuts no branch: its loop's runs work in worktrees of their own (`orchestrator.loop`). `skills/propose/SKILL.md` SHALL also tell Claude that when the checkout the session started in already lies under `.claude/worktrees/`, where Claude Code isolates a subagent's session and refuses every git command aimed at another worktree, in a directory the skills don't add themselves (they name theirs by a change ID, `<YYYYMMDD>-<slug>`, or `archive`, which earlier versions of `build` made), it cuts `spec/<id>` in that checkout instead, with `git switch -c spec/<id> origin/main`, does all later work there, removes no worktree, and switches it off the branch with `git switch --detach origin/main` once the branch has landed, before `clm wrap-up`; and that a session started in one of the skills' own worktrees adds its worktree under the clone's main checkout, the first path `git worktree list` prints, as `git worktree add <main>/.claude/worktrees/<id>`, never nested in the worktree it started in.

#### Scenario: propose with no step for an isolated session
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `git switch -c spec/<id> origin/main`
- WHEN the tests run
- THEN a test fails naming the skill and `git switch -c`

#### Scenario: propose counting its own worktree as isolated
- GIVEN a `skills/propose/SKILL.md` whose isolated-session exception doesn't leave out the worktrees named `<YYYYMMDD>-<slug>` or `archive`, or doesn't add the new worktree under the main checkout that `git worktree list` prints first
- WHEN the tests run
- THEN a test fails naming the skill and `<YYYYMMDD>-<slug>`

#### Scenario: propose switching the shared checkout
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `git worktree add .claude/worktrees/`
- WHEN the tests run
- THEN a test fails naming the skill and `git worktree add`

#### Scenario: propose leaving its worktree behind
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `git worktree remove`
- WHEN the tests run
- THEN a test fails naming the skill and `git worktree remove`


#### Scenario: init without a worktree
- GIVEN a `skills/init/SKILL.md` that doesn't mention `git worktree add`
- WHEN the tests run
- THEN a test fails naming the skill and `git worktree add`



#### Needs
- plugin.build/build-runs-the-loop
- clm/renamed-commands

#### Serves
- capability/work-concurrently
- architecture/land-locally

### Requirement: skills-run-verify <!-- base: b8bb7c23040b05e4 -->
`skills/triage/SKILL.md` SHALL tell Claude, at the end of the run, after its lands and before `clm wrap-up`, to run `clm verify --due --event work-done` and show the human its `verify` line; a `verify failed` line SHALL be shown as a slow test that broke on `main`, which the next triage takes in. It SHALL also tell Claude to run `clm verify --import` just before `clm intake`, and SHALL say, among `clm intake`'s lines, that `verify <sha> new <culprit> <component>...` is a slow test that failed after the land, which no item has taken in: Claude shows them together with no verifier, offering `P1`, and writes each one taken in as a `kind: bug` item whose `verify_failures` holds the commit's full ID (`git rev-parse <sha>`), whose `components` are the line's, and whose body names the culprit land, its change and the verify log, on the same `intake/<YYYYMMDD-HHMM>` branch as the other items taken in, saying that, once the item is on `main`, `clm land` holds code lands on those components until the fix, whose commit carries `Resolves: backlog/<slug>`, lands, and that an item for a failure that proves a flake is deleted on the human's word (`verify-after-land`). It SHALL say that taking a `verify` line in is the human's decision that the failure is real: until then `clm land` only warns.

#### Scenario: the shipped skills
- GIVEN the plugin's `skills/triage/SKILL.md`
- WHEN a test reads it
- THEN it contains `clm verify --due --event work-done`, `clm verify --import`, `verify <sha> new`, `verify_failures` and `verify-after-land`

#### Scenario: verify after the lands
- GIVEN the plugin's `skills/triage/SKILL.md`
- WHEN a test reads it
- THEN `clm verify --due --event work-done` comes before the last `clm wrap-up` in it

#### Needs
- plugin.build/build-runs-the-loop
- verify/verify-due
- verify/verify-import
- orchestrator.triage/intake-verify-failures
- clm/verify-command

#### Serves
- capability/slow-tests-after-land

### Requirement: launch-models <!-- base: 030ff83ec16315b7 -->
Every subagent the plugin's skills tell Claude to launch SHALL have its model named in the Agent tool's `model` parameter (`model-per-launch`), the session's own model being named in the words `the session's own model`, which Claude reads as the alias of the model its system prompt names. `skills/build/SKILL.md` SHALL tell Claude to start the loop with `clm build start --session-model <alias>`, `<alias>` the alias of the session's own model, and to launch each run with the model its `launch` line names, since the loop chooses each run's model (`orchestrator.loop/plan-order`). `skills/triage/SKILL.md` SHALL tell Claude to launch each draft with the session's own model and the fold with `model: "sonnet"`. `skills/propose/SKILL.md`'s builder check, `skills/health/SKILL.md`'s `spec-slimmer` launches and `skills/init/SKILL.md`'s per-node subagents SHALL be launched with `model: "sonnet"`. `skills/propose/SKILL.md`'s and `skills/contribution/SKILL.md`'s `architecture-auditor` launches SHALL name the session's own model, since the audit is a design review. `agents/spec-auditor.md`, `agents/architecture-auditor.md` and `agents/spec-slimmer.md` SHALL set `model: sonnet` in their frontmatter.

#### Scenario: skills and agents
- GIVEN the plugin's skills and agents
- WHEN a test reads them
- THEN `skills/build/SKILL.md` contains `--session-model`; `skills/triage/SKILL.md`, `skills/propose/SKILL.md`, `skills/health/SKILL.md` and `skills/init/SKILL.md` each contain `model: "sonnet"`; `skills/build/SKILL.md`, `skills/triage/SKILL.md`, `skills/propose/SKILL.md` and `skills/contribution/SKILL.md` each contain `the session's own model`; and the three agents' frontmatter has `model: sonnet`

#### Scenario: build launched without a model
- GIVEN a `build` `SKILL.md` that doesn't mention `--session-model`
- WHEN the test's check runs on it
- THEN it reports `--session-model`

#### Serves
- capability/right-model-per-task

#### Needs
- plugin.build/build-runs-the-loop
- orchestrator.run/review-model

### Requirement: unattended-permissions-check <!-- base: e8390775b69b74af -->
`skills/build/SKILL.md` and `skills/triage/SKILL.md` SHALL each tell Claude, once, before the first build or draft it launches, to run `clm init --check-permissions` and, when it exits 1, to tell the human that in auto mode Claude Code may deny the unattended steps those rules allow, show the rules it printed, say that they belong in `permissions.allow` in `.claude/settings.json` (or `.claude/settings.local.json` for this clone alone), and go on launching, never refusing to; when it exits with any other code, as a `clm` without the option does, to tell the human the check couldn't run and go on launching; and never to write either file itself. `skills/build/SKILL.md` SHALL tell Claude to release the claim of a component the human gives up for good with `clm branch delete`, and SHALL NOT contain `git push`.

#### Scenario: skills check the rules
- GIVEN the plugin's `skills/build/SKILL.md` and `skills/triage/SKILL.md`
- WHEN a test reads them
- THEN each contains `clm init --check-permissions`, `auto mode` and `.claude/settings.json`, and the build skill contains `clm branch delete` and doesn't contain `git push`

#### Scenario: skill without the check
- GIVEN a `triage` `SKILL.md` that doesn't mention `clm init --check-permissions`
- WHEN the test's check runs on it
- THEN it reports the skill and `clm init --check-permissions`

#### Serves
- capability/build-from-spec
- capability/triage-unattended

#### Needs
- plugin.build/build-runs-the-loop
- init/init-check-permissions
- host/branch-delete

### Requirement: builder-check <!-- base: 43c9efb5a25f25ab -->
`skills/propose/SKILL.md` SHALL tell Claude to run a builder check with a fresh subagent before opening the spec PR, to list the questions it leaves open under `## Open questions` in the PR body, to record facts and steps only a human can supply as `question` or `human-task` backlog items whose `blocks` name the requirements that wait on them, and to give a requirement that can't be built until another component's pending requirement is built a `#### Needs` block naming it (`requirement-needs`). It SHALL tell Claude that a requirement that removes or renames part of a component's Contract that another component's code uses is such a requirement: it needs the requirements that move each such component off the old contract, and a removal written only as `REMOVED Requirements` can't carry them, so it is stated as an added or modified requirement or split per `expand-contract` (`contract-removal-needs`); that a rename is written as two requirements of the provider in the same change (`rename-adds-then-removes`): an adding one that offers the new name beside the old, needs nothing and leaves every user's code building as it stands (a second name, never a widened type a user matches exhaustively), and a removing one, stated as an added or modified requirement, that needs each user's move, each move needing the adding one, so the needs form no cycle, the adding one's build marking the old name `clm:remove-by` the change, which stays live until the removal is built (`expand-contract`); and to ask the builder-check subagent to search the code of the other components for uses of each part of a Contract the change removes or renames, exhaustive ones (a `match`, a `switch` with no default, a map keyed by the type) included, and for exhaustive uses of each type the change widens. It SHALL tell Claude that a requirement that widens a type another component matches exhaustively (an added enum variant) needs that component's requirements that build and name the code the new value selects there, which need nothing of the widening and are tested by calling that code directly, so the widening's build adds the arm as a forced fix, and that component's requirements that reach the code through the new value need the widening (`forced-fixes-cover-additions`). It SHALL tell Claude that when the removal can't carry that `#### Needs` (the removing component is required and the component that must move is optional, or the removed text is a whole requirement), the contract step goes in a second change on the same branch, left `proposed`, and the branch also adds a `kind: deferred` backlog item for it whose `components` are the contract change's components and whose body names the contract change and each requirement it waits on with the version it must be current at, which the contract change's `resolves` doesn't name, so `clm triage` shows it until the human judges it due (`contract-step-deferred`). It SHALL tell Claude to have the builder name the work a requirement relies on from another component that no requirement of that component states, since a Contract or Design edit alone builds nothing, adding that component's requirement to the change and naming it in the relying requirement's `#### Needs` (`dependency-work-is-a-requirement`). It SHALL tell Claude to ask the builder-check subagent, for each requirement the change adds or modifies, which requirements not yet current it relies on, of any component, among the change's own and those `clm pending` lists, answering `none` for a requirement that relies on none (`builder-check-asks-order`), so a missing order shows as a named pair rather than a question the subagent may not think to ask; and that a requirement relying on a pending requirement of its own component that has a `#### Needs` block carries that block's needs too, since a need may not name its own component and the build's order within the component can't wait on another component's work. The build loop keeps those items in the human's queue, with the requirements they block (`orchestrator.loop/humans-queue`).

#### Scenario: skill without the check
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `## Open questions`
- WHEN the tests run
- THEN a test fails naming the skill

#### Scenario: skill without blocks
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `blocks`
- WHEN the tests run
- THEN a test fails naming the skill and `blocks`

#### Scenario: skill without needs
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `Needs`
- WHEN the tests run
- THEN a test fails naming the skill and `Needs`

#### Scenario: skill without contract removals
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `contract-removal-needs`
- WHEN the tests run
- THEN a test fails naming the skill and `contract-removal-needs`

#### Scenario: skill without the deferred contract step
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `contract-step-deferred`
- WHEN the tests run
- THEN a test fails naming the skill and `contract-step-deferred`

#### Scenario: skill without the rename split
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `rename-adds-then-removes`
- WHEN the tests run
- THEN a test fails naming the skill and `rename-adds-then-removes`

#### Scenario: skill without the widened type
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `forced-fixes-cover-additions`
- WHEN the tests run
- THEN a test fails naming the skill and `forced-fixes-cover-additions`

#### Scenario: skill without dependency work
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `builds nothing`
- WHEN the tests run
- THEN a test fails naming the skill and `builds nothing`

#### Scenario: skill without the order ask
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `builder-check-asks-order`
- WHEN the tests run
- THEN a test fails naming the skill and `builder-check-asks-order`

#### Scenario: skill without same-node needs
- GIVEN a `skills/propose/SKILL.md` that doesn't mention `its own component`
- WHEN the tests run
- THEN a test fails naming the skill and `its own component`

#### Serves
- capability/propose-intent
- capability/keep-a-backlog
- architecture/ask-the-architect

#### Needs
- plugin.build/build-runs-the-loop
- model/parse-backlog

### Requirement: skills-follow-agent-harness <!-- base: bd4508d6f22f458e -->
Every skill under `plugin/skills/` that launches a subagent, cuts a worktree for a build or asks for an approval, except `build`, whose loop reads the agent harness itself and says in each action how to launch (`orchestrator.loop/loop-follows-agent-harness`), SHALL tell the session to run `clm agent-harness` before its first such step and to follow what it prints: where `subagents` is `no`, to do each piece of work a subagent would take in the session itself, one at a time; where `isolated-worktrees` is `no`, to give a build its own worktree with `git worktree add .claude/worktrees/agent-<component>` before launching it, and to tell the build to work there; where `model-choice` is `no`, to launch with the session's own model, the model names the skill gives being Claude Code's; where `confirm` is `no`, to ask the human for their yes in the conversation and then to have them run `clm approve <branch>` in their own terminal, never running it from the session; and, where `named-agents` is `no`, to launch a general subagent told to follow the agent's file under the plugin's `agents/`. A skill SHALL name the Agent tool only beside the words `or the agent harness's own subagent tool`. This component's own skills (`help`, `health`, `configure`, `contribution`, `verify`) SHALL hold it; the children's skills hold it through their own requirements.

#### Scenario: health under Codex
- GIVEN `skills/health/SKILL.md`
- WHEN a test reads it
- THEN it names `clm agent-harness` and `subagents`

#### Scenario: a skill tied to one agent harness
- GIVEN a `SKILL.md` that names the Agent tool without `or the agent harness's own subagent tool`
- WHEN the test's check runs on it
- THEN it reports the skill

#### Needs
- plugin.propose/propose-follows-agent-harness
- orchestrator.loop/loop-follows-agent-harness
- plugin.triage/triage-follows-agent-harness
- plugin.init/init-follows-agent-harness
- clm/agent-harness-command

#### Serves
- capability/work-from-any-agent-harness

### Requirement: skills-push-with-clm <!-- base: 69c5495464f2a9ca -->
No `SKILL.md` under the plugin's `skills/` SHALL contain `git push`. Wherever a skill tells Claude to push a branch to `origin`, attended or not, it SHALL name `clm branch push`, and wherever it tells Claude to delete a branch on `origin`, `clm branch delete`, so that under Claude Code's auto mode every step that writes to `origin` is a `clm` command the project's allow rules cover (`unattended-steps-run-clm`).

#### Scenario: skills push through clm
- GIVEN the plugin's `skills/*/SKILL.md`
- WHEN a test reads each
- THEN none contains `git push`, and `propose`, `contribution` and `triage` each contain `clm branch push`

#### Scenario: skill with a raw push
- GIVEN a `SKILL.md` that tells Claude to push with `git push --force-with-lease`
- WHEN the test's check runs on it
- THEN it reports the skill and `git push`

#### Serves
- capability/build-from-spec
- capability/triage-unattended

#### Needs
- host/branch-push
- host/branch-delete
- plugin.build/build-runs-the-loop

### Requirement: wrap-up-left-lines <!-- base: 969152a0e11b1a70 -->
Each of `skills/triage/SKILL.md`, `skills/propose/SKILL.md`, `skills/contribution/SKILL.md` and `skills/init/SKILL.md` that tells Claude to run `clm wrap-up` SHALL tell Claude to pass each `left <path>: ...` line `clm wrap-up` prints to the human as it stands, a build's worktree it couldn't remove, which doesn't make the wrap-up incomplete, and to leave the directory, and any command the line names, to them. No skill SHALL tell Claude to run `git worktree prune` or delete such a directory itself. `skills/build/SKILL.md` runs no `clm wrap-up`: its loop passes those lines on in its `say` actions (`orchestrator.loop/step-housekeeping`).

#### Needs
- orchestrator.checkpoint/checkpoint
- plugin.build/build-runs-the-loop

#### Scenario: wrap-up's left lines passed on
- GIVEN the plugin's skills
- WHEN a test reads each one that names `clm wrap-up`
- THEN it holds a paragraph that names both `clm wrap-up` and `left <path>`, and no skill holds `git worktree prune`

#### Scenario: skill without the line
- GIVEN a `SKILL.md` that names `clm wrap-up` and no paragraph naming `left <path>` with it
- WHEN the test's check runs on it
- THEN it reports that skill

### Requirement: view-fallback-wording <!-- base: 2ed0320a52812b96 -->
`plugin/commands/view.md`, the text the model receives when no `clm-panel` hooks module answers `/clm:view`, SHALL NOT say that `clm-panel` isn't installed. It SHALL say that the panel either isn't installed or didn't load, and name `/plugin install clm-panel@clm` for the first case and `claude plugin validate` and `claude --debug` for the second, then go on to the same `clm status` and `clm view <name>` fallbacks as before, its names and argument hint being `status`, `queue`, `spec`, `flight` and `blockers`.

#### Scenario: installed but refused
- GIVEN `plugin/commands/view.md`
- WHEN a test reads it
- THEN it doesn't contain `isn't installed here`, and it contains `/plugin install clm-panel@clm`, `claude plugin validate`, `claude --debug` and `queue`

#### Serves
- capability/watch-beside-chat

#### Needs
- view.repo/view-queue
