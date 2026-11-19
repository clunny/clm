---
title: "The spec-slimmer's prompt predates the rule for a requirement"
kind: deferred
components: [plugin.agents]
origin: 20261118-the-pass-s-tooling
---

`plugin/agents/spec-slimmer.md` still proposes cuts by four kinds of its own and returns one list. `plugin.agents`' Design, as `20261118-the-pass-s-tooling` rewrote it, says what the subagent reads and returns: the component's spec, code and citing tests and its lines under `clm health`'s `outside-contract`, `test-placement`, `design-exit` and `uncited-test`; then four lists, reworded, newly stated (each with the test and assertion that prove it today, or `none`), moved and cut (each with what names its ID and what would be lost).

Rewrite the prompt to that Design. It is the fixed text of an agent file, which no requirement states (`prompt-text-is-source`), so the fix is code only, under `clm claim plugin.agents --fix backlog/the-spec-slimmer-predates-the-rule-for-a-requirement`, with no test. The frontmatter's `model: sonnet` stays (`plugin/model-names`); a session of phase 5's waves launches the subagent with the build model `clm agent-harness` prints. The `clm health` signals it reads exist once `check.health/rule-signals` is built.
