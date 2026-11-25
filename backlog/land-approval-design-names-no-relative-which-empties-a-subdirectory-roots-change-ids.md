---
title: "land.approval's Design reads change IDs with `--no-relative`, which finds none under a CLM root in a subdirectory"
kind: bug
components: [land.approval]
---

Step: building `land.approval/branch-head@781206cc`. The code review found it, and a test showed it.

What happens: the Design's **The branch** paragraph (`specs/land/approval/SPEC.md`) says the change IDs come from `git diff --name-only --no-renames --no-relative -z --diff-filter=AMT`, and that `--no-relative` keeps a user's `diff.relative` from emptying the list. The command runs in the CLM root. `--no-relative` gives paths from the repository's top level, so when the CLM root is a subdirectory `s/` (supported: `clm-land/tests/land.rs::clm_in_a_subdirectory`) git prints `s/changes/x/change.md`, `ledger`'s `change_ids` matches nothing, `clm approve` records `change: null` and a spec land records no `changes`. `diff.relative` never emptied the list, since the command's directory is the root.

What the build did: `Repo::changes` passes `--relative`, which gives the paths from the root whatever the user's `diff.relative` is, so the requirement's "the paths the head adds, modifies or changes the type of" (`changes/<id>/change.md` under the root) holds. `crates/clm-approval/tests/branch_head.rs::change_ids_under_a_subdirectory_root` proves it.

What is left: a spec change that rewords the Design paragraph to name `--relative` and its reason. `web.worker.review`'s approve reads tree listings and is unaffected.
