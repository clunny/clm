---
component: plugin.init
base: { frontmatter: "0beda6395462f53a" }
---

## MODIFIED Frontmatter
```yaml
serves: [architecture/method-in-the-plugin, capability/work-from-any-harness]
```

## ADDED Requirements

### Requirement: init-follows-harness
`skills/init/SKILL.md` SHALL hold `plugin/skills-follow-harness`'s rule for its per-component subagents and its approval: it SHALL tell the session to run `clm harness` before its first launch, to draft each component in the session itself, one at a time, where `subagents` is `no`, and, where `confirm` is `no`, to have the human run `clm approve` in their own terminal after their yes.

#### Scenario: init under Codex
- GIVEN `skills/init/SKILL.md`
- WHEN a test reads it
- THEN it names `clm harness` and `own terminal`



#### Serves
- capability/work-from-any-harness
