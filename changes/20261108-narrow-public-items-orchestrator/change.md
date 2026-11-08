---
status: proposed
resolves: [narrow-public-items-orchestrator]
---

## Intent

Take `claim_node` out of `orchestrator`'s Contract, now that `orchestrator.run` claims through `claim_local`, so the library can make it private and `clm.toml` can drop its `--allow` entry.
