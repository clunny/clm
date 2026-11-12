---
component: plugin.triage
base: { frontmatter: "a4a968c268dfc818" }
---

## MODIFIED Frontmatter
```yaml
serves: [architecture/method-in-the-plugin, capability/work-from-any-agent-harness]
```

## ADDED Requirements

### Requirement: triage-follows-agent-harness
`skills/triage/SKILL.md` SHALL hold `plugin/skills-follow-agent-harness`'s rule for its drafts, folds and approvals: it SHALL tell the session to run `clm agent-harness` before its first launch; where `isolated-worktrees` is `no`, to give each draft and fold its own worktree under `.claude/worktrees/` before launching it, as `build` does for a build; where `subagents` is `no`, to run each draft in the session itself, one at a time; and, where `confirm` is `no`, to ask for the human's yes in the conversation and then to have the human run each `clm approve` in their own terminal, never running it from the session.

#### Scenario: approvals under Codex
- GIVEN `skills/triage/SKILL.md`
- WHEN a test reads it
- THEN it names `clm agent-harness`, `confirm` and `own terminal`



#### Needs
- clm/agent-harness-command

#### Serves
- capability/work-from-any-agent-harness
