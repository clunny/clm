---
title: "A session's spec-auditor on a code fix runs on the agent file's default model"
kind: bug
components: [plugin]
---

Step: a code fix a session makes itself (`clm claim <component> --fix`, or a small code change), reviewed before landing as AGENTS.md's "Prove" step says: run `/code-review` and the `spec-auditor` subagent.

What happened: the session launched `clm:spec-auditor` with no `model`, so it ran on the agent file's `model: sonnet`. A build that `clm run` prompts launches the same auditor on the loop's session model (`--review-model`, `opus` here), and the propose skill names a model for each of its own launches (the builder check on `build-model`, the architecture audit on the session's model). Neither AGENTS.md nor the propose skill's `## Code fixes` section says which model the spec-auditor of a session's own code fix runs on, so that review runs on a weaker model than the same review in a build.

Expected: wherever a session is told to run the spec-auditor outside a `clm run` prompt (AGENTS.md's Prove step, the propose skill's Code fixes, the contribution skill), the instruction names its model as the build prompt does (`model-per-launch`).

Rule involved: `model-per-launch`.
