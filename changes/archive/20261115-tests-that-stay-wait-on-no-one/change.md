---
status: done
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`. The seven `question` items the `commands-in-crate` builds filed for tests that stay in the root's files are all answered, but an answered question still shows as waiting on the human until a change deletes it. They become `deferred` items, which is what they are: work that waits on a condition, the build run draining. The root's Design tells the remaining builds to file such an item as `deferred`.

It adds and removes no requirement, so it is archived in its own branch.

## Approaches

- **`deferred` items.** Taken: the answer is given for the whole class, the items keep their `blocks`, and nothing waits on the human.
- **Make the batched spec change now and again later.** Not taken: more builds are still to report, and each batch moves scenarios into the root while builds are editing its files.
- **Leave them as questions.** Not taken: `clm status` then lists as waiting on the human what waits on no one.

## Outcome

- Seven items change from `question` to `deferred` and lose their `choices`; each keeps the human's answer and its `blocks`.
- The root's Design: a build that leaves such a test files a `deferred` item.

## What it removes

Nothing.
