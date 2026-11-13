---
status: applied
resolves: [the-resulting-graph-check-keeps-answered-questions-that-clm]
---

## Intent

Make the graph a pending change is checked against the graph `clm apply` leaves: without the backlog items apply deletes with the change. Today the check keeps them, so a change that answers a question and removes the requirement the question `blocks` fails `clm check`, although applying it would leave a graph that loads.

## Problem

`clm apply` deletes the `question` items a change's `resolves` names (`apply/answered-questions`) and the item its `from` names, unless that is a `human-task` (`apply/promoted-item`). `apply/resulting-graph` builds the graph the change would leave to check it before anything is written, but it copies the whole backlog, "as it is". When one of those items has `blocks` naming a requirement the change removes, the model's graph check reports `model/parse-backlog` on the modelled graph, and both `clm check` and `clm apply` refuse the change. The item would be gone after apply, so the report is false.

Seen drafting `20261113-builder-check-text-lives`: the question `builder-check-text-lives-in-the-propose-skill-file` blocked `plugin/builder-check`, which the change removes. The draft dropped the item's `blocks` line by hand to get through.

No requirement states the behaviour the fix restores: `resulting-graph` says the backlog stays as it is. So the fix is a spec change (`code-fix-needs-a-written-rule`).

## Approaches

- **Leave out only the `question` items in `resolves`**, as the item asks. It fixes the case seen, but `from` has the same fault: a promoted item with `blocks` on a requirement its own change removes fails the same way. Rejected as half the rule.
- **Leave the items out only in the check, as for `workaround` flags.** `resulting` would then return a graph holding items the apply deletes, and `check`'s rules that read it would have to know. Flags are left out for a different reason: they stay in the backlog after apply, and only their version is unknown. Rejected.
- **Chosen: `resulting` returns the graph without every item apply deletes, from one function `clm apply` also calls.** The modelled graph and the apply can't drift, and a later rule that makes apply delete another kind of item changes both at once (`reuse-first`).

## Outcome

- `apply/resulting-graph-call`: `resulting(graph, change)` leaves out the `question` items the change's `resolves` names and the item its `from` names unless that is a `human-task`. The Design says `clm apply` and `resulting` take those items from one function.
- `apply/resulting-graph`: the graph validation checks is that graph, so a violation only such an item causes is no longer reported, in `clm check` and in `clm apply`.
- `check`'s two callers of `resulting` (`removed-names-before-apply`, `undeclared-imports-before-apply`) read no backlog item, so nothing in `check` changes.

## Constraints

- A resolved item of another kind (`bug`, `idea`, `deferred`, `human-task`) stays in the backlog until its work is done, so its `blocks` on a removed requirement is still reported: the scenario `backlog blocks a removed requirement` stays, and `resolved bug blocks a removed requirement` states it.
- `apply/loaded-graph` needs no edit: a violation of the loaded graph on an item the apply deletes is one the graph the change leaves no longer has, so by its text as written the change fixes it. The scenario `answered question already blocks no requirement` of `resulting-graph` states it.

## Edge cases and workarounds

None known.

## Capability tests

No capability test sees something different: the change removes a false refusal that no capability test exercises.
