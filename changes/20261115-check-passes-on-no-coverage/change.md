---
status: proposed
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`: the contract step of `20261115-one-route-to-coverage`. `clm-check` stops re-exporting `clm-coverage`'s library, and the requirements that exist to pass it through go or lose that sentence. It is left `proposed` (`contract-step-deferred`): it is applied, on a spec branch of its own, once the twelve crates have moved their imports (`backlog/twelve-crates-reach-coverage-through-clm-check-s-re-exports`), since a removal can't carry a `#### Needs` and `check`'s build would otherwise break the crates that haven't moved.

## Outcome

- **`check/path-ownership` is removed.** It states that `clm-check` offers `owner` and `path_matches` as re-exports. `check.coverage/owner` already states `owner`, with the same first scenario; this change adds to it the sentence on `matches` and the scenario `one glob`, word for word, so nothing the requirement states is lost.
- **`check/coverage-lists`** loses its first sentence, the re-export of five functions. What `clm coverage` prints is unchanged.
- **`check.coverage/state-names`** no longer says the constant is re-exported through `clm-check`.
- Four requirements of `check.affected` need `check.coverage/owner` in place of `check/path-ownership`: `affected-files` and the three others that name it.
- The Contracts and `check`'s Design state one route.

## What it removes

One requirement, `check/path-ownership`. What it states of `owner` and of matching one glob, `check.coverage/owner` holds after this change, with both scenarios and its `capability/work-concurrently` purpose. Not carried, since they state no behaviour: the name `path_matches` for `matches`, and the sentences "`scan(root)` stays public as it is" and "`size`'s counting is unchanged". Two sentences that only say a function is re-exported go too: the first of `check/coverage-lists`, and "re-exported through `clm-check`" in `check.coverage/state-names`. Its tests are deleted by `check`'s build as `obsolete`; the tests of `check.coverage/owner` prove `owner` and `matches` in `clm-coverage`'s own crate. The human agreed to this removal with `20261115-builds-change-only-their-own-files`, on whose branch this change was proposed.

## Constraints

No file, key or format an adopter holds changes: the re-exports are crate-internal.
