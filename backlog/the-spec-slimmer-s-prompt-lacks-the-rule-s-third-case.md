---
title: "The spec-slimmer's prompt lacks the rule's third case"
kind: deferred
components: [plugin.agents]
origin: 20261119-wave-1-by-the-rule
---

`requirements-state-what-the-contract-shows` lets a requirement about a file's text stay in three cases since `20261119-wave-1-by-the-rule`; the third is a value the project's own `clm.toml` holds, which no spec branch can set. `plugin/agents/spec-slimmer.md` gives two, so in every later wave it proposes cutting requirements such as `config/verify-here`, as it did in wave 1. Add the third case to the prompt's summary of the rule, as `docs/spec-format.md` words it. The prompt's text is source (`prompt-text-is-source`): fixed under `clm claim plugin.agents --fix`, with no test. Due before wave 2 launches the slimmer.
