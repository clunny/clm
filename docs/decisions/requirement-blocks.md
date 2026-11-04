---
title: "A backlog item blocks the requirements it names, and a blocked requirement is left out of its node's work"
date: 2026-10-23
status: accepted
---

`blocked-by-backlog` made a `question` or `human` backlog item block a *task*: the task's status file named the item, and the whole task, and every task that waited on it, stood still until the item was deleted. `pause-affected-tasks` did the same for a spec change a task found it needed. With building per node (`node-reconciliation`), a block that stopped the whole node would stop every node above it in the graph for one unanswered question.

Now the item carries the block. `backlog/<slug>.md` has `blocks: [<node>/<slug>, ...]`, the requirements that can't be built until the item is resolved. A build session that hits something only a human can supply adds the item with `blocks` and builds the rest. The queue reads it each time:

- a blocked requirement stays pending but is left out of its node's work, and out of its node's priority;
- a node whose every pending requirement is blocked is *settled*, like a converged one, so its dependants proceed;
- a node is `blocked` only when nothing else on it can be built, and `clm next`, `clm claim` and the views name the items.

Deleting the item, the human's answer in a small spec PR as before, releases every requirement it blocked at once. Adding a requirement to an existing item's `blocks` is a code-tier edit (`gate/backlog-additions`), so a later build can record that another requirement waits on the same answer; removing one is a spec edit, since it says the question no longer applies.

There are no deadlocks to report. `report-block-cycles` existed because a blocked task could hold up the very change that would delete its item; a blocked requirement holds nothing up, so the cycle can't form.

Blocks come only from backlog items. An unattended session can't raise a spec change, and a contract change it finds it needs is a `question` item for the human, who proposes the change.

Supersedes `pause-affected-tasks` and `report-block-cycles`. Refines `blocked-by-backlog`: the item blocks requirements rather than a task, and the rest of the node is built.

Rejected:
- **A per-node status file naming blocked requirements.** A second file for one fact, with its own writer; the item is already the thing being waited for.
- **Blocking the node.** It stops every dependant for one question, which is the stall `prefer_after` used to cause.
