---
title: "`clm check` reports imports between nodes that `depends_on` doesn't declare, from `clm survey`'s import edges"
date: 2026-10-27
status: accepted
---

`check`'s Contract has long listed "path ownership, undeclared dependencies" among its rules, and `docs/spec-format.md` uses the rule as its example, but nothing checked either (`backlog/dependency-check`). `depends_on` decides what a build sees (`spine-scoped-context`) and when a node is ready (`node-reconciliation`), so an import it doesn't declare means a build that can't see the contract it codes against. On 2026-10-27 three edges were undeclared: `orchestrator.run` and `orchestrator.checkpoint` on `model`, and the root on `sync`; `20261027-check-hardening` declares them.

`clm survey --nodes` already finds such edges for a proposed node map, with tree-sitter (`tree-sitter-analysis`). `init` now exposes those edges as `import_edges(root)`, and the `clm check` command judges them against the spec graph: an edge from node `a`'s file to node `b`'s file needs `b` in `a`'s `depends_on`. Only direct edges count, since the build of `a` reads only its direct dependencies' contracts, and a parent or child gets no exemption, since `depends_on` is separate from ownership (`nested-splits`).

It runs in the `clm check` command, not in `run`: tree-sitter is C and `clm-check` must build for wasm, and `run`'s other callers (`clm sync`, the Worker) don't need it. `--no-detectors` doesn't skip it, because it needs no tool and a doc-only land still changes `depends_on`.

`paths` overlap is the other half of the item and needs no parser, so it is a `run` rule, `check/paths-disjoint`.

Rejected: putting the import extraction in `clm-check` (it would lose the wasm build), and a separate checking node (one more node for one rule whose extraction already lives in `init`).
