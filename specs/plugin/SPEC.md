---
title: Plugin
status: active
owners: ["@clunny"]
depends_on: [check, context, orchestrator, orchestrator.run, orchestrator.checkpoint, orchestrator.triage, sync, config, view, plugin.hooks, plugin.panel, land, host, plugin.agents, agent-harness]
paths: ["plugin/bin/**", "plugin/hooks/**", "plugin/commands/**", "plugin/.claude-plugin/**", "plugin/.codex-plugin/**", ".claude-plugin/**", ".agents/plugins/**", "plugin/skills/**", "crates/clm/tests/plugin_suite/**"]
successor: ""
serves: [architecture/method-in-the-plugin, capability/work-from-any-agent-harness]
---

## Contract
The plugin `clm`, one set of files packaged for Claude Code and for Codex (`agent-harness-adapters`), each installed from this repo's marketplace for that agent harness. It provides, on both:
- the `clm` command on the shell tool's PATH;
- `clm defect`, which reports a defect in CLM itself as `config`'s `defects` setting says: as a backlog item in CLM's own repository, or as an issue filed through the host that the human approves (`defect-reports`, `host-adapters`); its rules are `plugin.hooks`';
- hooks that call `clm hook <name>`, whose rules are `plugin.hooks`': Codex's pass `--agent-harness codex`, and Claude Code's pass no `--agent-harness`, taking the default `claude-code`. They show the repo's state at session start, with the rule for reporting CLM defects, have the human confirm `clm approve`, block cutting releases and pushing to `main`, have the human approve each defect issue, and autosync spec work at the end of each turn;
- the skills `propose`, `build`, `triage`, `init`, `upgrade`, `contribution`, `help`, `health`, `configure` and `verify` (`skills`);
- the command `/clm:view` (`view-command`), which `plugin.panel` answers when the panel is installed;
- the subagents `spec-auditor`, `architecture-auditor`, `spec-slimmer` and `issue-verifier`, which are `plugin.agents`'.

The skills and commands are source files the plugin ships: what a skill or command tells a session is not a requirement (`prompt-text-is-source`).

The CLM panel, a second plugin `clm-panel` in the Claude Code marketplace, is `plugin.panel`; it is Claude Code's alone.

The plugin's version is the `clm` version it runs, in both packages. Users update with `claude plugin update clm`, or Codex's own plugin update, and the new version's shim downloads its binary on first use.

An agent of the generic tier, which has no package, gets CLM from the `clm` binary and the project's `AGENTS.md` alone (`platform`).

The skills carry CLM's method to every adopter (`method-in-the-plugin`): what they don't say themselves, they point at as a page of the guide that `clm guide <page>` prints.

## Design
The plugin is `plugin/` in this repository: `.claude-plugin/plugin.json` (`clm`, versioned with the binary), `.codex-plugin/plugin.json` (the same plugin for Codex), `bin/clm` (the shim), `hooks/hooks.json` and `hooks/codex.json`, the skills and agents the Contract lists, and `commands/view.md`; with the two marketplaces at the repository's root, `.claude-plugin/marketplace.json` and `.agents/plugins/marketplace.json`. The four subagents are `plugin.agents`'; this component keeps every other file. The skills and commands are source files the plugin ships, which this component owns as it owns its code: what a skill tells a session is not a requirement. `commands/view.md` makes `/clm:view` exist: with `clm-panel` installed, its hooks module answers the command itself (`plugin.panel`); without it, the file has the model run `clm status` or `clm view <name>` through the Bash tool. The `clm` CLI holds every rule; the plugin only runs it (`plugin-plus-rust-cli`, `hook-logic-in-clm`). The rules behind `clm hook` and `clm defect` are `plugin.hooks`'; the panel is `plugin.panel`.

