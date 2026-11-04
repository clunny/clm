---
title: "No route for a small code-only change on a converged component"
kind: bug
components: [orchestrator]
---

Step: making a small code change with no spec change on a component with nothing pending.

What happened: `clm claim <component>` refuses with "converged: nothing is pending", and `clm claim <component> --fix` needs a `kind: bug` or `kind: deferred` backlog item. The change was an improvement, not a bug, so the session had no claim to take. `clm land` accepted a branch that isn't named `impl/*`, so the work went on a plain branch with no claim.

Expected: a stated route for this case, either a claim that takes no backlog item or an instruction in CLAUDE.md and the guide saying that such a change goes on an unclaimed branch.

Rules involved: `code-fix-resolves-bug`, `fold-small-defer-large`.
