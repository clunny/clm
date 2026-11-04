---
title: "The nodes that decide what reaches main, and whether a yes is the human's, are marked significant"
date: 2026-10-27
status: superseded
superseded_by: yes-no-review
---

`significance-rubric` flags every change in a node whose frontmatter says `significant: true`, so a reviewer who reads a change's `## Significant` sees each edit to a trust boundary, Design and added requirements included, not only its Contract and modified requirements. A node is marked when it decides, without a human, what reaches `main`, or decides whether an approval is the human's:

- `gate`: which PRs land on checks alone, and at which tier (`computed-gate-tiers`);
- `land`: the only thing that pushes `main`; it runs the gate and the checks, and records and reads approvals (`land-locally`);
- `ci`: `.github/scripts/ci-check.sh`, the check command `clm land` runs before it pushes, and the release workflow that publishes what users install;
- `plugin.hooks`: the guard, which makes a `clm approve` or `clm review` the human's (`approve-in-session`) and blocks pushes to `main` (`guard-blocks-push-to-main`).

`orchestrator` and `orchestrator.run` are not marked. The backlog item that proposed this expected `run` to set the permissions of unattended sessions, but since `builds-in-subagents` a build runs under the permission mode of the human's session and `clm run` sets none and ships no allowlist. What a build may do is bounded by the guard and by `clm land`, both marked; the queue and the build prompt decide only what gets built, and every build still lands through the gate.

Marking is a frontmatter change, which the gate rates `HARD` (`gate/spec-element-tiers`), so a node is marked or unmarked only with a human's approval. A node added later that pushes `main`, approves, or bypasses the gate should be marked in the change that adds it.

Rejected:
- **Marking `orchestrator.run` too.** It sets no permissions, and every change to its prompt would then need citing, for edits a reviewer of the gate and the guard already catches downstream.
- **Marking only `gate` and `ci`**, as first proposed. Since the cutover (`land-locally`) hosted CI no longer gates `main`; `clm land` and the guard do, and leaving them unmarked would leave the boundary's two most direct nodes out.
