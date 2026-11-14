---
component: plugin
base: { frontmatter: "e0746c4e5af11e05", contract: "de10414c63096322", design: "896c4e2549654289" }
---

## MODIFIED Frontmatter

The five skill components are removed and their skill files return to `plugin`.

```yaml
depends_on: [check, context, orchestrator, orchestrator.run, orchestrator.checkpoint, orchestrator.triage, sync, config, view, plugin.hooks, plugin.panel, land, host, plugin.agents, agent-harness]
paths: ["plugin/bin/**", "plugin/hooks/**", "plugin/commands/**", "plugin/.claude-plugin/**", "plugin/.codex-plugin/**", ".claude-plugin/**", ".agents/plugins/**", "plugin/skills/**"]
```

## MODIFIED Contract

The plugin `clm`, one set of files packaged for Claude Code and for Codex (`agent-harness-adapters`), each installed from this repo's marketplace for that agent harness. It provides, on both:
- the `clm` command on the shell tool's PATH;
- `clm defect`, which reports a defect in CLM itself as `config`'s `defects` setting says: as a backlog item in CLM's own repository, or as an issue filed through the host that the human approves (`defect-reports`, `host-adapters`); its rules are `plugin.hooks`';
- hooks that call `clm hook <name> --agent-harness <agent harness>`, whose rules are `plugin.hooks`': to show the repo's state at session start, with the rule for reporting CLM defects, to have the human confirm `clm approve`, to block cutting releases and pushing to `main`, to have the human approve each defect issue, and to autosync spec work at the end of each turn;
- the skills `propose`, `build`, `triage`, `init`, `upgrade`, `contribution`, `help`, `health`, `configure` and `verify` (`skills`);
- the subagents `spec-auditor`, `architecture-auditor`, `spec-slimmer` and `issue-verifier`, which are `plugin.agents`'.

The skills, agents and commands are source files the plugin ships: what a skill tells a session is not a requirement. `launch-models` and `method-pointers` read `plugin.agents`' files too, though this component's build never edits a child's file (`path-ownership`).

The CLM panel, a second plugin `clm-panel` in the Claude Code marketplace, is `plugin.panel`; it is Claude Code's alone.

The plugin's version is the `clm` version it runs, in both packages. Users update with `claude plugin update clm`, or Codex's own plugin update, and the new version's shim downloads its binary on first use.

An agent of the generic tier, which has no package, gets CLM from the `clm` binary and the project's `AGENTS.md` alone (`platform`).

The skills carry CLM's method to every adopter (`method-in-the-plugin`): what they don't say themselves, they point at as a page of the guide that `clm guide <page>` prints.

## MODIFIED Design

The plugin is `plugin/` in this repository: `.claude-plugin/plugin.json` (`clm`, versioned with the binary), `bin/clm` (the shim), `hooks/hooks.json`, the skills and agents the Contract lists, and `commands/view.md`. The four subagents are `plugin.agents`'; this component keeps every other file. The skills, agents and commands are source files the plugin ships, which their component owns as it owns its code: what a skill tells a session is not a requirement. `commands/view.md` makes `/clm:view` exist: with `clm-panel` installed, its hooks module answers the command itself (`plugin.panel`); without it, the file has the model run `clm status` or `clm view <name>` through the Bash tool. The `clm` CLI holds every rule; the plugin only runs it (`plugin-plus-rust-cli`, `hook-logic-in-clm`). The rules behind `clm hook` and `clm defect` are `plugin.hooks`'; the panel is `plugin.panel`.

**The shim** (`binary-from-releases`). `bin/clm` resolves the binary for the plugin's version: `${XDG_CACHE_HOME:-~/.cache}/clm/<version>/clm`, downloading `clm-<target>.tar.gz` from the matching GitHub release and verifying its `.sha256` when it is missing, then runs it with the arguments it was given. `$CLM_BIN` overrides it, for developing against a local build. It is a POSIX `sh` script and uses `curl`, `tar` and `sha256sum` or `shasum`: it downloads both files with `curl -fsSL` from `https://github.com/clunny/clm/releases/download/v<version>/`, the public repository's release, so no `gh`, sign-in or token is needed (`binary-from-releases`). A missing `curl` is a failed download. The guard fails open (`guard-fails-open`): when the binary `$CLM_BIN` names is missing or can't run, or `clm hook guard` crashes, the hook exits with neither 0 nor 2, which Claude Code doesn't treat as a block, and the shim adds no wrapper that exits 2, since a session whose every `Bash` call is blocked can't rebuild the binary that would unblock it.

**Hooks** (`hooks-wired`). What each hook does with Claude Code's event is `plugin.hooks`.

**Skills.** The skills follow the repo's change flow (`CLAUDE.md`). A spec branch either needs review or lands on checks, and nothing is reviewed after it lands (`yes-no-review`). There are no labels and no `gh` calls in the skills: pull requests are opened with `clm pr open` and bodies set with `clm pr body` (`host-adapters`).

