---
status: applied
resolves:
  - web-worker-writes-skip-the-replica-object
---

# The Worker's writes go through the Replica object

## Intent

A proposal, a withdrawal and a decision made in the web app should go through the repo's `Replica` object, as `web.worker`'s Design and Contract already say, so the Worker's request stays within the Free plan's 10 ms and each write is judged against the tip. Nothing in `crates/clm-web` posts to the object's `/write` today. This adds the requirement that states the route, makes the object refuse to judge a write against a tip it knows to be behind, and removes the Worker's own copy of the checks when the component is built.

## Problem

`web.worker.replica/writes-through-object` is built and proven on the object's side, and no caller reaches it. The routes do the work themselves:

- `capabilities::decide` reads the spec files, the ledgers and the signers from GitHub and loads the graph during the Worker's request, against the Design's "a Worker request computes no view" and the 10 ms budget (`web-on-free-plan`). `web-repo-replica` records what that costs: requests above the budget are eventually killed with a 503, which the app shows as a repo it can't read.
- `proposals::propose` judges a slug from cached views up to 30 seconds old, so it can miss a push the object hasn't applied.
- `proposals::withdraw` reads the ledger and appends on the Worker, and each route then tells the object with a made-up `push` event (`views::ledger_moved`), which the webhook repeats a moment later.

No requirement of `web.worker` states the route, so the fix changes `specs/` (`code-fix-needs-a-written-rule`).

Reading the object's side for this also found two gaps its requirement doesn't cover, which the routes' written answers depend on. `Object::catch_up` ignores a failed read of the ref and validates anyway, while `decide` promises a 502 when the tip can't be read and compares the version with the tip "now". And `write` answers `preparing` only before it has a tip, while a replica still fetching its spec files refuses a proposal as a bare 422, where the Worker's route answers `preparing` today.

## Approaches

- **Route the writes through `/write` (chosen).** The Design, the Contract, `web-repo-replica` and the object's tested code all already say this, and only the caller is missing. It is what brings `decide` within the budget, which is the reason the object exists. The Worker's route becomes a body check, the access check and one post, whose answer it passes on.
- **Remove `/write` and rewrite the Design around the Worker's routes** (`no-dead-code`, as the item offers). Rejected: `decide` would keep loading the whole graph on the Worker's request, the kind of work `web-repo-replica` measured at about 350 ms of CPU against a 10 ms limit Cloudflare enforces by killing requests. It would keep working only for as long as overruns are tolerated.
- **Move only `decide`, the route over budget.** Rejected: a proposal's slug would still be judged against a stale view, and the Contract's one path for the token would become two.

A related choice is how the routes' tests prove the rules. The scenarios of `proposals`, `decide` and `architecture-decide` name what is written and answered, which the object now decides, so a scripted object would prove only the pass-through and every citing test would claim more than it checks. So `clm-web`'s tests drive those routes through a real `clm_web_replica::Object` over the fake GitHub, and only `writes-to-object`'s own scenarios script it. Its cost is one dev-dependency, in the direction imports already run.

## Outcome

- `web.worker/writes-to-object` (new): one post to `/write`, its body's shape, the answer passed on as given, 503 `unavailable` when the object can't be reached, and nothing else asked of GitHub or the object.
- `web.worker/proposals`, `decide` and `architecture-decide` keep the same answers to the app, reworded where they named the snapshot or the Worker's own reads, and they need `web.worker.replica/writes-through-object`.
- `web.worker.replica/writes-through-object` gains the 502 and the `preparing` answer.
- The Worker's slug goes: its body check refuses a title with no ASCII letter or digit, and `replica`'s `validate` is the one place that slugs. `clm-replica` and `clm-web` each hold the same dozen lines today.

The build removes what this obsoletes (`no-dead-code`): `views::ledger_moved`, `proposals::Facts`, the Worker's `slug`, the reads in `capabilities::decide`, and `clm-model` and `clm-effort` from `clm-web`'s dependencies where only its tests still use them. The tests' `signers_fail` setups for `decide` move to the fake's failing ref read. `web.worker.review`'s Contract stops saying the capabilities routes use `review::signed`; its code doesn't change, since `approve` still calls it.

## Constraints

- The app sees no change of shape: every status and body it reads stays as `web.worker`'s Contract gives it. A decision can now answer the 503 `preparing`, which the app's generic "Not decided (503)." covers.
- The token stays out of everything but the one post, and the object keeps it no longer than that request.
- Each write costs the Worker its access check and one Durable Object request, within the 50 subrequests, and the object's GitHub calls count against the object's own budget. The daily Durable Object allowance isn't near: writes are made by hand.

## Edge cases and workarounds

- A push whose spec files didn't all fit the object's 50 subrequests leaves the replica without a full spec tree until the alarm fetches the rest. A decision or a proposal made then answers 422 as a tip whose specs don't load, and the app shows the tip as it is. It lasts a minute at most and only for a very large push, so it stays accepted, with no new refusal kind (`yagni`).
- `settles` is computed from the ledgers the object holds, not read anew from GitHub. A flag settled elsewhere seconds ago, whose webhook hasn't arrived, may be settled twice, which is harmless, since settling is by slug.
