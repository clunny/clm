---
status: applied
resolves: [a-draft-s-left-items-never-reach-the-loop-s-list]
---

## Intent

`clm draft --finish <change>` prints `left <slug>: <change> doesn't resolve it` for each item the draft claimed that is still in `origin/main`'s `backlog/` and that the draft's change doesn't resolve. So the items a draft left reach the build loop's list of them by the route `orchestrator.loop/return-rules` already states, which records each `left <slug>: <reason>` line a draft's finish prints, and the loop stops drafting them again. Nothing in `orchestrator.loop` changes, and the held `plan-order` is not edited.

## Problem

`return-rules` says the loop records each `left <slug>: <reason>` line a draft's finish prints among the items drafts of the loop left, and `crates/clm-loop/src/returned.rs` reads them from `clm draft --finish`'s output. But the finish prints no such line: `left <slug>:` comes only from `clm draft` when a claim is refused, and from the draft's subagent in its final message, of which the session reports only the last line to `clm build returned`. So a draft's left items reach the loop's list only through an escalation, `plan-order` never skips them, and the loop drafts each again at its next step, spending a session to leave it again.

No change or code fix since 2026-11-15 fixes it: the loop's and triage's changes of that time (`20261116-the-build-loop-shows`, `20261117-a-deleted-runs-task`, `20261117-the-build-loop-drafts`, `20261122-a-deferred-item-that`, the two wave-4 changes and those archived) change the queue, task IDs, the draft's wait for its build and the requirements' wording, and none makes the finish print a left item.

## Approaches

- **Taken: the finish works out which items the draft left, from what is on `origin`.** It already reads the draft's claims from its place (`propose/<slug>` lines) and a waiting change's `resolves` from `origin/spec/<change>` (`moved_items`). A draft names every item in `resolves` and pushes that `change.md` first, then takes out of `resolves` each item it leaves; an item it fixes on a code branch or accepts is deleted from `origin/main`. So an item the place names that `origin/main` still holds and that the change's `resolves` doesn't name is one the draft left. The rule is code with a test, read from git, in the command the loop already runs on every draft's return (`build-loop-in-clm`, `deterministic-enforcement`, `reuse-first`), and `return-rules` holds as written.
- **Not taken: the session passes the draft's `left` lines to `clm build returned`.** It needs a new argument and a sentence in the `build` skill, and leaves copying lines out of a subagent's report to a model, which is the rule-in-a-model's-hands the loop exists to avoid (`build-loop-in-clm`); a report the harness doesn't deliver loses them, as `claim-gone-ends-a-build` records.
- **Not taken: `return-rules` stops claiming that a draft's finish prints them.** The loop would then redraft every left item at each step until the loop ends, and `plan-order`'s skip of a left item, which the plan holds, would never fire.

The reason the line gives is the finish's own (`<change> doesn't resolve it`), not the draft's: the loop records only the slug, and the triage skill still reads the draft's own `left` lines, with their reasons, from its report.

## Outcome

- `clm draft --finish <change>`, with the place `<change>` there, prints `left <slug>: <change> doesn't resolve it`, after its `released` lines and before `freed <change>`, for each item the place claims that `origin/main`'s `backlog/` holds and that the change's `resolves` doesn't name; the `change.md` is read from `origin/main` (live or archived) when it is there, else from `origin/spec/<change>`, and with none, every such item is printed. With no place it prints no `left` line, so a second finish of the same draft (after a fold lands, `land:` returns) records nothing again.
- `clm build returned` for a draft records those slugs as it already does (`returned.rs`); `finish_gone_drafts` prints nothing new, since a draft it finishes has none of its items on `origin/main`.

## Edge cases

- A draft whose return closes with no line, or with `stopped:` before it pushed its `change.md`, has every claimed item still on `main` and no `resolves`: each is printed as left, and the loop drafts none of them again this loop. A draft stopped with a cause (`context`, `limit`, `stopped` and the rest) is resumed, not finished, so this happens only for a draft that ended without a spec branch, whose items a redraft would most likely leave again; the next loop or `/clm:triage` takes them.
- An item whose claim has lapsed and that another session claimed again is printed too, whether its claim is released or kept: the loop skips a group whose item a claim holds anyway.
- An item whose `components` the draft corrected (`left <slug>: components corrected to <components>`) is printed as left, as the draft's own line already names it, so this loop doesn't draft it on its corrected components; the next loop or `/clm:triage` does. That is what `return-rules` states for any `left` line a draft's finish prints; see the open question.

## Open questions

- The draft prompt says the next `clm triage` drafts an item whose components the draft corrected; under the loop that waits for the next loop. Telling such an item apart (its `backlog/<slug>.md` changed by the change) is possible while the change waits on `origin/spec/<change>`, but not once it has landed and that branch is gone, so the finish doesn't try (`yagni`). If the human wants the loop to redraft such items in the same run, that is a change to `return-rules` or `plan-order`, which the plan holds.
- This modifies `orchestrator.triage/draft-finish`, which wave 4's build of `orchestrator.triage` re-cites: landing this before that build lands moves the requirement's version under it, which its `clm rebase` picks up as `behind`. If the fold batch holding this comes up for approval before both wave builds have landed, the human may prefer to wait.
