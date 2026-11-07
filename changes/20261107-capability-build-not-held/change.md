---
status: applied
resolves: [capability-build-not-held-by-a-behind-serving-rule]
---

## Intent

Hold a pending capability back in the queue while a rule serving it has changed (`behind`) and can't be rebuilt yet because something blocks it, so no hand-written backlog item has to hold it. Today `clm pending` holds a capability only on serving rules that were never built (`serving-rules-order-capabilities`): when a change rewrites a capability's entry so that its test exercises a rule the change also edits, and that rule's `#### Needs` wait on unbuilt work elsewhere, the capability's build is offered anyway and either stops at the capability or re-cites a test that still proves the old text (GitHub issues #249 and #250).

## Problem

A capability can hold no `#### Needs`, so the queue derives its wait from the rules that serve it. It counts only `unbuilt` ones, on purpose: a `behind` rule has been built and only changed, so a capability whose test already runs shouldn't wait for an unrelated edit to an old rule. But the case in the issues is a related edit: the change edits the entry and the rule together, the rule waits on other work, and nothing holds the capability, except a backlog item written by hand that `blocks` it and that the change bringing the rule up to date lists in `resolves`. That is the hand-made hold the decision set out to remove.

## Approaches

- *Wait on every `behind` serving rule.* Closes the gap and the race below, but holds a pending capability back for every unrelated edit to an old rule that serves it, the cost `serving-rules-order-capabilities` avoided; broad capabilities here (`build-from-spec`, `prove-by-tests`) are served by whole components, so this would hold them often. Rejected, as the item asks.
- *`#### Needs` on capability entries.* Already rejected by `serving-rules-order-capabilities`: extra syntax in the product manager's file that must be kept in step with the rules.
- *Tell related edits from unrelated ones* (a rule and an entry edited by the same change). An entry's edit is a hand edit to `CAPABILITIES.md`, not a delta, so nothing in the graph links it to the change; rejected as needing new bookkeeping (`yagni`).
- *Wait on a `behind` serving rule while it is itself blocked (chosen).* A changed rule that can be built now holds nothing, so an unrelated edit to an old rule never holds a capability back; one that can't be built yet holds the capability only as long as the rule itself is held, and the capability's blocker chain shows why. It reuses what `pending` already computes (a rule's `blocked_by` and `waits_on`), and the capability's priority flows to the rule and through it to the work it waits on, unchanged in the queue (`priority-orders-nodes`).

## Design

- `check.coverage/pending-report`: a pending capability is blocked, in `waits_on`, by each serving rule that is `unbuilt`, or `behind` and blocked by an item or a wait of its own; by no current rule and no `behind` rule that nothing blocks. A rule's blockers never include a capability (a rule is never an entry, and a `Needs` can't name one), so `pending` computes the rules' blockers first and the capabilities' from them, in the same pass.
- `orchestrator/priority-orders-components`: the wait list it seeds through now includes such a rule, so an urgent capability pulls forward the work a changed serving rule waits on. Its code reads `waits_on` and needs no change beyond the new scenario's test, which `#### Needs` `check.coverage/pending-report`.
- `capability`'s Design and `orchestrator.run/capability-waits-for-changed-rule` (added): the build of the capability tests that finds a test can't pass because a ready `behind` rule hasn't been rebuilt stops and leaves the capability pending, filing nothing; the build prompt says so, since the method is written where sessions read it (`method-in-the-plugin`).
- A `behind` serving rule counts as blocked when an item names it or any requirement is in its own `waits_on`, whatever that requirement's state and whatever blocks it in turn (settled from the builder check).
- Hand edits: `needs-alone-order` in `ARCHITECTURE.md`, `docs/spec-format.md`'s building section, and the decisions `serving-rules-order-capabilities` (amended) and `needs-alone-order` (its refinement line).

## Edge cases and workarounds

- A changed serving rule that nothing blocks and the capability that exercises it can be offered at once. A build that takes the capability first finds its test can't pass, stops at it and leaves it pending; the rule's build is ready, so the capability's next build finds it current. Accepted, as `needs-alone-order` accepts a build stopping for missing work.
- A `behind` serving rule held by something unrelated to the capability's edit holds the capability too, for as long as the rule is held. Accepted: the wait shows in the blocker chain, and the capability's priority flows to what holds the rule.

## Constraints

`serving-rules-order-capabilities`, `needs-alone-order`, `priority-orders-nodes`, `human-layer-proof`, `optional-nodes`, `yagni`, `reuse-first`.

## Open questions

- The two changes folded here met in `orchestrator`'s Design: this change restates it whole, so it also carries the place-file `slot` line and the slot choice under the lock that `orchestrator/place-slots` (change `20261107-parallel-builds-test-runs`) requires (architecture audit finding; `spec-is-current`). Land them together.
- `orchestrator/priority-orders-components` is `behind` and its code is said to need no change beyond the new scenario's test, which waits on `check.coverage/pending-report` (its `#### Needs`).
- Left to the builder (builder check): how the capability-tests prompt paragraph recognises its component (reuse `prompt-serves`' predicate), and whether it also covers a test that passes but proves old text.
