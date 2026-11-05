---
title: "Work a requirement relies on from another node is that node's requirement, added in the same change and named in a Needs"
date: 2026-10-26
status: accepted
---

`node-reconciliation` makes a node pending on requirements only: a Contract or Design edit alone builds nothing. So when a change makes one node's requirement rely on new data or behaviour from another node and writes that work only into the other node's Design, the other node's build has nothing pending for it, leaves it undone and counts as settled. The queue then offers the dependant, whose build can't edit the other node's paths and dead-ends on the requirement. That happened with `20261025-forge-adapters`: `web/activity-feed` and `web/implementation-view` needed lands and approvals from the ledger fold in the Worker's response, which only `web.worker`'s Design described, and `web`'s build could only raise a `question` item until `20261026-ledger-backed-views` added `web.worker/implementation-ledger` and the `#### Needs` lines.

Now the spec author owns it. A change that makes a requirement rely on work another node does adds or modifies a requirement of that node stating the work, so that node is pending and its build does it, and gives the relying requirement a `#### Needs` line naming it (`requirement-needs`), so the queue holds the relying requirement until the work is current. Both spec-writing paths say so where they already ask for needs: `propose`'s builder check, whose builder is asked to name such work, and the prompt `clm draft` prints.

This answers the case `node-reconciliation` left open ("reopen if Design edits that should drive refactors turn out common") without reopening it: the edit that should drive work becomes a requirement, which is the unit the queue already reads.

Refines `requirement-needs` and `blocked-by-backlog`.

Rejected:
- **Design-only changes making a node pending.** `node-reconciliation`'s reasons hold: a node's pending state would no longer be read from markers alone, and nothing would say when a Design edit is built.
- **A check that flags a requirement naming data no dependency's requirement provides.** It needs to know which words in a requirement are data and which requirement provides them; that is the heuristic `requirement-needs` rejected for paths. The builder check reads the change as its builder would, which is where the gap shows.
- **Leaving it to the build's `question` item.** That is today's behaviour: it works, but costs a dead-end build and a second spec PR for each case.

(Amended 2026-11-05, `20261105-capability-harness-move-has`: work in the `capability` component that a rule makes necessary, by changing what a capability test observes, is not a requirement, since that component holds none of its own: the rule's own build updates those tests (`capability-tests-follow-rules`), and the change names them in the rule's text.)
