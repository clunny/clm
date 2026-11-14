---
title: Plugin
status: active
owners: ["@clunny"]
depends_on: [check, context, orchestrator, orchestrator.run, orchestrator.checkpoint, orchestrator.triage, sync, config, view, plugin.hooks, plugin.panel, land, host, plugin.agents, agent-harness]
paths: ["plugin/bin/**", "plugin/hooks/**", "plugin/commands/**", "plugin/.claude-plugin/**", "plugin/.codex-plugin/**", ".claude-plugin/**", ".agents/plugins/**", "plugin/skills/**"]
successor: ""
serves: [architecture/method-in-the-plugin, capability/work-from-any-agent-harness]
---

## Contract
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

## Design
The plugin is `plugin/` in this repository: `.claude-plugin/plugin.json` (`clm`, versioned with the binary), `bin/clm` (the shim), `hooks/hooks.json`, the skills and agents the Contract lists, and `commands/view.md`. The four subagents are `plugin.agents`'; this component keeps every other file. The skills, agents and commands are source files the plugin ships, which their component owns as it owns its code: what a skill tells a session is not a requirement. `commands/view.md` makes `/clm:view` exist: with `clm-panel` installed, its hooks module answers the command itself (`plugin.panel`); without it, the file has the model run `clm status` or `clm view <name>` through the Bash tool. The `clm` CLI holds every rule; the plugin only runs it (`plugin-plus-rust-cli`, `hook-logic-in-clm`). The rules behind `clm hook` and `clm defect` are `plugin.hooks`'; the panel is `plugin.panel`.

**The shim** (`binary-from-releases`). `bin/clm` resolves the binary for the plugin's version: `${XDG_CACHE_HOME:-~/.cache}/clm/<version>/clm`, downloading `clm-<target>.tar.gz` from the matching GitHub release and verifying its `.sha256` when it is missing, then runs it with the arguments it was given. `$CLM_BIN` overrides it, for developing against a local build. It is a POSIX `sh` script and uses `curl`, `tar` and `sha256sum` or `shasum`: it downloads both files with `curl -fsSL` from `https://github.com/clunny/clm/releases/download/v<version>/`, the public repository's release, so no `gh`, sign-in or token is needed (`binary-from-releases`). A missing `curl` is a failed download. The guard fails open (`guard-fails-open`): when the binary `$CLM_BIN` names is missing or can't run, or `clm hook guard` crashes, the hook exits with neither 0 nor 2, which Claude Code doesn't treat as a block, and the shim adds no wrapper that exits 2, since a session whose every `Bash` call is blocked can't rebuild the binary that would unblock it.

**Hooks** (`hooks-wired`). What each hook does with Claude Code's event is `plugin.hooks`.

**Skills.** The skills follow the repo's change flow (`CLAUDE.md`). A spec branch either needs review or lands on checks, and nothing is reviewed after it lands (`yes-no-review`). There are no labels and no `gh` calls in the skills: pull requests are opened with `clm pr open` and bodies set with `clm pr body` (`host-adapters`).

**A worktree per session branch** (`session-worktrees`). A skill that cuts a branch in the human's session never switches the checkout the session started in, which other sessions on the same clone share.

**The method** (`method-in-the-plugin`, `first-graph-teaches-shape`). An adopter's project has none of this repository's `docs/`, `ARCHITECTURE.md` rationale or specs, so the skills are where it learns the method. `clm guide <page>` prints a page from the binary the shim runs, offline and at the plugin's version (`guide-in-the-binary`), so a skill names it and not `docs/guide/...`, a path only this repository has. A test checks that each page a skill or agent names is one `clm guide` prints (`method-pointers`).

Builds use superpowers as `unattended-superpowers` decides; their prompt is `orchestrator.run`'s `clm run`.

**No eval cases** (`cut-evals`). The plugin has no `evals/` directory. `claude plugin validate plugin` checks that Claude Code loads the plugin (`ci/plugin-validate`), and a skill's behaviour is judged where it is used, in sessions.

