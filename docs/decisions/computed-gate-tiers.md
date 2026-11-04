---
title: "Gate tiers are computed from the diff"
date: 2026-10-22
status: superseded
superseded_by: yes-no-review
---

A spec PR's tier is the highest tier among the things its diff changes, worked out by `clm gate` with fixed rules: HARD for invariants, `CLAUDE.md`, `docs/` other than new decisions, the root node, Contracts, frontmatter, and modified or removed requirements, nodes or decisions; SOFT for added requirements, nodes and decisions; AUTO for a node's Design and for backlog edits. Anything unrecognised is HARD. `change.md`'s `gate` can only raise the tier, and the gate blocks a `change.md` that declares less than its diff needs.

This follows the Spec Growth Engine's "authority follows blast radius and reversibility": the outward surface (contracts, requirements, boundaries) is gated, while a node's inward surface (its Design) changes autonomously. Added requirements are SOFT because they extend behaviour without breaking what others rely on.

Alternatives considered: trusting `change.md`'s `gate` (Claude writes it, so Claude could approve its own spec changes, against `humans-review-intent` and `deterministic-enforcement`); having the spec-auditor or a model judge the tier (model judgment can advise but never gates).
