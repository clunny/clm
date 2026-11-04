---
title: "A code PR that fixes a bug deletes its item, named by a commit trailer"
date: 2026-10-25
status: accepted
---

A bug whose fix changes no spec, such as a wrong test or code that falls short of requirements it already has, took two PRs: a code PR for the fix, and a spec PR holding a change with nothing to build, archived in its own PR (`archive-in-spec-pr`) only so its `resolves` could delete the item. A code PR couldn't delete the item itself: `gate/backlog-additions` exempted a deletion only when a change merged earlier listed the item, and adding a `change.md` to the code PR made it `mixed`.

So a commit on the code PR's branch may name the item with the trailer `Resolves: backlog/<slug>`, and the gate passes the deletion when the item at the merge base is `kind: bug` and the PR's code changes a file under the `paths` of a node the item names. `propose` builds such a fix in its own session, since `clm run` builds only pending requirements and this fix has none. It claims the node with `clm claim <node> --fix backlog/<slug>`, which claims a node even when it is converged, the usual state of a node whose bug needs no spec change. Only an `impl/<node>` branch works: CI deletes it after merge, `clm checkpoint` treats it as a build, and the queue holds the node while it lives. The flag claims for a named bug on that node and nothing else, so it can't become a general way around the queue's refusals; a hand-pushed `impl/<node>` would have copied the claim's logic and bypassed its refusal.

This lets a branch name and delete an item in one go, which `resolves` deliberately doesn't: `resolves` is read at the merge base so only a reviewed change can close an item. The trade is bounded. Deleting a bug item is an `AUTO` edit, merged on checks with no human review either way; only bugs qualify, not ideas, questions or human steps, whose closing is a decision; and the code must touch a node the bug names, so a PR can't close bugs it never went near. Whether the fix fixes the bug is left to its tests, as for any code PR (`ai-writes-code`). The trailer lives in a commit, so the evidence is in git (`git-is-truth`), not in a PR body.

Alternatives considered:
- A new archived change with no deltas in the code PR, its `resolves` naming the item. It reuses the `resolves` machinery and keeps a written record, but puts an unreviewed `change.md` in a code PR, which `gate/classify` exists to keep out.
- Keeping two PRs and only saying so in `propose`. Cheapest, but leaves the cost the defect reported.

(Amended 2026-10-31, `20261031-a-build-cannot-delete`: `kind: deferred` items qualify too, under the same bounds, so a build that does deferred work, such as removing code kept for another node, deletes its item in the same branch (`deferred-work-resolves-in-code`). Ideas, questions and human steps still don't.)

(Amended 2026-11-02, `20261102-clm-claim-fix-refuses`: `clm claim <node> --fix backlog/<slug>` takes a `kind: deferred` item as it takes a bug, so deferred work that changes no spec and lies on a converged node has a claim to land from; the gate already passed its deletion. A draft sends such an item down the same fix-by-node route as a code-only bug. Ideas, questions and human steps are still refused.)
