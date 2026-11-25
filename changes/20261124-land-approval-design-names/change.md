---
status: proposed
resolves:
  - land-approval-design-names-no-relative-which-empties-a-subdirectory-roots-change-ids
---

## Intent

`land.approval`'s Design said the change IDs come from `git diff ... --no-relative ...`, so that a user's `diff.relative` can't empty the list. The command runs in the CLM root, and `--no-relative` gives paths from the repository's top, so with a CLM root in a subdirectory (which `land` supports) git prints `s/changes/x/change.md`, `ledger`'s `change_ids` matches nothing, and `clm approve` and a spec land record no change. `diff.relative` never emptied the list, since the command already runs in the root.

The build of `branch-head@781206cc` already passes `--relative`, which gives the paths from the root whatever `diff.relative` says, and `crates/clm-approval/tests/branch_head.rs::change_ids_under_a_subdirectory_root` proves it. `branch-head` as written ("the paths the head adds, modifies or changes the type of", `changes/<id>/change.md` under the root) already states that behaviour. This change rewords the Design paragraph **The branch** to name `--relative`, that the command runs in the CLM root, and why.

## Approaches

- Reword the Design to match the code (chosen): the requirement already states the behaviour, and the code and its test already hold it.
- Keep `--no-relative` and strip the root's prefix from each path, as `gate` does for its paths: more code for the same answer, since this read only ever looks under the root's `changes/` (`yagni`).

## Outcome

The Design names the flag the code passes. No requirement or scenario changes, nothing is cut, no component becomes pending, and no code work is left: `Repo::changes` and its doc comment already say `--relative`.
