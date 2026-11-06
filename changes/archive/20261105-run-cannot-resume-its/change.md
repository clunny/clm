---
status: done
resolves: [run-cannot-resume-its-own-pushed-claim]
---

# clm run resumes a claim its own interrupted launch pushed

## Intent

`clm run <component>` records its claim in the local branch `impl/<component>` before pushing it, so a launch interrupted at any point is resumed or claimed afresh by the next `clm run <component>`, without deleting a remote branch and without letting another clone take a live claim (`lease-claims`). `orchestrator` adds the library call `claim_local`, which makes the local branch at the claim commit before the first push and deletes it when the claim visibly failed; `clm run` claims through it, and its leftovers step removes a local branch that holds only an unpushed claim. Decision: `claim-local-before-push`. Resolves issue #148 (`backlog/run-cannot-resume-its-own-pushed-claim`).

## Problem

`clm run` pushed the claim commit to `origin/impl/<component>` and only then made the local branch. Interrupted between the two, the clone kept no trace of the claim: every later `clm run <component>` refused the component as claimed by someone, `clm run --finish` didn't change that, and the only ways on were deleting the remote claim branch, which auto mode blocks as destructive, or waiting out the four-hour expiry.

## Approaches

- **Local branch before the push (chosen).** The local branch is the record of the claim, and it exists before the claim does, so no interruption splits them. After the push, the existing resume rule (the local branch contains `origin/impl/<component>`) resumes it; before the push, the branch's tip is an empty commit on `main` that holds no work, which the leftovers step removes like a landed branch.
- **Recognise one's own claim on `origin`.** The claim commit names no clone, and the clone's build place is no proof: a place can outlive a run interrupted before its push while another clone claims. Rejected, since it could take a live claim from another clone.
- **Delete the remote claim from `--finish` or the leftovers step.** Destructive, blocked in auto mode, and racy against another clone's new claim. Rejected.
- **Make `claim_node` (and so `clm claim`) write the branch.** It would break `clm-run`'s build, which makes the branch itself after `claim_node`, from another component, and change `clm claim`'s Contract. Rejected for an added call.

## Outcome

- `orchestrator/claim-local` (added): `claim_local` makes the local branch, with its upstream set, before its first push; refuses an existing local branch without pushing; deletes the branch when the tip read again isn't the claim commit; leaves it when the tip can't be read. `claim` and `claim_node` make no branch.
- `orchestrator.run/run` (modified): claims through `claim_local`; the leftovers step also removes a local branch, checked out nowhere, whose tip is an unpushed claim (one parent, the same tree as it, the parent on `origin/main`, not contained in `origin/impl/<component>`). New scenarios cover an interruption after the push, one before it, and another clone's claim made meanwhile. It needs `orchestrator/claim-local`.

## Constraints

- `lease-claims`: a clone resumes only a claim its own local branch is at; another clone's claim is never resumed or deleted.
- `path-ownership`: `claim_local` is added beside `claim_node`, so `orchestrator`'s build doesn't touch `clm-run`, whose build then moves to it.
- `no-dead-code`: `clm-run`'s own branch creation after the claim, and the `release` of a claim whose branch couldn't be made, go with the move.

## Edge cases and workarounds

- A clone already left with a pushed claim and no local branch by an older `clm` still waits for the claim to lapse; the state no longer arises.
- An unpushed claim is told by its structure, not its message: any one-parent commit that adds nothing to a tree on `main` holds nothing to lose, whoever made it.

## Open questions

- The architecture audit noted that `lease-claims`'s `#### Enforced by` lists `orchestrator/claim`, `backlog-claim` and `claim-fix` but not the new `orchestrator/claim-local`. Left out: `claim_local` shares `claim`'s compare-and-swap push, which `orchestrator/claim` already enforces, and editing `ARCHITECTURE.md` would put the branch under review for a cross-reference. Add it if the human wants every claim path listed.
- Building `run` makes one existing `clm-run` test, `resume_refuses_once_the_claim_is_lost`, meet the new rule: its local branch is a bare claim another clone's claim replaced, which is now removed as an unpushed claim before the claim is refused. The build adjusts the test's assertions; the outcome (refused while the other claim lives) is unchanged.

- `clm claim <component> --fix` still sets its local branch after the push, so a fix claim interrupted there has the same window. The item scoped the fix to `clm run`; moving `--fix` onto `claim_local` is left until the window is seen to bite (`yagni`).
