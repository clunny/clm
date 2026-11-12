---
component: init
---

## MODIFIED Requirements

### Requirement: init-files <!-- base: 633c5ab3abef0323 -->
`clm init` SHALL write `clm.toml`, `scripts/clm-check.sh` (executable), `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/SPEC.md`, `AGENTS.md` and `CLAUDE.md`, each only when it is missing, and SHALL write no file under `.github/`. `AGENTS.md` SHALL hold the project's instructions for every harness (`platform`), and the `CLAUDE.md` it writes SHALL be the one line `@AGENTS.md`, which Claude Code reads as an import, so every harness reads one text. When the repository already has a `CLAUDE.md` that doesn't hold the line `@AGENTS.md`, `clm init` SHALL keep it, still write a missing `AGENTS.md`, and print a line saying to add `@AGENTS.md` to `CLAUDE.md` so Claude Code reads the same instructions. `clm.toml` SHALL set `host` (from `--host`, else what `clm-host`'s `host_for_remote` gives for `origin`'s URL, or `none` with no remote), `approval = "any"` and `[land] check = "scripts/clm-check.sh"`. Neither `ARCHITECTURE.md` nor `CAPABILITIES.md` SHALL hold an entry; `ARCHITECTURE.md` SHALL name, outside its `## Constraints` section, `clm guide constraints`, where CLM's carried constraints are. On the result, `clm check` SHALL pass. Outside a git repository it SHALL exit 1 and write nothing. It SHALL call no `gh` and no `az`.

#### Scenario: empty repository
- GIVEN an empty git repository whose `origin` is `https://github.com/o/r.git`
- WHEN `clm init --lang rust` runs
- THEN all seven files exist, `CLAUDE.md` is `@AGENTS.md`, `clm-check.sh` is executable and holds `cargo test`, `clm.toml` holds the Rust detector, `host = "github"`, `approval = "any"` and `check = "scripts/clm-check.sh"`, `.github/` doesn't exist, and `clm check` exits 0

#### Scenario: human layers
- GIVEN an empty git repository, and another holding a `CAPABILITIES.md` of its own
- WHEN `clm init` runs in each
- THEN in the first `clm hashes architecture` and `clm hashes capability` list nothing, `clm pending` lists nothing, and `ARCHITECTURE.md` names `clm guide constraints`, and in the second `CAPABILITIES.md` is unchanged and the output says `kept CAPABILITIES.md`

#### Scenario: no remote
- GIVEN an empty git repository with no remote
- WHEN `clm init` runs
- THEN `clm.toml` holds `host = "none"`

#### Scenario: Azure DevOps remote
- GIVEN an empty git repository whose `origin` is `https://o@dev.azure.com/o/p/_git/r`
- WHEN `clm init` runs
- THEN `clm.toml` holds `host = "azure"`

#### Scenario: host given
- GIVEN the GitHub repository above
- WHEN `clm init --host git` runs
- THEN `clm.toml` holds `host = "git"`

#### Scenario: project files kept
- GIVEN a repository with its own `CLAUDE.md` and `clm.toml`
- WHEN `clm init` runs
- THEN both are unchanged, the output lists `kept CLAUDE.md` and `kept clm.toml`, `AGENTS.md` is written, and the output says to add `@AGENTS.md` to `CLAUDE.md`

#### Scenario: not a repository
- GIVEN a directory that isn't in a git repository
- WHEN `clm init` runs
- THEN it exits 1 and the directory is unchanged

#### Needs
- model/human-layers
- config/host-setting
- clm/init-host-values
- host/host-for-remote

### Requirement: claude-md-points-at-guide <!-- base: 7f48ae23b4c8e49a -->
The `AGENTS.md` that `clm init` writes SHALL point at the guide by naming `clm guide`, and SHALL hold no URL (`http://` or `https://`).

#### Scenario: pointer
- GIVEN an empty repository
- WHEN `clm init` runs
- THEN its `AGENTS.md` holds `clm guide` and no `https://`

#### Serves
- architecture/method-in-the-plugin

### Requirement: claude-md-names-the-skills <!-- base: d2ecddb8d151aa8e -->
The `AGENTS.md` that `clm init` writes SHALL name `/clm:propose`, `/clm:build` and `/clm:help`, saying they are the `propose`, `build` and `help` skills whatever the harness calls them, and SHALL NOT name `/clm:implement`. It SHALL tell the session never to run `clm approve` itself, but to have the human run it, through the harness's confirmation or in their own terminal (`approval-policy`).

#### Scenario: a new repository
- GIVEN a repository with no `AGENTS.md`
- WHEN `clm init` runs
- THEN its `AGENTS.md` holds `/clm:propose`, `/clm:build`, `/clm:help` and `clm approve`, and not `/clm:implement`

#### Serves
- architecture/method-in-the-plugin
- capability/work-from-any-harness

#### Needs
- plugin.build/skill-named-build
- plugin/help-skill