## Requirements

### Requirement: package
The repo SHALL have `.claude-plugin/marketplace.json` listing a plugin `clm` with source `./plugin`, and `plugin/.claude-plugin/plugin.json` SHALL name it `clm` with a `version` equal to the cargo workspace version.

#### Scenario: version bump
- GIVEN a commit that changes the workspace version but not `plugin.json`
- WHEN the tests run
- THEN a test fails naming both versions

#### Serves
- capability/install-and-release

### Requirement: binary-shim
`plugin/bin/clm` SHALL exec `$CLM_BIN` with its arguments when set. Otherwise it SHALL exec the cached binary for the plugin's version, downloading it first with `curl` from the public repository's GitHub release for that version when absent, with no `gh` and no sign-in. It SHALL verify the download against its `.sha256` before caching it. When no binary can be had it SHALL cache nothing, and:
- for `hook session-start`, print a line saying the CLM state and the guard on `clm approve` are unavailable, with the `cargo install` command for that version, and exit 0; the line SHALL NOT name `clm review` or `spec-approved`;
- for any other `hook` command, print nothing and exit 0;
- otherwise, exit 1 printing the `cargo install` command for that version.

#### Scenario: first use
- GIVEN an empty cache, no `gh` on the `PATH`, and a `curl` that serves a release asset and a matching checksum
- WHEN `bin/clm status` runs twice
- THEN `curl` is asked for `https://github.com/clunny/clm/releases/download/v<plugin version>/` and the asset and its `.sha256` once, and both runs exec the cached binary with `status`

#### Scenario: bad checksum
- GIVEN a `curl` that serves an asset whose checksum doesn't match
- WHEN `bin/clm` runs
- THEN it exits 1 and the cache holds no binary

#### Scenario: guard without a binary
- GIVEN an empty cache and a `curl` that fails
- WHEN `bin/clm hook guard` runs
- THEN it prints nothing, exits 0, and the cache holds no binary

#### Scenario: session start without a binary
- GIVEN an empty cache and a `curl` that fails
- WHEN `bin/clm hook session-start` runs
- THEN it exits 0, and its output says the guard is unavailable, names `clm approve` and `cargo install`, and doesn't name `clm review` or `spec-approved`

#### Serves
- capability/install-and-release
- capability/guard-sessions

### Requirement: hooks-wired
`plugin/hooks/hooks.json` SHALL run `clm hook session-start` on `SessionStart`, `clm hook guard` on `PreToolUse` for `Bash`, and `clm hook autosync` on `Stop`, all through `${CLAUDE_PLUGIN_ROOT}/bin/clm`.

#### Scenario: guard wired
- GIVEN the plugin's `hooks.json`
- WHEN it is parsed
- THEN `PreToolUse` has a `Bash` matcher whose command ends in `bin/clm" hook guard`

#### Scenario: autosync wired
- GIVEN the plugin's `hooks.json`
- WHEN it is parsed
- THEN `Stop` has a command that ends in `bin/clm" hook autosync`

#### Serves
- capability/guard-sessions
- architecture/land-locally

### Requirement: skills
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

### Requirement: shim-targets
`plugin/bin/clm` SHALL download `clm-x86_64-unknown-linux-musl.tar.gz` on `Linux x86_64`, `clm-aarch64-unknown-linux-musl.tar.gz` on `Linux aarch64`, `clm-x86_64-apple-darwin.tar.gz` on `Darwin x86_64` and `clm-aarch64-apple-darwin.tar.gz` on `Darwin arm64`, and map no other platform. On any other platform, Windows's Git Bash and MSYS2 shells included (`no-windows-support`), it SHALL download nothing and fail as `binary-shim` describes, saying there's no prebuilt `clm` for that platform. Each target it maps SHALL be written as one `target=<target>` assignment, which `ci/release`'s prepare step reads. Its text SHALL name no Windows target and no `clm.exe`.

