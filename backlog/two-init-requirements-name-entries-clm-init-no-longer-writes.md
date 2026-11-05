---
title: "Two init requirements name entries clm init no longer writes"
kind: bug
components: [init]
---

The ARCHITECTURE.md that `clm init` writes holds no entry of CLM's own since `architecture-template-proof` dropped the `## Enforced by clm` list. `init/template-says-pull-request` and `init/templates-say-new-words` still have scenarios and prose naming lines of that list; they need a spec change to drop the unobservable parts. Their tests assert the remainder meanwhile.
