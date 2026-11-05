---
title: "`clm run` records its claim in the local branch before pushing it"
date: 2026-11-05
status: accepted
---

`clm run <component>` pushed its claim commit to `origin/impl/<component>` and only then made the local branch `impl/<component>`. Interrupted between the two (issue #148), the clone kept no trace of a claim it had just pushed: every later `clm run <component>` refused the component as claimed, `clm run --finish` didn't change that, and the only ways on were deleting the remote branch, which auto mode blocks as destructive, or waiting out the four-hour expiry.

The claim now goes through `orchestrator`'s `claim_local`, which makes the local branch at the claim commit before the first push, so no interruption can leave a pushed claim without its local branch. The existing resume rule then covers an interruption after the push: the local branch contains `origin/impl/<component>`, so the next run resumes it. An interruption before the push leaves a local branch whose tip is an *unpushed claim*: a commit with one parent, the same tree as that parent, the parent on `origin/main`, and not on `origin/impl/<component>`. It holds no work, so the next run removes it as it removes a landed leftover and claims as usual. Another clone's claim is never resumed, since this clone never has a local branch at it (`lease-claims`). A failed claim whose tip shows nothing was claimed deletes its local branch; one whose tip couldn't be read leaves it for the next run's fetch to judge.

Rejected:
- **Recognise one's own claim on `origin` with no local trace.** A claim commit names no clone, and this clone's build place is no proof: a place can outlive a run interrupted before its push while another clone claims the component, which would then be taken from it, breaking `lease-claims`. Writing a clone ID into the claim commit would work, but adds state and a format to keep where an ordering change suffices.
- **Have `clm run --finish` or the leftovers step delete the remote claim.** Deleting a remote branch is the destructive step the item reports as blocked, and it could delete another clone's claim pushed meanwhile.
- **Make `claim_node`, and so `clm claim`, write the local branch.** `clm-run` makes the branch after `claim_node` returns, so the change would break its build from another component, and a plain `clm claim` leaving the clone as it was is its Contract. `claim_local` is added beside it.

Accepted edge cases:
- A clone already left in the reported state, by a `clm` from before `claim_local`, still waits for its claim to lapse; the state no longer arises.
- `clm claim <component> --fix` still sets its branch after the push, so a fix claim interrupted there has the same window; the item scoped the fix to `clm run`, and moving `--fix` onto `claim_local` waits until that window is seen to bite (`yagni`).