**A worktree per session branch** (`session-worktrees`). A skill that cuts a branch in the human's session never switches the checkout the session started in, which other sessions on the same clone share.

**The method** (`method-in-the-plugin`, `first-graph-teaches-shape`). An adopter's project has none of this repository's `docs/`, `ARCHITECTURE.md` rationale or specs, so the skills are where it learns the method. `clm guide <page>` prints a page from the binary the shim runs, offline and at the plugin's version (`guide-in-the-binary`), so a skill names it and not `docs/guide/...`, a path only this repository has. A test checks that each page a skill or agent names is one `clm guide` prints (`method-pointers`).

Builds use superpowers as `unattended-superpowers` decides; their prompt is `orchestrator.run`'s `clm run`.

**No eval cases** (`cut-evals`). The plugin has no `evals/` directory. `claude plugin validate plugin` checks that Claude Code loads the plugin (`ci/plugin-validate`), and a skill's behaviour is judged where it is used, in sessions.

## MODIFIED Requirements

### Requirement: skills <!-- base: c316b655141a2635 -->
The plugin SHALL ship exactly these skills, each as `skills/<name>/SKILL.md` with a non-empty `description` in its frontmatter: `build`, `configure`, `contribution`, `health`, `help`, `init`, `propose`, `triage`, `upgrade` and `verify`. It SHALL hold no other directory under `skills/`.

#### Scenario: skill without a description
- GIVEN a plugin whose `skills/health/` is missing, or whose `skills/health/SKILL.md` has no `description` in its frontmatter
- WHEN the tests run
- THEN a test fails naming the skill

#### Scenario: an extra directory
- GIVEN a plugin with a directory `skills/implement/`
- WHEN the tests run
- THEN a test fails naming `implement`

#### Serves
- capability/propose-intent
- capability/build-from-spec
- architecture/land-locally
- architecture/approval-policy

### Requirement: method-pointers <!-- base: 1ae4a99670fc6815 -->
Every `clm guide <page>` a skill or agent names, `<page>` being a lowercase name of letters, digits and hyphens, SHALL be a page `clm guide` prints: a file `docs/guide/<page>.md`, `docs/spec-format.md` for `spec-format`, `docs/guide/README.md` for `index`, or the generated page `constraints` (`init/guide-constraints-page`).

#### Needs
- init/guide-command
- init/guide-constraints-page

#### Scenario: a page that doesn't exist
- GIVEN a skill that names `clm guide no-such-page`
- WHEN the tests run
- THEN a test fails naming the skill and `no-such-page`

### Requirement: launch-models <!-- base: b048204d63b949c8 -->
`agents/spec-auditor.md`, `agents/architecture-auditor.md` and `agents/spec-slimmer.md` SHALL set `model: sonnet` in their frontmatter.

#### Scenario: the agents
- GIVEN the plugin's agents
- WHEN a test reads them
- THEN the three agents' frontmatter has `model: sonnet`

#### Serves
- capability/right-model-per-task

## REMOVED Requirements

### Requirement: defect-check-before-pr <!-- base: 246594cddb3404c2 -->
### Requirement: session-worktrees <!-- base: 36e7721a28b80b1d -->
### Requirement: view-fallback-wording <!-- base: fc0defe8ac934adb -->
### Requirement: health-skill <!-- base: 71b58876db72a1ab -->
### Requirement: skills-write-serves <!-- base: 44502341711784c3 -->
### Requirement: skills-write-enforcement <!-- base: 001e455c9edd112a -->
### Requirement: recommend-the-right-design <!-- base: df33f4b268a74acd -->
### Requirement: renamed-commands <!-- base: bbfd9d22d68f4ff8 -->
### Requirement: says-architectural-constraint <!-- base: c7ceabe6a896fbfe -->
### Requirement: skills-say-new-words <!-- base: 8eeb6251732c7c42 -->
### Requirement: skills-say-component <!-- base: 092ab3ef28962cb2 -->
### Requirement: contribution-skill <!-- base: c27fbc2344564da5 -->
### Requirement: architecture-audit <!-- base: 504aa12bf869b600 -->
### Requirement: help-skill <!-- base: bc92d819611f2b2f -->
### Requirement: skills-push-with-clm <!-- base: 76ab4ceb2c63f2ab -->
### Requirement: unattended-permissions-check <!-- base: 2663670ef8809b45 -->
### Requirement: wrap-up-left-lines <!-- base: 655a7df10d28c07b -->
### Requirement: skills-run-verify <!-- base: 0f5313c096b6ad86 -->
### Requirement: configure-skill <!-- base: 0cbec06f589bafcf -->
### Requirement: verify-skill <!-- base: 26e884db179743be -->
### Requirement: skills-follow-agent-harness <!-- base: 27ba8027b33b5b6f -->
