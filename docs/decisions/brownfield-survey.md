---
title: "An existing codebase's spec graph is drawn from a deterministic survey, then filled per component"
date: 2026-10-22
status: accepted
---

Bringing an existing codebase under CLM recovers as much of its spec graph as the code, tests and docs support. The human chooses whether to adopt all of the code at once or one part first, leaving the rest `outside` every component for later changes to adopt (`incremental-adoption`). The per-component work can't start until there are components, so it runs in passes:

1. **Map.** `clm survey` prints a compact, deterministic map: directories with file and line counts per language, manifests, test locations, docs and ADRs, and the import edges between directories, extracted with tree-sitter (`tree-sitter-analysis`). One agent drafts the component map (components, `paths`, `depends_on`, a line of responsibility each) from it and the docs. `clm survey --components` reports uncovered files, overlapping paths and imports `depends_on` doesn't allow, and the agent revises until it reports nothing.
2. **Check in.** The human sees the component map and corrects it before anything else is written: boundaries are the costliest thing to get wrong and the cheapest to fix here.
3. **Fill.** One subagent per component, given only its paths, writes the Contract, Design and requirements. Requirements describe what the code does today, not what it should; bugs become `bug` backlog items.

The result is an ordinary change on a spec branch that needs review, since it edits `ARCHITECTURE.md` and `CAPABILITIES.md` (`yes-no-review`). Work is built per component, not per change (`node-reconciliation`): its components' builds prove each requirement by citing it at its current version (`req:<component>/<slug>@<version>`) in existing tests or new ones. Until every requirement the change added is cited, `clm pending` lists the rest as `unbuilt` and the change isn't complete; once all are current, `clm rebase` archives it (`archive-on-sync`). Detectors start off (`clm init --no-detectors`) so the existing code passes `clm check`; the builds remove the dead code in their components, and the root component's build adds the detectors to `clm.toml`.

Existing CI workflows' build and test steps move into the check script `clm land` runs (`scripts/clm-check.sh`, `[land] check`) and those workflows are removed in the bootstrap commit the human pushes to `main`, before the spec branch is cut, because `clm land` lands on that script and `clm check`, and a failing test elsewhere could still land. They are code paths, so they can't ride on the spec branch: a branch holding spec and code paths is `mixed`, which the gate blocks. The skill warns the human first and the init change lists what moved under `## Intent`. Deploy, release and scheduled workflows stay.

Rejected: the agent exploring the code with search tools alone, which works on a small repository but doesn't fit one context on a large one and draws a different map each run.