**The shim** (`binary-from-releases`). `bin/clm` resolves the binary for the plugin's version, read from `.claude-plugin/plugin.json` in both packages, whose versions are equal (`package`, `codex-package`): `${XDG_CACHE_HOME:-$HOME/.cache}/clm/<version>/clm`, downloading `clm-<target>.tar.gz` from the matching GitHub release and verifying its `.sha256` when it is missing, then runs it with the arguments it was given. It reads the version from the first line of that file whose key is `"version"`, the layout `ci/release`'s bump writes, so a one-line `plugin.json` reads as having no version. `$CLM_BIN` overrides it, for developing against a local build. It is a POSIX `sh` script under `set -u` and uses `curl`, `tar` and `sha256sum` or `shasum`: it downloads both files with `curl -fsSL` from `https://github.com/clunny/clm/releases/download/v<version>/`, the public repository's release, so no `gh`, sign-in or token is needed (`binary-from-releases`). It downloads into a temporary directory inside the version's cache directory and moves the binary into place last, so an interrupted or failed download never leaves a bad binary. The time limits keep a stalled network from holding the session-start or guard hook, which have no `timeout` in the hooks files, and a hook's failed download marks the cache with the time it failed, so the hooks of the next hour don't each wait on the network again; a hook that skips the download leaves the mark as it is, or a busy session's guard would push the hour on for ever. The commands the shim runs before the download, its writes to the cache and its removals of the download's files send their errors nowhere, a mark it can't refresh is removed and written anew so the hour holds, and a hook's failed write to the cache changes nothing of its exit: under dash a failed redirection on `:` would end the script with 2, so the mark is written with `touch`. Every expansion of an unset variable is guarded, since `set -u` would otherwise end the script with a status Claude Code reads as a block. The guard fails open (`guard-fails-open`): the shim adds no wrapper around `$CLM_BIN` that could exit 2: a missing file ends the hook with 127 and one that isn't executable with 126, and what a binary that runs returns is its own; the shim adds none since a session whose every `Bash` call is blocked can't rebuild the binary that would unblock it.

**Hooks** (`hooks-wired`, `codex-hooks-wired`). What each hook does with its agent harness's event is `plugin.hooks`'.

**Skills.** The skills follow the repo's change flow (`CLAUDE.md`). A spec branch either needs review or lands on checks, and nothing is reviewed after it lands (`yes-no-review`). There are no labels and no `gh` calls in the skills: pull requests are opened with `clm pr open` and bodies set with `clm pr body` (`host-adapters`).

**A worktree per session branch** (`session-worktrees`). A skill that cuts a branch in the human's session never switches the checkout the session started in, which other sessions on the same clone share.

**The method** (`method-in-the-plugin`, `first-graph-teaches-shape`). An adopter's project has none of this repository's `docs/`, `ARCHITECTURE.md` rationale or specs, so the skills are where it learns the method. `clm guide <page>` prints a page from the binary the shim runs, offline and at the plugin's version (`guide-in-the-binary`), so a skill names it and not `docs/guide/...`, a path only this repository has.

Builds use superpowers as `unattended-superpowers` decides; their prompt is `orchestrator.run`'s `clm run`.

**No eval cases** (`cut-evals`). The plugin has no `evals/` directory. `claude plugin validate plugin` checks that Claude Code loads the plugin (`ci/plugin-validate`), and a skill's behaviour is judged where it is used, in sessions.

## Requirements

### Requirement: package
The repo SHALL have `.claude-plugin/marketplace.json` listing a plugin `clm` with source `./plugin`, and `plugin/.claude-plugin/plugin.json` SHALL name it `clm` with a `version` equal to the cargo workspace version. `.claude-plugin/marketplace.json` SHALL list, after `clm`, a plugin `clm-panel` with source `./plugin-panel`, and no other plugin.

#### Scenario: version bump
- GIVEN a commit that changes the workspace version
- WHEN `plugin/.claude-plugin/plugin.json` is read at that commit
- THEN its `version` is the workspace's new version

#### Scenario: two plugins
- GIVEN the repository
- WHEN Claude Code reads `.claude-plugin/marketplace.json`
- THEN its `plugins` are `clm` with source `./plugin`, then `clm-panel` with source `./plugin-panel`, and no other

#### Serves
- capability/install-and-release

