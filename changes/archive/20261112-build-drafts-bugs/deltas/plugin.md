---
component: plugin
base: { contract: "d556666c6bf89e90", design: "1097a3da520b3cd6" }
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
- `builder-check` and `architecture-audit`: `propose` runs a builder check and, beside it, the `architecture-auditor` before its pull request; `build` lists the `question` and `human-task` items that block what it leaves; `contribution` launches the auditor too;
- `session-worktrees`: `propose`, `init` and `build` cut each branch in a worktree of its own under `.claude/worktrees/`, never switching the checkout the session started in, except in a session already isolated there; `triage` cuts its archive branch the same way, a rule its own requirement states (`plugin.triage/triage-archives-complete`);
- `defect-check-before-pr`: `propose` and `build` review the session for CLM defects before any pull request; `propose` fixes the reported ones once its branches have landed, and `build` drafts them with the backlog's other bugs (`plugin.build/build-drafts-bugs`);
- `recommend-the-right-design`: `triage` and `propose` read the specs and code before recommending, and recommend the correct design over the convenient one;
- `skills-write-serves` and `skills-write-enforcement`: `propose` and `init` give components `serves`, and constraints `#### Enforced by` or `#### Untested`;
- `launch-models`: every subagent a skill launches has its model named;
- `skills-follow-agent-harness`: every skill reads `clm agent-harness` and follows what the agent harness can do, in words no single agent harness owns;
- `skills-push-with-clm`: no skill says `git push`; pushes and remote deletes go through `clm branch push` and `clm branch delete`;
- `unattended-permissions-check` and `skills-run-verify`: `build` and `triage` check the allow rules before their first launch, and run `clm verify --due --event work-done` before `clm wrap-up`;
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

**Shared steps** (`defect-check-before-pr`). Both skills resolve what `clm rebase` leaves them the same way: a `spec changed` report by reading it and `clm pending <component>` and adapting the work, conflicted files by hand, stale deltas by rebasing them on `clm hashes`, check failures by fixing them. Both skills review the session for CLM defects before opening any pull request (`defect-check-before-pr`). Where `clm defect` writes to the backlog, `propose` then **fixes reported defects** once its branch has landed, before `clm wrap-up` and the summary (`defect-reports`), in the branch's worktree switched with `git switch --detach origin/main`; the fix branch's pull request lists the defect as fixed, and the landed pull request that reported it isn't edited. `build` has no such step: the defects its builds report are `kind: bug` items, which its drain drafts with the backlog's other bugs once they reach `origin/main` (`plugin.build/build-drafts-bugs`).

**The method** (`method-in-the-plugin`, `first-graph-teaches-shape`). An adopter's project has none of this repository's `docs/`, `ARCHITECTURE.md` rationale or specs, so the skills are where it learns the method. `clm guide <page>` prints a page from the binary the shim runs, offline and at the plugin's version (`guide-in-the-binary`), so a skill names it and not `docs/guide/...`, a path only this repository has, except the two pages a spec branch edits that `method-pointers` lets `propose` name. A test per skill keeps its pointers, and a test checks that each page a skill names is a file in `docs/guide/`.

Builds use superpowers as `unattended-superpowers` decides; their prompt is `orchestrator.run`'s `clm run`.

**No eval cases** (`cut-evals`). The plugin has no `evals/` directory. The skills' tests read their text, `claude plugin validate plugin` checks that Claude Code loads the plugin (`ci/plugin-validate`), and a skill's behaviour is judged where it is used, in sessions.

## MODIFIED Requirements

### Requirement: defect-check-before-pr <!-- base: 3c314926b5fb51f9 -->
`skills/propose/SKILL.md` and `skills/build/SKILL.md` SHALL tell Claude, before opening any PR, to review the session for CLM defects neither reported nor fixed, report each with `clm defect` unless the session-start rule has it folded into the branch, naming with `--component` the component a fix would lie in, and list every defect of the session under `## CLM defects` in the PR body, each fixed one with its fix. `skills/propose/SKILL.md` SHALL also have a step, **fix reported defects**, for when `clm defect` writes to the backlog: once the session's branch has landed and before `clm wrap-up` and the summary, fix each small defect the session reported whose item is on `origin/main`, one at a time and one round only, with `clm claim <component> --fix backlog/<slug>` as `propose`'s code fixes do, in the branch's worktree and never in the checkout the session started in, unless that checkout is an isolated session's own under `.claude/worktrees/` (`session-worktrees`), where the fix is made in it; fold a defect met during such a fix when it lies in that fix's component and leave it filed otherwise; and when a claim is refused or a fix turns out large, leave the item filed, release a claim it took by deleting its branch on `origin`, and name the item in the summary. `skills/build/SKILL.md` SHALL have no such step, and no paragraph holding `fix reported defects`: its drain drafts the defects its builds report with the backlog's other bugs (`plugin.build/build-drafts-bugs`).

#### Scenario: skill without the check
- GIVEN a `skills/build/SKILL.md` that doesn't mention `## CLM defects`
- WHEN the tests run
- THEN a test fails naming the skill and `## CLM defects`

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
- clm/renamed-commands
- plugin.build/build-drafts-bugs

#### Serves
- capability/report-defects

