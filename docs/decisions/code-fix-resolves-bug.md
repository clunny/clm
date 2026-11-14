---
title: "A code PR that fixes a bug deletes its item, named by a commit trailer"
date: 2026-10-25
status: accepted
---

A bug whose fix changes no spec, such as a wrong test or code that falls short of requirements it already has, took two PRs: a code PR for the fix, and a spec PR holding a change with nothing to build, archived in its own PR (`archive-in-spec-pr`) only so its `resolves` could delete the item. A code PR couldn't delete the item itself: `gate/backlog-additions` exempted a deletion only when a change merged earlier listed the item, and adding a `change.md` to the code PR made it `mixed`.

So a commit on the code PR's branch may name the item with the trailer `Resolves: backlog/<slug>`, and the gate passes the deletion when the item at the merge base is `kind: bug` or `kind: deferred` and the PR's code changes a file under the `paths` of a node the item names. A build that does deferred work, such as removing code kept for another node, deletes its item in the same branch this way (`deferred-work-resolves-in-code`).

`propose` builds a fix that changes no spec in its own session, since `clm run` builds only pending requirements and this fix has none. It claims the node with `clm claim <node> --fix backlog/<slug>`, which claims a node even when it is converged, the usual state of a node whose bug needs no spec change. The claim takes a `kind: deferred` item as it takes a bug, so deferred work that changes no spec and lies on a converged node has a claim to land from, and a draft sends such an item down the same fix-by-node route as a code-only bug. Only an `impl/<node>` branch works: it is cleaned up after it lands and `clm wrap-up` treats it as a build's, and the queue holds the node while it lives. The flag claims for a named bug or deferred item on that node and nothing else, so it can't become a general way around the queue's refusals; a hand-pushed `impl/<node>` would have copied the claim's logic and bypassed its refusal.

This lets a branch name and delete an item in one go, which `resolves` deliberately doesn't: `resolves` is read at the merge base so only a change already on `main` can close an item. The trade is bounded. Deleting such an item lands on checks with no human review either way; only bugs and deferred items qualify, not ideas, questions or human steps, whose closing is a decision, and both the gate and `clm claim --fix` refuse those; and the code must touch a node the item names, so a PR can't close items it never went near. Whether the fix fixes the bug is left to its tests, as for any code PR (`ai-writes-code`). The trailer lives in a commit, so the evidence is in git (`git-is-truth`), not in a PR body.

Alternatives considered:
- A new archived change with no deltas in the code PR, its `resolves` naming the item. It reuses the `resolves` machinery and keeps a written record, but puts an unreviewed `change.md` in a code PR, which `gate/classify` exists to keep out.
- Keeping two PRs and only saying so in `propose`. Cheapest, but leaves the cost the defect reported.
