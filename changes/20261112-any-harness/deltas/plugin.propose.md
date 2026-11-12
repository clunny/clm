---
component: plugin.propose
base: { frontmatter: "1b79214002e6ad1e" }
---

## MODIFIED Frontmatter
```yaml
serves: [architecture/method-in-the-plugin, capability/work-from-any-harness]
```

## ADDED Requirements

### Requirement: propose-follows-harness
`skills/propose/SKILL.md` SHALL hold `plugin/skills-follow-harness`'s rule for its builder check and `architecture-auditor` launches and for its approval step: it SHALL tell the session to run `clm harness` before the builder check, to launch both subagents with the harness's own subagent tool, or to run the builder check and the audit in the session itself one after the other where `subagents` is `no`, and, where `confirm` is `no`, to ask for the human's yes in the conversation and then to have the human run `clm approve spec/<id>` in their own terminal, landing once they say it has run, and never to run `clm approve` itself there.

#### Scenario: approval under a harness that can't confirm
- GIVEN `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it names `clm harness`, `confirm` and `own terminal`



#### Serves
- capability/work-from-any-harness