### Requirement: binary-shim
`plugin/bin/clm` SHALL exec `$CLM_BIN` with its arguments when `CLM_BIN` is set and not empty, adding nothing around it, so its exit status is that binary's when it runs, 127 when `$CLM_BIN` names no file, and 126 when the file it names isn't executable. Otherwise it SHALL exec the cached binary for the plugin's version, the `version` in the `.claude-plugin/plugin.json` beside its `bin/`, at `$XDG_CACHE_HOME/clm/<version>/clm`, or at `$HOME/.cache/clm/<version>/clm` when `XDG_CACHE_HOME` is unset or empty, downloading it first with `curl` from the public repository's GitHub release for that version when absent, with no `gh` and no sign-in, each `curl` call given a 10 second limit to connect and a 120 second limit in all. It SHALL verify the download against its `.sha256` before caching it. When it can read no version, or `XDG_CACHE_HOME` and `HOME` are each unset or empty, it SHALL download nothing. When no binary can be had it SHALL cache nothing, leaving in the version's cache directory neither a binary nor any file of the download, and:
- for `hook session-start`, print on stdout one line saying the CLM state and the guard on `clm approve` are unavailable, with the `cargo install` command for that version, its tag `v<version>` as written when no version can be read, and exit 0; the line SHALL NOT name `clm review` or `spec-approved`;
- for any other `hook` command, print nothing on stdout or stderr and exit 0;
- otherwise, exit 1 printing on stderr why, the `cargo install` command for that version, and that `CLM_BIN` is to be set to the installed binary's path.

When the download of a `hook` command fails, each `hook` command within the hour after that failure SHALL download nothing and act as when no binary can be had, while any other command SHALL still download; a hook that so downloads nothing SHALL leave that hour's start where it was, so the first hook after the hour downloads again. When no binary can be had, a `hook` command that can't write to the version's cache directory, or to what it keeps there, SHALL still exit 0, printing nothing for `hook guard` and only its line for `hook session-start`.

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

#### Scenario: CLM_BIN
- GIVEN `CLM_BIN` naming, in turn: an executable that records its arguments; an executable that exits 101; a path where no file exists; a file that isn't executable; and, last, `CLM_BIN` set to the empty string with an empty cache and a `curl` that serves a release
- WHEN `bin/clm` runs with `next --expiry 2`, and with `hook guard` for the missing and the non-executable file
- THEN the first runs that executable with `next --expiry 2` and calls no `curl`; the second exits 101; the missing file exits 127 and the non-executable one 126; and the empty `CLM_BIN` downloads and runs the cached binary, as when `CLM_BIN` is unset

#### Scenario: a hook after a failed download
- GIVEN an empty cache and a `curl` that fails, and a `bin/clm hook guard` whose download failed 59 minutes ago
- WHEN `bin/clm hook guard` and `bin/clm hook session-start` run; then `bin/clm status`; then the failure is made 2 minutes older than the shim then records it, and `bin/clm hook guard` runs again
- THEN the first two call no `curl`, exit 0, and `hook session-start` still prints its line; `status` calls `curl`; and the last `hook guard` calls `curl`

#### Scenario: a cache it can't write
- GIVEN a `curl` that fails, and in turn a version cache directory that exists but can't be written to; a writable one whose record of a failed download is read-only and more than an hour old; and a writable one whose record is a link, itself more than an hour old, to a place the shim can't write
- WHEN `bin/clm hook guard`, `bin/clm hook session-start`, `bin/clm hook guard` again and `bin/clm status` run
- THEN `hook guard` prints nothing on stdout or stderr and exits 0, and `hook session-start` prints only its line, on stdout, and exits 0; in the two writable directories the second `hook guard` calls no `curl`; and `status` exits 1, saying on stderr for the unwritable directory that it can't write to it

#### Scenario: no cache directory
- GIVEN `CLM_BIN` unset, a `curl` that serves a release, and in turn `HOME` and `XDG_CACHE_HOME` both unset; `XDG_CACHE_HOME` set to the empty string with `HOME` unset; and both set to the empty string
- WHEN `bin/clm hook guard`, `bin/clm hook session-start` and `bin/clm status` run
- THEN none calls `curl`; `hook guard` prints nothing on stdout or stderr and exits 0; `hook session-start` prints its line and exits 0; and `status` exits 1 saying why on stderr

#### Scenario: no version
- GIVEN a plugin whose `.claude-plugin/plugin.json` holds no `version`, and a `curl` that serves a release
- WHEN `bin/clm status`, `bin/clm hook guard` and `bin/clm hook session-start` run
- THEN none calls `curl`; `status` exits 1 saying on stderr that it can't read the version; `hook guard` prints nothing and exits 0; and `hook session-start` exits 0 printing its line with `--tag v<version>`