#### Scenario: Linux arm64
- GIVEN an empty cache, a `uname` that reports `Linux` and `aarch64`, and a `curl` that serves release assets
- WHEN `bin/clm status` runs
- THEN `curl` is asked for `clm-aarch64-unknown-linux-musl.tar.gz`

#### Scenario: unsupported platform
- GIVEN an empty cache and a `uname` that reports `FreeBSD` and `amd64`
- WHEN `bin/clm status` runs
- THEN `curl` is not called, and it exits 1 saying there's no prebuilt `clm` for that platform

#### Scenario: Windows unsupported
- GIVEN an empty cache, and a `uname` that reports `MINGW64_NT-10.0` and `x86_64`, and in a second run `UCRT64_NT-10.0-22631` and `x86_64`
- WHEN `bin/clm status` runs
- THEN `curl` is not called either time, and each run exits 1 saying there's no prebuilt `clm` for that platform

#### Scenario: no Windows target in the shim
- GIVEN the text of `plugin/bin/clm`
- WHEN the tests run
- THEN its `target=` assignments are exactly the four targets above, and the test fails when the text, comments included, names `windows` in any case or `clm.exe`

#### Serves
- capability/install-and-release

#### Needs
- ci/release

### Requirement: method-pointers
Every `clm guide <page>` a skill or agent names, `<page>` being a lowercase name of letters, digits and hyphens, SHALL be a page `clm guide` prints: a file `docs/guide/<page>.md`, `docs/spec-format.md` for `spec-format`, `docs/guide/README.md` for `index`, or the generated page `constraints` (`init/guide-constraints-page`).

#### Needs
- init/guide-command
- init/guide-constraints-page

#### Scenario: a page that doesn't exist
- GIVEN a skill that names `clm guide no-such-page`
- WHEN the tests run
- THEN a test fails naming the skill and `no-such-page`

### Requirement: launch-models
`agents/spec-auditor.md`, `agents/architecture-auditor.md` and `agents/spec-slimmer.md` SHALL set `model: sonnet` in their frontmatter.

#### Scenario: the agents
- GIVEN the plugin's agents
- WHEN a test reads them
- THEN the three agents' frontmatter has `model: sonnet`

#### Serves
- capability/right-model-per-task

### Requirement: manifest-owner
`plugin/.claude-plugin/plugin.json` SHALL name `Christopher Lunny` as its `author` and `MIT` as its `license`, and `.claude-plugin/marketplace.json` SHALL name `Christopher Lunny` as its `owner`.

#### Scenario: the owner's full name
- GIVEN the repository
- WHEN a test reads `plugin/.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json`
- THEN the plugin's `author.name` is `Christopher Lunny` and its `license` is `MIT`, and the marketplace's `owner.name` is `Christopher Lunny`

#### Serves
- capability/install-and-release

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
- capability/work-from-any-agent-harness

### Requirement: codex-hooks-wired
`plugin/hooks/codex.json` SHALL run, through the plugin's `bin/clm` named as Codex's hook documentation names a plugin's own files, `clm hook session-start --agent-harness codex` at a session's start, `clm hook guard --agent-harness codex` before each shell command, and `clm hook autosync --agent-harness codex` at the end of each turn, in Codex's hook configuration format. `plugin/hooks/hooks.json` SHALL keep wiring Claude Code's events as `hooks-wired` says.

#### Scenario: guard wired for Codex
- GIVEN `plugin/hooks/codex.json`
- WHEN it is parsed
- THEN its pre-tool hook for the shell tool runs a command ending in `bin/clm" hook guard --agent-harness codex`

#### Needs
- plugin.hooks.guard/guard-per-agent-harness
- plugin.hooks/hooks-per-agent-harness

#### Serves
- capability/guard-sessions
- capability/work-from-any-agent-harness
- architecture/land-locally
