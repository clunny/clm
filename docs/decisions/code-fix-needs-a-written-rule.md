---
title: "A bug is fixed on a code branch only when a requirement as written states the behaviour the fix restores"
date: 2026-10-30
status: accepted
---

`code-fix-resolves-bug` let a `kind: bug` item whose fix changes no spec path be fixed straight on `impl/<node>`. On 2026-10-30 the `clm apply` defect was fixed that way and both reviews found its tests citing requirements whose text didn't state the fixed behaviour; the fix needed `apply/loaded-graph` first. A fix that no requirement states is code that serves nothing written: when the rule its test cites is reworded, the test is rebuilt to the new text, and a slimming or `no-dead-code` pass can remove the code without anyone seeing a requirement go.

So the code-branch route is for a bug that breaks a requirement or scenario as written, and the fix's test cites it. Anything else starts with a spec branch that adds the requirement or scenario, usually small and landing on checks, and the code fix follows citing it. `propose` and `clm draft`'s prompt say so; the `spec-auditor` flags a test whose cited rule doesn't state what it checks. Judging that is reading, not a program's, so it advises (`deterministic-enforcement`).

The same day showed the other half: `web/loading-indicator` and `web/graph-fills-height` had requirements, but nothing in the human layers asked for them, so the re-founding cut them as tracing to nothing. `ui-quality` in `ARCHITECTURE.md` now names them under `#### Enforced by`, which puts their removal under review.

Rejected: a gate rule refusing a `Resolves:` code fix whose test cites no requirement added on the branch (most real bugs do break a rule as written, and a program can't tell which).

(Amended 2026-11-07, `20261107-harness-only-capability-work`: a bug in the capability suite's harness alone is outside this rule. It restores no behaviour a person or test observes, only the Design of the component that builds the capability tests, which holds no requirement of its own, so there is nothing for a test to cite and nothing for a requirement to add. It is fixed on a small-change branch whose commit deletes the item with `Resolves: backlog/<slug>`, never with `clm claim --fix` (`human-layer-proof`).)