#### Scenario: where the message goes
- GIVEN an empty cache and a `curl` that fails
- WHEN `bin/clm hook session-start` and `bin/clm status` run
- THEN the session start's line is on stdout, one line, with nothing on stderr; and `status`'s message is on stderr, with nothing on stdout, and names `CLM_BIN`

#### Scenario: nothing left behind
- GIVEN an empty cache and, in turn, no `curl` on the `PATH`; a `curl` that serves an archive holding no `clm`, with a matching checksum; and a `curl` that serves a release, after a hook's failed download
- WHEN `bin/clm status` runs
- THEN with no `curl` and with no `clm` in the archive it exits 1, and the version's cache directory holds no binary and no download directory; and after the download that succeeds it holds only `clm`

#### Scenario: a stalled download
- GIVEN an empty cache and a `curl` that records its arguments and serves a release
- WHEN `bin/clm status` runs
- THEN each `curl` call is given `--connect-timeout 10` and `--max-time 120`

#### Scenario: the cache
- GIVEN a `curl` that serves a release, and in turn `XDG_CACHE_HOME` set; `XDG_CACHE_HOME` unset with `HOME` set; and `XDG_CACHE_HOME` set to the empty string with `HOME` set
- WHEN `bin/clm status` runs
- THEN the binary is cached at `$XDG_CACHE_HOME/clm/<plugin version>/clm` in the first case and at `$HOME/.cache/clm/<plugin version>/clm` in the other two

#### Serves
- capability/install-and-release
- capability/guard-sessions

### Requirement: hooks-wired
`plugin/hooks/hooks.json` SHALL run `clm hook session-start` on `SessionStart`, `clm hook guard` on `PreToolUse` for `Bash`, and `clm hook autosync` on `Stop` with a `timeout` of 30, all through `${CLAUDE_PLUGIN_ROOT}/bin/clm`. `plugin/hooks/hooks.json` SHALL name no hooks module: it SHALL have no `modules` key, so the `clm` plugin's command hooks never share a plugin with a hooks module.

#### Scenario: guard wired
- GIVEN the plugin's `hooks.json`
- WHEN it is parsed
- THEN `PreToolUse` has a `Bash` matcher whose command ends in `bin/clm" hook guard`

#### Scenario: autosync wired
- GIVEN the plugin's `hooks.json`
- WHEN it is parsed
- THEN `Stop` has a command that ends in `bin/clm" hook autosync`

#### Scenario: autosync bounded
- GIVEN the plugin's `hooks.json`
- WHEN it is parsed
- THEN the `Stop` command's `timeout` is 30

#### Scenario: no hooks module in clm
- GIVEN the plugin's `hooks.json`
- WHEN it is parsed
- THEN it has no `modules` key

#### Serves
- capability/guard-sessions
- architecture/land-locally

### Requirement: skills
The plugin SHALL ship exactly these skills, each as `skills/<name>/SKILL.md` with a non-empty `description` in its frontmatter: `build`, `configure`, `contribution`, `health`, `help`, `init`, `propose`, `triage`, `upgrade` and `verify`. It SHALL hold no other directory under `skills/`.

#### Scenario: skill without a description
- GIVEN a plugin whose `skills/health/` is missing, or whose `skills/health/SKILL.md` has no `description` in its frontmatter
- WHEN the plugin's `skills/` is read
- THEN it breaks this requirement at `health`

#### Scenario: an extra directory
- GIVEN a plugin with a directory `skills/implement/`
- WHEN the plugin's `skills/` is read
- THEN it breaks this requirement at `implement`

#### Serves
- capability/propose-intent
- capability/build-from-spec
- architecture/land-locally
- architecture/approval-policy

### Requirement: shim-targets
`plugin/bin/clm` SHALL download `clm-x86_64-unknown-linux-musl.tar.gz` on `Linux x86_64`, `clm-aarch64-unknown-linux-musl.tar.gz` on `Linux aarch64`, `clm-x86_64-apple-darwin.tar.gz` on `Darwin x86_64` and `clm-aarch64-apple-darwin.tar.gz` on `Darwin arm64`, and map no other platform. On any other platform, Windows's Git Bash, MSYS2 and Cygwin shells included (`no-windows-support`), it SHALL download nothing and fail as `binary-shim` describes, saying there's no prebuilt `clm` for that platform. Each target it maps SHALL be written as one `target=<target>` assignment, the format `ci/release`'s prepare step parses to learn the shim's targets (`shim-assets-before-release`).

