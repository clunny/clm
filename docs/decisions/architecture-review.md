---
title: "Architecture requirements are reviewed like capabilities, in a view of their own, costed by the rules that enforce them"
date: 2026-11-01
status: accepted
---

Builds the architecture half of `review-capability-cost` (`changes/20261101-architecture-review/`). Brainstormed with the human on 2026-11-01. The web app showed no `ARCHITECTURE.md` entry anywhere, and the human wants to see both whether each one is actually held and what it costs, and to decide on it as on a capability.

- **The same review as a capability, plus its proof.** Each entry shows its slug and statement, whether it is held (each enforcing rule current or pending, and its `#### Untested` text), its cost, its unsettled workarounds, "can we live without it?" and its latest decision, with the same value and Keep, Cut, Redesign and Accept. The proof is what a capability has no counterpart for, and the question an architect asks first.
- **Its own top-level view**, Architecture, after Capabilities in the sidebar and the tab bar (`Arch.` there, six tabs), not a tab inside a shared Review view: the human's choice, so each list is one tap away.
- **Cost is what its `#### Enforced by` rules cost**, each rule's part split equally over the entries that name it, as a second split of the same `effort` records, counted apart from the capabilities': the architecture list adds up to what the enforcing rules cost, never to the total, and the views say so. An entry holding only `#### Untested` costs nothing, since nothing is built for it.
- **Workarounds are the friction items on its enforcing rules**, settled per entry by slug as on a capability (`effort/unsettled-flags`). `clm friction`'s redesign threshold (`redesign-before-workaround`) still counts per capability only.
- **`decided` records name the entry in their existing `capability` key**, `capability/<slug>` or `architecture/<slug>`, so every ledger written so far and every `clm` already installed read them alike.
- **The CLI first, the web app showing it** (`deterministic-enforcement`): `clm architecture [<slug>]` and `clm architecture decide` beside `clm capabilities`, the guard asking the human about `decide` as it does for capabilities (`approval-policy`), and the Worker serving `clm_effort::report`'s `architecture` list. Whether a rule is current comes from the caller (`clm coverage`, or the Worker's marker index, `None` while it is indexing), so the pure part stays free of `clm-check` (`effort-builds-for-wasm`).

Rejected:
- **Cost by `serves`**: a rule's effective serves is loose about architecture, often inherited from its node's list (37 rules serve the judgement `reuse-first`), so judgement entries would look expensive; `#### Enforced by` is the link the architect reviews.
- **One table of capabilities and architecture**: the columns differ, and one ranking would count the same work twice.
- **Proof only, cost and decisions later**: half of what the human asked for.
- **Renaming the `capability` key to `entry`**: older ledgers and installed `clm`s would stop reading decisions, for a name only the JSON shows.
- **The panel's Architecture tab in this change**: the human asked for the web app; it is `backlog/panel-architecture-tab`.
