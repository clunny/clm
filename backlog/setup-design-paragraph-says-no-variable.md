---
title: "orchestrator.run's Setup design paragraph still says clm setup sets no variable"
kind: idea
components: [orchestrator.run]
---

## Intent

The `Setup` paragraph of `orchestrator.run`'s Design says `clm setup` runs "under no lock and setting no variable", but `setup-command` now has it set each `[env]` variable of the settings (`config/env-setting`). The code follows the requirement. A spec branch should reword the Design sentence to match.
