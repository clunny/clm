---
component: plugin
base: { frontmatter: "6ff9475e5f330216", contract: "a6cf2e4060450357" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [check, context, orchestrator, orchestrator.run, orchestrator.checkpoint, orchestrator.triage, sync, config, view, plugin.hooks, plugin.panel, land, host, plugin.propose, plugin.triage, plugin.build, plugin.init, plugin.upgrade, plugin.agents, harness]
paths: ["plugin/bin/**", "plugin/hooks/**", "plugin/commands/**", "plugin/.claude-plugin/**", "plugin/.codex-plugin/**", ".claude-plugin/**", ".agents/plugins/**", "plugin/skills/help/**", "plugin/skills/health/**", "plugin/skills/configure/**", "plugin/skills/contribution/**", "plugin/skills/verify/**"]
serves: [architecture/method-in-the-plugin, capability/work-from-any-harness]
```

## MODIFIED Contract

The plugin `clm`, one set of files packaged for Claude Code and for Codex (`harness-adapters`), each installed from this repo's marketplace for that harness. It provides, on both:
- the `clm` command on the shell tool's PATH;
- `clm defect`, which reports a defect in CLM itself as `config`'s `defects` setting says: as a backlog item in CLM's own repository, or as an issue filed through the host that the human approves (`defect-reports`, `host-adapters`); its rules are `plugin.hooks`';
- hooks that call `clm hook <name> --harness <harness>`, whose rules are `plugin.hooks`': to show the repo's state at session start, with the rule for reporting CLM defects, to have the human confirm `clm approve`, to block cutting releases and pushing to `main`, to have the human approve each defect issue, and to autosync spec work at the end of each turn;
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
- `defect-check-before-pr`: `propose` and `build` review the session for CLM defects before any pull request, and fix the reported ones once their branches have landed;
- `recommend-the-right-design`: `triage` and `propose` read the specs and code before recommending, and recommend the correct design over the convenient one;
- `skills-write-serves` and `skills-write-enforcement`: `propose` and `init` give components `serves`, and constraints `#### Enforced by` or `#### Untested`;
- `launch-models`: every subagent a skill launches has its model named;
- `skills-follow-harness`: every skill reads `clm harness` and follows what the harness can do, in words no single harness owns;
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

## ADDED Requirements

### Requirement: codex-package
The repository SHALL hold a Codex plugin marketplace, in the file and format Codex's plugin documentation gives for a repository's marketplace, listing a plugin `clm` with source `./plugin`, and `plugin/.codex-plugin/plugin.json` SHALL name it `clm` with a `version` equal to the cargo workspace version, pointing Codex at the same `plugin/skills/` the Claude Code package ships and at `plugin/hooks/codex.json` for its hooks. Both packages SHALL run the same `plugin/bin/clm` shim.

#### Scenario: version bump
- GIVEN a commit that changes the workspace version but not `plugin/.codex-plugin/plugin.json`
- WHEN the tests run
- THEN a test fails naming both versions

#### Scenario: one set of skills
- GIVEN the two manifests
- WHEN a test reads them
- THEN both resolve their skills to `plugin/skills/`

#### Serves
- capability/install-and-release
- capability/work-from-any-harness

### Requirement: codex-hooks-wired
`plugin/hooks/codex.json` SHALL run, through the plugin's `bin/clm` named as Codex's hook documentation names a plugin's own files, `clm hook session-start --harness codex` at a session's start, `clm hook guard --harness codex` before each shell command, and `clm hook autosync --harness codex` at the end of each turn, in Codex's hook configuration format. `plugin/hooks/hooks.json` SHALL keep wiring Claude Code's events as `hooks-wired` says.

#### Scenario: guard wired for Codex
- GIVEN `plugin/hooks/codex.json`
- WHEN it is parsed
- THEN its pre-tool hook for the shell tool runs a command ending in `bin/clm" hook guard --harness codex`

#### Needs
- plugin.hooks.guard/guard-per-harness
- plugin.hooks/hooks-per-harness

#### Serves
- capability/guard-sessions
- capability/work-from-any-harness
- architecture/guard-blocks-push-to-main

### Requirement: skills-follow-harness
Every skill under `plugin/skills/` that launches a subagent, cuts a worktree for a build or asks for an approval SHALL tell the session to run `clm harness` before its first such step and to follow what it prints: where `subagents` is `no`, to do each piece of work a subagent would take in the session itself, one at a time; where `isolated-worktrees` is `no`, to give a build its own worktree with `git worktree add .claude/worktrees/agent-<component>` before launching it, and to tell the build to work there; where `model-choice` is `no`, to launch with the session's own model, the model names the skill gives being Claude Code's; where `confirm` is `no`, to ask the human for their yes in the conversation and then to have them run `clm approve <branch>` in their own terminal, never running it from the session; and, where the harness doesn't load the plugin's agents by name, to launch a general subagent told to follow the agent's file under the plugin's `agents/`. A skill SHALL name the Agent tool only beside the words `or the harness's own subagent tool`. This component's own skills (`help`, `health`, `configure`, `contribution`, `verify`) SHALL hold it; the children's skills hold it through their own requirements.

#### Scenario: health under Codex
- GIVEN `skills/health/SKILL.md`
- WHEN a test reads it
- THEN it names `clm harness` and `subagents`

#### Scenario: a skill tied to one harness
- GIVEN a `SKILL.md` that names the Agent tool without `or the harness's own subagent tool`
- WHEN the test's check runs on it
- THEN it reports the skill

#### Needs
- plugin.propose/propose-follows-harness
- plugin.build/build-follows-harness
- plugin.triage/triage-follows-harness
- plugin.init/init-follows-harness
- clm/harness-command

#### Serves
- capability/work-from-any-harness