### Requirement: session-worktrees <!-- base: 6d976f973d41572a -->
`skills/propose/SKILL.md` SHALL tell Claude to cut `spec/<id>` with `git worktree add .claude/worktrees/<id> -b spec/<id> origin/main`, to change into that worktree and do all later work there, never switching the checkout the session started in onto another branch, and, once the branch has landed, to change back to that checkout and remove the worktree with `git worktree remove` before `clm wrap-up`. `skills/init/SKILL.md` SHALL tell Claude to cut its spec branch the same way, in `.claude/worktrees/<YYYYMMDD>-init`, after the bootstrap push, and `skills/build/SKILL.md` to cut its archive branch with `git worktree add` and remove that worktree with `git worktree remove` once it has landed (`session-worktrees`). `skills/propose/SKILL.md` SHALL also tell Claude that when the checkout the session started in already lies under `.claude/worktrees/`, where Claude Code isolates a subagent's session and refuses every git command aimed at another worktree, in a directory the skills don't add themselves (they name theirs by a change ID, `<YYYYMMDD>-<slug>` or `archive`), it cuts `spec/<id>` in that checkout instead, with `git switch -c spec/<id> origin/main`, does all later work there, removes no worktree, and switches it off the branch with `git switch --detach origin/main` once the branch has landed, before `clm wrap-up`; and that a session started in one of the skills' own worktrees adds its worktree under the clone's main checkout, the first path `git worktree list` prints, as `git worktree add <main>/.claude/worktrees/<id>`, never nested in the worktree it started in. `skills/build/SKILL.md` SHALL tell Claude, in its archive step, that a session isolated under `.claude/worktrees/` in that sense cuts its archive branch in the checkout it has instead of in `.claude/worktrees/archive`: it cuts it with `git switch -c impl/archive/<id> origin/main`, removes no worktree, and switches the checkout off the branch with `git switch --detach origin/main` once it has landed, or its push was refused, before deleting a local branch with `git branch -D` and before `clm wrap-up`.

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

#### Scenario: archive in the shared checkout
- GIVEN a `skills/build/SKILL.md` that doesn't mention `git worktree add`
- WHEN the tests run
- THEN a test fails naming the skill and `git worktree add`

#### Scenario: init without a worktree
- GIVEN a `skills/init/SKILL.md` that doesn't mention `git worktree add`
- WHEN the tests run
- THEN a test fails naming the skill and `git worktree add`

#### Scenario: archive worktree left behind
- GIVEN a `skills/build/SKILL.md` that doesn't mention `git worktree remove`
- WHEN the tests run
- THEN a test fails naming the skill and `git worktree remove`

#### Scenario: build archiving with no step for an isolated session
- GIVEN a `skills/build/SKILL.md` that doesn't mention `git switch -c impl/archive/<id> origin/main`
- WHEN the tests run
- THEN a test fails naming the skill and `git switch -c impl/archive/<id>`

#### Needs
- clm/renamed-commands

#### Serves
- capability/work-concurrently
- architecture/land-locally

### Requirement: launch-models <!-- base: f7550b3b6abe5f83 -->
Every subagent the plugin's skills tell Claude to launch SHALL have its model named in the Agent tool's `model` parameter (`model-per-launch`), the session's own model being named in the words `the session's own model`, which Claude reads as the alias of the model its system prompt names. `skills/build/SKILL.md` SHALL tell Claude to run `clm run <component> --review-model <alias>`, `<alias>` the alias of the session's own model, and to launch each build with `model: "sonnet"`, except that a component's build launched again after a build of it stopped without landing, for a reason other than a backlog item or a spec change, runs on the session's own model. `skills/triage/SKILL.md` and `skills/build/SKILL.md` SHALL each tell Claude to launch each draft with the session's own model and the fold with `model: "sonnet"`. `skills/propose/SKILL.md`'s builder check, `skills/health/SKILL.md`'s `spec-slimmer` launches and `skills/init/SKILL.md`'s per-node subagents SHALL be launched with `model: "sonnet"`. `skills/propose/SKILL.md`'s and `skills/contribution/SKILL.md`'s `architecture-auditor` launches SHALL name the session's own model, since the audit is a design review. `agents/spec-auditor.md`, `agents/architecture-auditor.md` and `agents/spec-slimmer.md` SHALL set `model: sonnet` in their frontmatter.

#### Scenario: skills and agents
- GIVEN the plugin's skills and agents
- WHEN a test reads them
- THEN `skills/build/SKILL.md` contains `--review-model` and `model: "sonnet"`; `skills/triage/SKILL.md`, `skills/propose/SKILL.md`, `skills/health/SKILL.md` and `skills/init/SKILL.md` each contain `model: "sonnet"`; `skills/build/SKILL.md`, `skills/triage/SKILL.md`, `skills/propose/SKILL.md` and `skills/contribution/SKILL.md` each contain `the session's own model`; and the three agents' frontmatter has `model: sonnet`

#### Scenario: build launched without a model
- GIVEN an `build` `SKILL.md` that doesn't mention `model: "sonnet"`
- WHEN the test's check runs on it
- THEN it reports `model: "sonnet"`

#### Serves
- capability/right-model-per-task

#### Needs
- orchestrator.run/review-model
- plugin.build/build-drafts-bugs
