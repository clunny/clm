---
component: clm
base: { frontmatter: "ebca29b2c812fa99" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [model, check, check.affected, check.unreferenced, check.health, gate, apply, context, vcs, orchestrator, orchestrator.run, orchestrator.checkpoint, orchestrator.triage, orchestrator.runner, config, init, view, view.repo, plugin.hooks, plugin.hooks.guard, ledger, land, host, sync, effort, rename, upgrade, verify, agent-harness]
```

## ADDED Requirements

### Requirement: agent-harness-command
`clm agent-harness` SHALL print `agent harness <name>`, the name of the agent harness `agent-harness`'s `detect` finds in its environment, then one line `<ability> yes` or `<ability> no` for each of `confirm`, `guard`, `session-hooks`, `subagents`, `isolated-worktrees`, `named-agents` and `model-choice` from `agent-harness`'s `abilities`, in that order, then `build-model <alias>` when `model-choice` is yes, and exit 0. It needs no CLM repository, and `clm --help` SHALL list it.

#### Scenario: under Codex
- GIVEN Codex's marker set and `CLAUDECODE` unset
- WHEN `clm agent-harness` runs outside any repository
- THEN it prints `agent harness codex`, `confirm no`, `guard yes`, `subagents yes`, `isolated-worktrees no`, `named-agents no` and `model-choice no`, no `build-model` line, and exits 0

#### Scenario: under Claude Code
- GIVEN `CLAUDECODE=1`
- WHEN `clm agent-harness` runs
- THEN it prints `agent harness claude-code`, `confirm yes` and `build-model sonnet`

#### Needs
- agent-harness/detect
- agent-harness/abilities

#### Serves
- capability/work-from-any-agent-harness

### Requirement: hook-agent-harness-flag
`clm hook guard`, `clm hook session-start` and `clm hook autosync` SHALL take `--agent-harness <name>`, a name `agent-harness`'s `AgentHarness::from_name` reads, `claude-code` when it is left out, and SHALL pass the event on stdin and that agent harness to `plugin.hooks.guard`'s `guard_hook` and `plugin.hooks`' `session_start_hook` and `autosync_hook`, printing their stdout and exiting with their exit code. An unknown name SHALL make `clm hook guard` exit 1, naming it on stderr, an exit Claude Code doesn't read as a block (`guard-fails-open`) and that the build confirms Codex doesn't either from its hooks documentation, choosing another non-blocking exit if it does, and make the other two print nothing and exit 0.

#### Scenario: push to main under Codex
- GIVEN a Codex pre-tool event for `git push origin main` in a repository under CLM
- WHEN `clm hook guard --agent-harness codex` runs
- THEN it gives Codex's block, naming `guard-blocks-push-to-main`

#### Scenario: no flag
- GIVEN a Claude Code `PreToolUse` event for `clm approve spec/x`
- WHEN `clm hook guard` runs with no `--agent-harness`
- THEN it prints the `ask` permission decision's JSON and exits 0, as before

#### Scenario: unknown agent harness
- GIVEN any event
- WHEN `clm hook guard --agent-harness nope` runs
- THEN it exits 1 naming `nope`

#### Needs
- plugin.hooks.guard/guard-per-agent-harness
- plugin.hooks/hooks-per-agent-harness

#### Serves
- capability/work-from-any-agent-harness
- architecture/agent-harness-adapters

## MODIFIED Requirements

### Requirement: retired-terms <!-- base: b30c0e50ba076b6e -->
`scripts/retired-terms.txt` SHALL hold the list of retired terms, one per line, a line that is blank or starts with `#` holding none, each the old name a completed rename replaced (`plain-terms-renames`) in a form ordinary English never takes: a command after `clm` and a space, a command's outcome line, a flag, a key, a `kind:` written in backticks, a coined term of several words, or a coined word that isn't an English word. A word of plain English SHALL NOT join the list on its own, whatever it once named, since people keep using it in its everyday sense; its retired name is listed in the forms above instead. The scan SHALL be `scripts/retired-terms.sh`, a POSIX `sh` script that needs only `sh` and `grep`: run as `sh scripts/retired-terms.sh [--list <file>] [<path>...]` from the repository root, it reads the list from `<file>` (`scripts/retired-terms.txt` by default) and searches each `<path>`, a file or a directory read recursively, skipping binary files, or with no path the text CLM ships or works from: every file, recursively, under `docs/guide/`, `plugin/`, `specs/` and `web/src/`, and `docs/spec-format.md`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `AGENTS.md` and `README.md`. A list line's term is its text with leading and trailing whitespace and any carriage return removed, a run of spaces or tabs within it counting as one gap. A default path that doesn't exist is skipped. It SHALL print one line `<path>:<line>: <term>`, `<path>` as given or as found under a directory given, for each term a line holds as a whole word, in path, then line, then list order, and exit 1 when it printed any, exit 0 when it printed none, and exit 2 naming the problem when the list can't be read or a search fails. Matching ignores case on both sides; a whole word is one with no letter, digit or `_` directly before or after it, so a hyphen is a boundary (`frob` matches in `frob-scoped` and not in `frobnicate`), and a term of several words matches with any run of spaces between them. A line holding the text `clm:remove-by` anywhere SHALL be exempt, so an alias may keep an old name until its contract change removes it. History and proposals SHALL be exempt by not being read: `changes/`, archived or live, `docs/decisions/`, `backlog/` and the ledger keep the names they were written with, since a change that describes a rename has to name the old word; what a live change's deltas say reaches `specs/` when it is applied, and is read there. `clm check` runs the scan as a detector (`config/retired-terms-detector`), so a spec branch whose text names a retired term fails before its land. A test in `crates/clm/tests/` SHALL run the scan with no path on this repository, and on the help `clm --help` and `clm <command> --help` print for every command and subcommand, each written to a file of its own in a temporary directory, named for its command with `-` between its words (`clm-land`), failing with the scan's lines when it prints any. The change that completes a rename adds the old names it retires, in those forms.

#### Scenario: a retired term in a skill
- GIVEN a list file holding `grommet stub` and a skill's `SKILL.md` line reading `make it a grommet stub`
- WHEN the scan runs with that list on the skill's directory
- THEN it prints that file, its line number and `grommet stub`, and exits 1

#### Scenario: an alias kept for expand-contract
- GIVEN a list file holding `clm frob` and a line reading `` `clm frob` still runs `clm twiddle` // clm:remove-by 20261201-x ``
- WHEN the scan runs with that list on its file
- THEN it prints nothing for that line

#### Scenario: history is left alone
- GIVEN a fixture repository whose `scripts/retired-terms.txt` holds `frobbed`, and whose archived change, live change's `change.md` and decision use it
- WHEN the scan runs with no path at the fixture's root
- THEN it prints nothing and exits 0

#### Scenario: every named path is read
- GIVEN a fixture repository whose `scripts/retired-terms.txt` holds `frobbed`, and a file using it under each directory the scan names and in each of the files it names
- WHEN the scan runs with no path at the fixture's root
- THEN it prints one line for each of them

#### Scenario: whole words only
- GIVEN a list file holding `frob`
- WHEN the scan runs with it on a guide page reading `frobnicate`, and on one reading `the frob-scoped hook`
- THEN it prints nothing for the first and a line for the second

#### Scenario: a multi-word term across spaces
- GIVEN a list file holding `grommet stub`
- WHEN the scan runs with it on a page reading `GROMMET   stub`
- THEN it prints that line

#### Scenario: no list
- GIVEN a list path that names no file
- WHEN the scan runs with it
- THEN it exits 2 naming that path

#### Scenario: no plain English word on the list
- GIVEN the list check, a function over a list of terms, run once on a fixture list holding `clm frob`, `--frob` and the bare word `frob`, and once on `scripts/retired-terms.txt`
- WHEN it judges each term
- THEN on the fixture it fails naming `frob` alone, and on `scripts/retired-terms.txt` it passes: every term holds a space, a hyphen, an underscore or a backtick, or is the old name of the status `unbuilt`

#### Serves
- architecture/plain-terms
