---
title: "spec-auditor: the build agent's own model is set by the implement skill, beyond the binary's boundary"
kind: workaround
components: [capability]
requirement: capability/right-model-per-task@06c2f40a
---

The scenario says the build's agent names its model, a cheaper one than the main session's. That agent is launched by /clm:implement's Agent call with model: "sonnet" (plugin/skills/implement/SKILL.md), which nothing at the clm binary boundary carries: clm run's launcher names no model. The capability test can observe only that the review model passes through --review-model into the prompt and that the prompt names sonnet for every other subagent; the build agent's own model goes unproven. The spec-auditor reported it as a scenario the code can't observe.
