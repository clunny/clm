---
component: plugin.hooks
base: { frontmatter: "85d6fea685d2be9c" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [config, model, sync, view, view.repo, host, vcs, upgrade, harness]
serves: [capability/guard-sessions, capability/work-from-any-harness, architecture/approval-policy, architecture/guard-blocks-push-to-main, architecture/harness-adapters]
```

## ADDED Requirements

### Requirement: hooks-per-harness
`clm hook session-start` and `clm hook autosync` SHALL take `--harness <name>` as `plugin.hooks.guard/guard-per-harness` does, reading the event as Claude Code's when it is left out, and SHALL find the event's `cwd` with `harness`'s `parse_event` for that harness. `clm hook session-start` SHALL write what it prints in the form that harness adds to the session's context: plain stdout for Claude Code, and Codex's session-start output as its hooks documentation gives it. An unknown name SHALL make either hook print nothing and exit 0, as an event it can't parse does.

#### Scenario: Codex session start
- GIVEN a Codex session-start event whose `cwd` has a valid `specs/`
- WHEN `clm hook session-start --harness codex` runs
- THEN Codex's session-start output carries the `CLM status` text, and it exits 0

#### Scenario: Codex end of turn
- GIVEN a Codex end-of-turn event whose `cwd` is a spec branch's checkout with uncommitted spec work
- WHEN `clm hook autosync --harness codex` runs
- THEN it runs `clm autosync` there, prints nothing and exits 0

#### Serves
- capability/work-from-any-harness
- architecture/harness-adapters
