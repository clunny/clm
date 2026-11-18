---
status: done
from: backlog/clm-check-still-re-exports-the-coverage-library
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, the second closing branch of its step 7: the contract step of `20261115-one-route-to-coverage`. `clm-check` stops re-exporting `clm-coverage`'s library, and the requirements that exist to pass it through go or lose that sentence. It was left `proposed` on 2026-11-15 (`contract-step-deferred`), since a removal can't carry a `#### Needs` and `check`'s build would have broken the crates that hadn't moved. It is applied now, from `backlog/clm-check-still-re-exports-the-coverage-library`, because nothing outside `clm-check` names a re-exported item any more.

## What landed first, as code

Two small code branches landed on 2026-11-17 before this one, each with the human's yes, so that the change needs no requirement that only orders a build:

- **The shared test helper names no coverage type.** `crates/clm-orchestrator/tests/common/mod.rs` named `clm_check::Marker` for two fixtures that only `crates/clm-orchestrator/tests/queue.rs` calls; they moved into that file, and `clm-triage` and `clm-run` dropped the `clm-check` dev-dependency they held for the helper.
- **`scenario_list_reaches_clm_check` is deleted.** It sat in `check`'s `crates/clm-check/tests/state_names.rs`, cited `check.coverage/state-names` and proved only the re-export. `check.coverage`'s own test keeps that requirement current.

## Approaches

For the helper: move the two fixtures to their one caller as code (taken); state the same move as requirements of `orchestrator`, `orchestrator.triage` and `orchestrator.run`, which is three builds for text that describes files; or rename the import and give `clm-triage` a `clm-coverage` dependency for fixtures it never calls, which the backlog item suggested. For the test: delete it first as code (taken), or have `check`'s build delete it, which leaves `check`'s exemption in its frontmatter for a second spec branch and orders `check.coverage` after `check`.

## Outcome

- **`check/path-ownership` is removed.** It states that `clm-check` offers `owner` and `path_matches` as re-exports. `check.coverage/owner` already states `owner`, with the same first scenario; this change adds to it the sentence on `matches` and the scenario `one glob`, word for word, so nothing the requirement states is lost.
- **`check/coverage-lists`** loses its first sentence, the re-export of five functions. What `clm coverage` prints is unchanged.
- **`check.coverage/state-names`** no longer says the constant is re-exported through `clm-check`.
- Four requirements of `check.affected` need `check.coverage/owner` in place of `check/path-ownership`: `affected-files` and the three others that name it.
- The Contracts and `check`'s Design state one route.
- So do the sentences that named the old route elsewhere, which the architecture audit found (`spec-is-current`): the Designs of `view`, `view.repo`, `replica` and `check.health` name `clm_coverage::` for the calls their code already imports from it, and `clm/glossary-covers-terms` no longer says the states come "through `clm-check`". The first four build nothing; the root re-cites one requirement.
- **`check` loses its one exemption** from `tests-cite-their-owner`, for `crates/clm-check/tests/state_names.rs`: the file is gone.
- `check`'s Design no longer gives `crates/clm/tests/run.rs`, which holds only the root's tests now, as its example of a test file counted toward the component it cites.
- **The builds, in the order the needs give.**
  - `check.coverage` first: it re-cites `owner` and `state-names`, and writes two tests of `matches` in `crates/clm-coverage/tests/` citing `check.coverage/owner`: one for `one glob`, which has none there, and one for `globs as git reads them`, written from `paths_match_as_git_glob_pathspecs_do` in `crates/clm-check/tests/pending.rs`, the only test that compares the matcher with `git ls-files ':(glob)...'`. `check/path-ownership` is removed and so can carry no need: if `check` is built first for its obsolete markers alone, that test is read from `main` at `165f1e9a`.
  - `check` next, since `check/coverage-lists` needs `check.coverage/owner` so that test exists before `check` deletes its own: it removes the two `pub use clm_coverage::` blocks with their `clm:remove-by` markers, imports what its own modules use from `clm_coverage` privately, and rewrites the crate's doc comment, which still says the library is re-exported. It deletes the four tests that cite `check/path-ownership` (three in `ownership.rs`, and `paths_match_as_git_glob_pathspecs_do`) and `clm_check_re_exports_the_test_lists` in `coverage.rs`, which proves only the sentence `coverage-lists` loses; the tests in `coverage_lists.rs` are re-cited. Its other tests import `clm_coverage` for `scan` and `Pending`.
  - `check.affected` and the root re-cite.

## What it removes

One requirement, `check/path-ownership`. What it states of `owner` and of matching one glob, `check.coverage/owner` holds after this change, with both scenarios and its `capability/work-concurrently` purpose. Not carried, since they state no behaviour: the name `path_matches` for `matches`, and the sentences "`scan(root)` stays public as it is" and "`size`'s counting is unchanged". Two sentences that only say a function is re-exported go too: the first of `check/coverage-lists`, and "re-exported through `clm-check`" in `check.coverage/state-names`. Its tests are deleted by `check`'s build as `obsolete`; the tests of `check.coverage/owner` prove `owner` and `matches` in `clm-coverage`'s own crate. The human agreed to this removal with `20261115-builds-change-only-their-own-files`, on whose branch this change was proposed, and asked for this branch on 2026-11-17.

## Constraints

No file, key or format an adopter holds changes: the re-exports are crate-internal.
