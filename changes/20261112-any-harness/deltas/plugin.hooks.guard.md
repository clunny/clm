---
component: plugin.hooks.guard
base: { frontmatter: "e5a872faaf5caa52" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [plugin.hooks, plugin.hooks.guard.shell, config, vcs, harness]
serves: [capability/guard-sessions, capability/work-from-any-harness, architecture/approval-policy, architecture/guard-blocks-push-to-main, architecture/harness-adapters]
```

## ADDED Requirements

### Requirement: guard-per-harness
`clm-guard` SHALL offer `guard_hook(harness, event)`, which reads the event with `harness`'s `parse_event` for that harness, judges the event's shell command and `cwd` by the same rules whatever the harness, and returns the stdout and exit code `harness`'s `render` gives for that harness, so an `ask` on a harness that can't confirm blocks the command. An event for another tool than the harness's shell tool SHALL be allowed, as a non-`Bash` event is today, and an event that can't be parsed SHALL return what the guard returns for one today. `guard` and `guard_within` SHALL return `harness`'s `Decision`, and `ask_json` SHALL be `harness`'s Claude Code rendering of an `Ask`, so each format is written once.

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
- harness/hook-events
- harness/abilities

#### Serves
- capability/work-from-any-harness
- architecture/harness-adapters
- architecture/guard-blocks-push-to-main
