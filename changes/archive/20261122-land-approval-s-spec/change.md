---
status: done
resolves: [land-approval-s-spec-still-offers-the-calls-old-calls-gone-removed]
---

## Intent

`land.approval`'s Contract and Design describe the crate as it is now: they no longer offer `Repo::head(branch, refuse_unpushed)` or the free `held(fold, branch, fingerprint)`, which `old-calls-gone`'s build removed, and the Design no longer says either "stays" until `clm-land` moves. The requirement `old-calls-gone` stays as written, the rule that keeps both calls out. No requirement changes, so nothing is built.

## Problem

`20261121-wave-3-land-by-the-rule` planned the removal as an expand-contract inside one component: the Contract kept the old calls beside `branch_head` and `Repo::held`, "kept until `clm-land` calls" the new ones, and `old-calls-gone`, which `Needs` `land/land` and `land/changes-requested-holds`, removed them once `land`'s build had moved. That build has landed: `crates/clm-approval` has neither call, and `crates/clm-approval/tests/old_calls_gone.rs` proves it at `old-calls-gone`'s current version. The Contract and the Design were written in the future tense of the plan, and a build can't edit them, so they now name an API that doesn't exist, against `spec-is-current`. A dependant's build receives the Contract (`ancestor-scoped-context`) and would be told it may call `Repo::head`.

No change or code fix since 2026-11-15 fixes it: the only change of that time with a delta for `land.approval` is `20261121-wave-3-land-by-the-rule` itself, and no commit since names the item in a `Resolves:` trailer.

## Approaches

- **Taken: drop the two calls from the Contract and the two sentences from the Design, and keep `old-calls-gone` unchanged.** The Contract's `Repo::new` bullet loses its `head(...)` clause and the free `held` bullet goes; **The branch** loses the sentence that `Repo::head` stays beside `branch_head`, and **Held** the clause that the free `held` stays. Every other word of both sections is unchanged. `old-calls-gone`'s text still holds read as the current state: its condition ("once `clm-land` calls `Repo::branch_head` and `Repo::held`") is met, and what it states, that the crate offers neither call, is true and proven. Its version stays, so its citing test stays current and no build is needed.
- **Not taken: reword `old-calls-gone` into the present tense** ("`clm-approval` SHALL NOT offer ..."), dropping the met condition. It changes no behaviour and would make the requirement `behind`, costing a build whose only work is to re-cite one test.
- **Not taken: remove `old-calls-gone`.** It would delete its test, a removal the human must say yes to (`removal-needs-a-yes`), and it is the one rule that keeps an unsigned hold judgement from coming back as a public call. Whether it should go, now that it states only an absence (`requirements-state-what-the-contract-shows`), is put to the human as a question item (below), not decided in this fix.
- **Not taken: a sentence in the Contract saying the calls are gone.** A Contract lists what a component offers; history lives in git and the change archive (`spec-is-current`).

The item's further point, that a change planning an expand-contract leaves its provider's Contract naming the old call once the removal is built, recurs for every such change and is no part of this fix: it is filed as `backlog/an-expand-contract-s-old-call-stays-in-its-provider-s-contract-after-the-removal-is-built.md`.

## Outcome

`specs/land/approval/SPEC.md`'s Contract offers `branch_head` and `Repo::held` with no older form beside them, and its Design describes one head rule and one hold rule. `clm pending land.approval` lists nothing new.

## Open questions

- The architecture audit found that, with the Contract edited, `old-calls-gone` states only an absence and names two calls the Contract no longer offers, which `requirements-state-what-the-contract-shows` counts as a fault. It can't be reworded to name neither call and still say anything, and cutting it is a removal the human must say yes to, which this bug fix shouldn't hold its land for. Filed as the question `backlog/land-approval-s-old-calls-gone-names-calls-its-contract-no-longer-offers.md`: cut it with its test, or keep it.

## Constraints

- No requirement is added, modified or removed, so no component becomes pending and no test moves.
- Edge cases and workarounds: none.
