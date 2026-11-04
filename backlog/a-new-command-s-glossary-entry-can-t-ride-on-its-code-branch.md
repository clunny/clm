---
title: "A new command's glossary entry can't ride on its code branch"
kind: bug
components: [plugin, gate]
---

Step: building a requirement that adds a CLI command, on an `impl/*` branch.

What happened: the requirement `clm/glossary-covers-terms` (enforced by a test) makes every command name appear in the guide's glossary. The glossary is a spec path, so adding the entry on the code branch made `clm gate --base origin/main` print `mixed`. The build had to land the glossary line on a separate branch of its own before the code branch could land: two branches, two landings and an extra check run for one requirement.

Expected: a requirement that adds a command can be built and landed on one branch. Either the spec branch that adds the command's requirement adds its glossary entry too (the `propose` skill would have to say so, and the glossary test would then pass as soon as the code lands), or a code branch may carry the glossary entries for the terms its code introduces.

Rules involved: `clm/glossary-covers-terms`, the gate's spec/code path classification (`yes-no-review`, `mixed`).