#### Scenario: Linux arm64
- GIVEN an empty cache, a `uname` that reports `Linux` and `aarch64`, and a `curl` that serves release assets
- WHEN `bin/clm status` runs
- THEN `curl` is asked for `clm-aarch64-unknown-linux-musl.tar.gz`

#### Scenario: unsupported platform
- GIVEN an empty cache and, in one run each, a `uname` that reports `FreeBSD` and `amd64`, `CYGWIN_NT-10.0` and `x86_64`, `MINGW64_NT-10.0` and `x86_64`, and `UCRT64_NT-10.0-22631` and `x86_64`
- WHEN `bin/clm status` runs
- THEN `curl` is not called, and it exits 1 saying there's no prebuilt `clm` for that platform

#### Scenario: no Windows target in the shim
- GIVEN the text of `plugin/bin/clm`
- WHEN it is searched for `target=<target>` with the pattern `ci/release`'s prepare step uses
- THEN the targets matched are exactly the four targets above, each matched once

#### Serves
- capability/install-and-release

#### Needs
- ci/release

### Requirement: manifest-owner
`plugin/.claude-plugin/plugin.json` SHALL name `Christopher Lunny` as its `author` and `MIT` as its `license`, and `.claude-plugin/marketplace.json` SHALL name `Christopher Lunny` as its `owner`.

#### Scenario: the owner's full name
- GIVEN the repository
- WHEN `plugin/.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json` are read
- THEN the plugin's `author.name` is `Christopher Lunny` and its `license` is `MIT`, and the marketplace's `owner.name` is `Christopher Lunny`

#### Serves
- capability/install-and-release

### Requirement: codex-package
The repository SHALL hold `.agents/plugins/marketplace.json` listing a plugin `clm` whose `source` is `{"source": "local", "path": "./plugin"}`, and `plugin/.codex-plugin/plugin.json` SHALL name it `clm` with a `version` equal to the cargo workspace version, `skills` `./skills/` and `hooks` `./hooks/codex.json`, so Codex reads the same `plugin/skills/` the Claude Code package ships. Both packages SHALL run the same `plugin/bin/clm` shim.

#### Scenario: version bump
- GIVEN a commit that changes the workspace version
- WHEN `plugin/.codex-plugin/plugin.json` is read at that commit
- THEN its `version` is the workspace's new version

#### Scenario: one set of skills
- GIVEN the two manifests
- WHEN they are read
- THEN both resolve their skills to `plugin/skills/`

#### Serves
- capability/install-and-release
- capability/work-from-any-agent-harness

### Requirement: codex-hooks-wired
`plugin/hooks/codex.json` SHALL run, through `"${PLUGIN_ROOT}/bin/clm"`, `clm hook session-start --agent-harness codex` on `SessionStart`, `clm hook guard --agent-harness codex` on `PreToolUse` with matcher `Bash`, and `clm hook autosync --agent-harness codex` on `Stop` with a `timeout` of 30.

#### Scenario: guard wired for Codex
- GIVEN `plugin/hooks/codex.json`
- WHEN it is parsed
- THEN its pre-tool hook for the shell tool runs a command ending in `bin/clm" hook guard --agent-harness codex`

#### Scenario: autosync bounded for Codex
- GIVEN `plugin/hooks/codex.json`
- WHEN it is parsed
- THEN the `Stop` command's `timeout` is 30

#### Needs
- plugin.hooks.guard/guard-per-agent-harness
- plugin.hooks/hooks-per-agent-harness

#### Serves
- capability/guard-sessions
- capability/work-from-any-agent-harness
- architecture/land-locally

### Requirement: view-command
The plugin SHALL ship `commands/view.md` with a non-empty `description` in its frontmatter.

#### Scenario: command without a description
- GIVEN a plugin whose `commands/view.md` is missing, or has no `description` in its frontmatter
- WHEN the plugin's `commands/` is read
- THEN it breaks this requirement at `view`

#### Serves
- capability/see-project-state
- capability/watch-beside-chat
