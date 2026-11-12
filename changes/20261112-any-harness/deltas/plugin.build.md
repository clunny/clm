---
component: plugin.build
base: { frontmatter: "95346ad1e6e10600" }
---

## MODIFIED Frontmatter
```yaml
serves: [architecture/method-in-the-plugin, capability/work-from-any-agent-harness]
```

## ADDED Requirements

### Requirement: build-follows-agent-harness
`skills/build/SKILL.md` SHALL hold `plugin/skills-follow-agent-harness`'s rule for its builds: it SHALL tell the session to run `clm agent-harness` before its first launch; where `isolated-worktrees` is `no`, to add each build's worktree with `git worktree add .claude/worktrees/agent-<component> origin/main` and tell the build, before the launcher `clm run` printed, to work in that directory, so `clm run --finish` and `clm wrap-up` tidy it as they tidy a subagent's `agent-*` worktree; where `subagents` is `no`, to build the components one at a time in the session itself, each in such a worktree; and, where `model-choice` is `no`, to launch with the session's own model and pass its name as `--review-model`.

#### Scenario: builds under Codex
- GIVEN `skills/build/SKILL.md`
- WHEN a test reads it
- THEN it names `clm agent-harness`, `isolated-worktrees` and `.claude/worktrees/agent-<component>`



#### Needs
- clm/agent-harness-command

#### Serves
- capability/work-from-any-agent-harness
