---
component: plugin.hooks.guard
base: { frontmatter: "e5a872faaf5caa52" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [plugin.hooks, plugin.hooks.guard.shell, config, vcs, agent-harness]
serves: [capability/guard-sessions, capability/work-from-any-agent-harness, architecture/approval-policy, architecture/guard-blocks-push-to-main, architecture/agent-harness-adapters]
```

## ADDED Requirements

### Requirement: guard-per-agent-harness
`clm-guard` SHALL offer `guard_hook(agent_harness, event)`, which reads the event with `agent-harness`'s `parse_event` for that agent harness, judges the event's shell command and `cwd` by the same rules whatever the agent harness, and returns the stdout and exit code `agent-harness`'s `render` gives for that agent harness, so an `ask` on an agent harness that can't confirm blocks the command. An event for another tool than the agent harness's shell tool SHALL be allowed, as a non-`Bash` event is today, and an event that can't be parsed SHALL return what the guard returns for one today. `guard` and `guard_within` SHALL return `agent-harness`'s `Decision`, and `ask_json` SHALL be `agent-harness`'s Claude Code rendering of an `Ask`, so each format is written once.

#### Scenario: push to main under Codex
- GIVEN a Codex pre-tool event for `git push origin main` in a repository under CLM
- WHEN `guard_hook` runs for `Codex`
- THEN it returns Codex's block, naming `guard-blocks-push-to-main`

#### Scenario: approve under Codex
- GIVEN a Codex pre-tool event for `clm approve spec/x`
- WHEN `guard_hook` runs for `Codex`
- THEN it returns Codex's block, its reason saying to run the command in the human's own terminal

#### Scenario: Claude Code unchanged
- GIVEN a Claude Code `PreToolUse` event for `clm approve spec/x`
- WHEN `guard_hook` runs for `ClaudeCode`
- THEN it returns the `ask` permission decision's JSON and exit 0

#### Needs
- agent-harness/hook-events
- agent-harness/abilities

#### Serves
- capability/work-from-any-agent-harness
- architecture/agent-harness-adapters
- architecture/guard-blocks-push-to-main
