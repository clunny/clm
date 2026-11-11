---
status: proposed
---

## Intent

Slim the ten components `clm health` flagged on 2026-11-11 (`host` and `capability` for their text; `check.coverage`, `orchestrator.checkpoint`, `plugin.panel`, `sync.regenerate`, `view`, `view.repo`, `web.client-live` and `web.worker.platform` as size outliers). Each component's Design and Contract lose the sentences that restate a requirement or describe what the code shows, and scenarios that another one fully covers are dropped or merged. Nothing a person or another component relies on changes. `host`'s two "as before" phrases go with the paragraphs that held them.

## Problem

`clm health` reported `history-words` on `specs/host/SPEC.md:34` and `:36`, `design-heavy` on `capability` (1193 Design tokens against 104 for its Contract and requirements), and `outlier-size` on the other eight. A `spec-slimmer` pass on each found the same causes the 2026-11-09 pass (`20261109-slim-restated-specs`) found: Designs that restate their requirements, Contracts that list what the code shows, and scenarios that prove the same rule twice.

## Approaches

- **A. Drop the source-scan requirements too.** The slimmer proposed removing `host/shared-plumbing`, `view.repo/shared-plumbing` and `sync.regenerate/git-through-layer` as lints. Rejected: they are how `shared-plumbing-in-git` and `remote-plumbing-scan` are proven, crate by crate, and `host/git-through-layer` enforces `host-adapters`. Removing them would leave those decisions without a test (`proven-by-tests`, `deterministic-enforcement`). Their prose is trimmed instead.
- **B. Cut restated text and duplicate scenarios across all ten, each cut checked against the spec and code (chosen).** The human chose B.

## Outcome

<!-- filled in once the deltas are written -->

## Constraints

- No behaviour changes: every scenario dropped is covered by one kept, and every rule cut is stated elsewhere.
- No requirement or Contract part that another component's `#### Needs`, an `#### Enforced by` block or a decision names is removed or renamed.
- `orchestrator.checkpoint` and `check.coverage` were slimmed by `20261109-slim-restated-specs`; text that pass kept as rationale stays. Their flag is code per requirement, which merging requirements would only raise.
- Code no requirement needs, which the slimmer found in `web.worker.platform`, `web.client-live` and `view.repo`, is not this branch's: it goes to a `kind: deferred` item for a code branch.
