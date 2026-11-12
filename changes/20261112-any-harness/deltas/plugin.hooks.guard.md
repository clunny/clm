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
`clm hook guard` SHALL take `--harness <name>`, a name `harness`'s `Harness::from_name` reads, and with no `--harness` SHALL read the event as Claude Code's, as it does today. It SHALL read the event with `harness`'s `parse_event` for that harness, judge the event's shell command and `cwd` by the same rules whatever the harness, and print and exit by `harness`'s `render` for that harness, so an `ask` on a harness that can't confirm blocks the command. An event for another tool than the harness's shell tool SHALL be allowed, as a non-`Bash` event is today. An unknown `--harness` name SHALL exit with neither 0 nor 2, naming it, as the guard fails open on a fault of its own (`guard-fails-open`).

#### Scenario: push to main under Codex
- GIVEN a Codex pre-tool event for `git push origin main` in a repository under CLM
- WHEN `clm hook guard --harness codex` runs
- THEN it gives Codex's block, naming `guard-blocks-push-to-main`

#### Scenario: approve under Codex
- GIVEN a Codex pre-tool event for `clm approve spec/x`
- WHEN `clm hook guard --harness codex` runs
- THEN it gives Codex's block, its reason saying to run the command in the human's own terminal

#### Scenario: no flag
- GIVEN a Claude Code `PreToolUse` event for `clm approve spec/x`
- WHEN `clm hook guard` runs with no `--harness`
- THEN it prints the `ask` permission decision's JSON and exits 0, as before

#### Serves
- capability/work-from-any-harness
- architecture/harness-adapters
- architecture/guard-blocks-push-to-main
