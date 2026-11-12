---
status: applied
---

## Intent

Slim the ten components `clm health` flagged on 2026-11-11 (`host` and `capability` for their text; `check.coverage`, `orchestrator.checkpoint`, `plugin.panel`, `sync.regenerate`, `view`, `view.repo`, `web.client-live` and `web.worker.platform` as size outliers). Each component's Design and Contract lose the sentences that restate a requirement or describe what the code shows, and five requirements of `host` and `plugin.panel` lose text that restated another rule or said "as before". No scenario is dropped and nothing a person or another component relies on changes. `host`'s two "as before" phrases go with the paragraphs that held them.

## Problem

`clm health` reported `history-words` on `specs/host/SPEC.md:34` and `:36`, `design-heavy` on `capability` (1193 Design tokens against 104 for its Contract and requirements), and `outlier-size` on the other eight. A `spec-slimmer` pass on each found the same causes the 2026-11-09 pass (`20261109-slim-restated-specs`) found: Designs that restate their requirements, Contracts that list what the code shows, and scenarios that prove the same rule twice.

## Approaches

- **A. Drop the source-scan requirements too.** The slimmer proposed removing `host/shared-plumbing`, `view.repo/shared-plumbing` and `sync.regenerate/git-through-layer` as lints. Rejected: they are how `shared-plumbing-in-git` and `remote-plumbing-scan` are proven, crate by crate, and `host/git-through-layer` enforces `host-adapters`. Removing them would leave those decisions without a test (`proven-by-tests`, `deterministic-enforcement`). Their prose is trimmed instead.
- **B. Cut restated text and duplicate scenarios across all ten, each cut checked against the spec and code (chosen).** The human chose B.

## Outcome

Each cut the slimmer proposed was checked against the spec and the code first. Most scenario merges it proposed were rejected: each pair proved a different trigger, entry point or requirement.

- `capability`, `sync.regenerate`, `view`, `view.repo`, `web.client-live`, `web.worker.platform`: Design (and for `view.repo` Contract) only. No requirement changes, so nothing is rebuilt. `web.worker.platform`'s "One way down" now points at `web.worker`'s Design, where the rule is.
- `host`: the Design loses the restated paragraphs, the history sentence about `clm pr-body`, both "as before" phrases, and a wrong claim that `issue` prints nothing on `git` and `none`. `shared-plumbing` keeps its scan but names the one kind of message that may still say `origin` instead of restating `branch-delete`; `pr-open-defaults` loses a closing sentence about its own build. Their tests are re-cited.
- `plugin.panel`: the Design loses its restatements. `blocker-chains` and `superseded-draw` say what is drawn where they said "as before"; `build-opens-panel` loses two clauses other text already forces. Their tests are re-cited.
- `check.coverage` and `orchestrator.checkpoint` get no delta: every proposed cut was text `20261109-slim-restated-specs` kept on purpose, or a scenario that proves something no other does.
- `backlog/platform-fake-dead-helpers` (`kind: deferred`) carries the fake's unused helpers and fields, which `20261109-slim-flagged-specs` meant to remove but, being Design-only, never built.

## Constraints

- No behaviour changes: every scenario dropped is covered by one kept, and every rule cut is stated elsewhere.
- No requirement or Contract part that another component's `#### Needs`, an `#### Enforced by` block or a decision names is removed or renamed.
- `orchestrator.checkpoint` and `check.coverage` were slimmed by `20261109-slim-restated-specs`; text that pass kept as rationale stays. Their flag is code per requirement, which merging requirements would only raise.
- Code nothing uses, found only in `web.worker.platform`, is not this branch's: it goes to a `kind: deferred` item for a code branch. What the slimmer flagged in `web.client-live` and `view.repo` is used code whose behaviour no requirement states (the switch's shared `ask`, its "try again" messages, `checkout not compared` on stderr); whether to state it or cut it is a behaviour question, left open.

## Open questions

- `web.client-live`: `Notify.ask` (one subscription request shared by the switches drawn at once), `turnOff`'s fallback to the remembered endpoint and the "Not turned on/off: try again." messages on a thrown error are in no requirement; the backoff's reset after a minute open is required but has no scenario. State each or cut it?
- `view.repo`: `checkout not compared: <e>` on stderr is in no requirement. State it or fold the error into no checkout line?
