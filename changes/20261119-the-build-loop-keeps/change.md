---
status: applied
resolves: [the-build-loop-keeps-printing-a-draft-launch-whose-only-item-a]
---

## Intent

The build loop no longer prints, step after step, the launch of a draft whose items have all left `main`. Before it reads the snapshot, each step finishes every draft it printed that no subagent has started (a `planned` run, or an orphan of a dead session) when `origin/main`'s backlog holds none of the draft's items and `origin` has no spec branch for it: it runs `clm draft --finish`, which frees the place and releases what is left of the claims, forgets the run, and prints nothing for it. So a draft whose bug a build fixed meanwhile no longer keeps the loop from draining, and no session is spent only to find its items gone. The fix is outside the plan's freeze, so a question item holds its build back until the human says whether to make the exception.

## Problem

A build's review filed a `kind: bug` item, and the loop printed a `launch draft:<id>` for a group holding only that item. The launch was held while a build of the same component ran; that build fixed the bug, deleted the item with a `Resolves:` trailer and landed. Every later `clm build step` and `clm build returned`, with the checkout at `origin/main`, still printed the same `launch draft:<id>`, so the loop couldn't drain; no `propose/*` claim was left on `origin`.

The design is wrong, not the code: `step-actions` prints a `planned` run again, unchanged, until it is reported launched, and nothing in the loop looks again at a draft's items once `clm draft` has prepared it. Only the draft's own prompt checks each item against `origin/main`, after a session is spent. No requirement states that the loop drops such a draft, so the fix is a requirement of this change (`code-fix-needs-a-written-rule`).

No change or code fix since 2026-11-12 fixes it. `20261117-the-build-loop-drafts` (`draft-waits-for-its-build`, blocked by the plan's freeze) holds back a draft not yet prepared while its component's build is held; it makes the reported sequence rarer but doesn't touch a draft already printed, and a draft whose items another clone, a build printed after it, or the human resolves still arises.

## Approaches

- **Taken: the step finishes the draft once its items are gone.** The step already finishes builds whose claim has left `origin` before it reads the snapshot (`claim-gone-ends-a-build`), and already reads `origin/main`'s `backlog/` tree for the human's queue, so the rule needs no new input and no new command (`reuse-first`); `clm draft --finish` frees the place and releases the claims, so the place's and claims' rules stay `orchestrator.triage`'s. It applies only to drafts no subagent holds: a `planned` run, and an orphan, whose session is gone.
- **Taken: a pushed spec branch keeps the run.** A draft that pushed `spec/<change-id>` may hold work (a deleted item, a reported defect) a resumed draft lands; its prompt handles items gone. The loop reads the remote-tracking ref only to tell that the branch exists.
- **Taken: every item must be gone.** A draft with one item left has design work; its prompt drops the others as already fixed.
- **Not taken: dropping a draft when `clm triage` no longer prints its group.** The draft's own claims keep its items out of every group, so the test would drop every printed draft.
- **Not taken: dropping a paused draft.** A pause is the human's word (`pause`), and a paused run doesn't keep a drain from ending; once `clm build pause --end` makes it `planned`, the next step drops it.
- **Not taken: leaving it to the draft's prompt.** That spends a whole session per such draft, and a loop whose session holds the launch back can't drain meanwhile.
- **Not taken: accepting the defect,** the plan's default answer while the freeze holds. Each occurrence spends a draft session or stalls a drain, and every remaining phase is built through the loop; whether to build it now is the human's, so a question item asks.

## Outcome

- `orchestrator.loop/draft-of-gone-items-dropped` (new): each command that prints the actions to take now finishes, after its fetch, each `planned` or orphan draft none of whose items `origin/main` holds and with no `origin/spec/<change-id>`, with `clm draft --finish`, takes it off the list keeping its task ID by name, and prints nothing for it.
- `orchestrator.loop/step-actions` (modified): a `planned` run is printed again until reported, save such a draft.
- `orchestrator.loop`'s Design gains the paragraph *A draft whose items are gone*.
- `backlog/the-fix-for-a-draft-whose-items-are-gone-is-outside-the-freeze` (`kind: question`) blocks the two requirements until the human answers.
- No capability test sees a difference: `capability/build-from-spec` drains components, not drafts.

## Constraints

- `clm draft --finish`, `clm triage` and the loop file's format are unchanged; a dropped run leaves the list as a released build does.

## Edge cases

- **A draft whose subagent was launched and not yet reported.** The session reports `clm build launched` at once after launching; should a step run between, the run is dropped, `launched` then finds no run of its name and refuses, and a late return is ignored as for any run not listed (`returned-ignores-an-earlier-task`). The subagent's prompt finds its items gone and stops. Accepted.
- **An item renamed rather than deleted.** The draft is dropped; the renamed item reaches a later `group` line.

## Open questions

None.
