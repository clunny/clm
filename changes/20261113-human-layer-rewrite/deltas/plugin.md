---
component: plugin
---

## MODIFIED Requirements

### Requirement: binary-shim <!-- base: 38609dbc2804e462 -->
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

### Requirement: hooks-wired <!-- base: fe0e2818fb2317cd -->
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

### Requirement: health-skill <!-- base: 71b5887660b6af26 -->
The plugin SHALL ship `skills/health/SKILL.md` with a `description` in its frontmatter. It SHALL tell Claude to run `clm health`, to summarise each section for the human, naming the reconsider candidates with their scores and what each signal means, to launch the `spec-slimmer` subagent for each component a `design-heavy`, `history-words`, `outlier-size` or `unexercised` line names, to list the cuts it proposes, and to end by pointing at `/clm:propose` for acting on any of it and at `clm guide using-clm` for what the signals mean. It SHALL tell Claude to edit no spec, file no backlog item and land nothing from this skill.

#### Scenario: skill without the command
- GIVEN a `skills/health/SKILL.md` that doesn't mention `clm health`
- WHEN the tests run
- THEN a test fails naming the skill and `clm health`

#### Scenario: skill without the slimmer
- GIVEN a `skills/health/SKILL.md` that doesn't mention `spec-slimmer`
- WHEN the tests run
- THEN a test fails naming the skill and `spec-slimmer`

#### Scenario: skill that acts
- GIVEN a `skills/health/SKILL.md` that names `clm apply`
- WHEN the tests run
- THEN a test fails naming the skill and `clm apply`

#### Serves
- capability/know-health

### Requirement: codex-hooks-wired <!-- base: 1b8616249f016e8e -->
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
