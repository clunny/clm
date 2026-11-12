---
component: plugin.hooks
base: { frontmatter: "85d6fea685d2be9c" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [config, model, sync, view, view.repo, host, vcs, upgrade, agent-harness]
serves: [capability/guard-sessions, capability/work-from-any-agent-harness, architecture/approval-policy, architecture/guard-blocks-push-to-main, architecture/agent-harness-adapters]
```

## ADDED Requirements

### Requirement: hooks-per-agent-harness
`clm-hooks` SHALL offer `session_start_hook(agent_harness, event)` and `autosync_hook(agent_harness, event)`, which find the event's `cwd` with `agent-harness`'s `parse_event` for that agent harness and do what `session-status` and `autosync-hook` say, returning stdout and an exit code of 0. `session_start_hook` SHALL write what it prints in the form that agent harness adds to the session's context: plain stdout for Claude Code, and Codex's session-start output as its hooks documentation gives it.

#### Scenario: Codex session start
- GIVEN a Codex session-start event whose `cwd` has a valid `specs/`
- WHEN `session_start_hook` runs for `Codex`
- THEN Codex's session-start output carries the `CLM status` text, and the exit code is 0

#### Scenario: Codex end of turn
- GIVEN a Codex end-of-turn event whose `cwd` is a spec branch's checkout with uncommitted spec work
- WHEN `autosync_hook` runs for `Codex`
- THEN it runs `clm autosync` there, its stdout is empty and the exit code is 0

#### Needs
- agent-harness/hook-events

#### Serves
- capability/work-from-any-agent-harness
- architecture/agent-harness-adapters
