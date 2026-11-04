---
title: "Two human layers, CAPABILITIES.md and ARCHITECTURE.md, above the node specs; every rule names what it serves"
date: 2026-10-29
status: accepted
---

From the re-founding (`docs/the-clm-project.md`, `changes/archive/20261029-refounding/design.md` §1 and §2). Humans reviewed hundreds of machine-level rules instead of what the product is for, and nothing tied a rule to a purpose, so nothing was ever cut. Two small layers now sit above the node specs, owned by people:

- **Capabilities**, in `CAPABILITIES.md` at the root, owned by the product manager: what a person can do with the product, each with a `#### Why` and at least one scenario seen from outside.
- **Architecture requirements**, in `ARCHITECTURE.md` at the root, owned by the architect: what must always hold, each with `SHALL` or `MUST` and at least one scenario. They are the old invariants, the platform, the principles the re-founding added and the decisions that carry real constraints, each kept to a short statement that names the decision or node spec holding its mechanism.

**Format.** Each file reuses the requirement-block format: a short preamble, then `## Requirements` of `### Requirement: <slug>` blocks (`## Capabilities` of `### Capability: <slug>` blocks). The model reads only those blocks; a file that is absent, or has no such section, has no entries (`model/human-layers`). The format is in `docs/spec-format.md`, "The human layers".

**IDs and versions.** An entry's ID is `architecture/<slug>` or `capability/<slug>`, and its version is the first 8 hex digits of its block's content hash, as for a rule. Tests cite a capability as `req:capability/<slug>@<version>`, and coverage, `clm pending`, dangling markers and `clm hashes architecture|capability` treat capabilities like rules. `architecture` and `capability` are reserved: no node may take either ID (`model/reserved-ids`). An entry's slug is shared, like a decision's, so a node spec may name it bare.

**How the entries are proven** is `human-layer-proof`: a capability by black-box tests of the node `capability`, an architecture requirement by the rules its `#### Enforced by` names, never by a test that cites it.

**`serves`.** A node's frontmatter `serves` lists the entries its rules serve; a requirement whose purpose differs overrides it with one `#### Serves` block of `- <id>` lines. A rule's effective serves is its own block if it has one, its node's `serves` otherwise. The IDs carry no versions, so rewording a capability makes no rule stale. `clm context <node>` adds the capability blocks the node's rules serve. `check/untraced`, which fails a rule that serves nothing, comes once every rule is mapped (`traced-rules`).

**Edits.** Any edit to either file needs review (`humans-review-intent`). The node map, the change flow and the glossary that `ARCHITECTURE.md` used to hold moved out, to the root node's Design and to the guide's reference page, because adding or splitting a node must not need a human.

Rejected: one file per entry (the layers are small, concurrent edits to different entries merge cleanly, and `ARCHITECTURE.md` was already read first and prepended to every bundle); a version in `serves` (a reworded capability would make hundreds of rules stale); a stored value rating or "can we live without it?" per capability (the first is a review decision kept in the ledger, the second would go stale).
