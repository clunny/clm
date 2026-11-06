---
title: "Use the first real required migration in the upgrade-clm capability test"
kind: deferred
components: [capability]
---

`capability/upgrade-clm`'s second scenario needs a project with a pending `required` migration, but the shipped `clm` holds only advised migrations, so the capability test reaches the refusal through a too-new `upgraded` stamp instead, and does not check that the message points at `clm upgrade`.

The human chose to wait (`20261106-required-migration-test-hook`): "A: wait — Keep the workaround until a real required migration ships, then use it in the test." No test hook or boundary migration list is added.

Condition: the first `required` migration ships in `crates/clm-upgrade`. Then the capability test sets up a project with that migration pending and checks that the refusal exits 4 naming the migration and `clm upgrade` (`clm/upgrade-blocks-commands`, scenario "required migration pending"). The flag `backlog/workaround-capability-upgrade-clm-1.md` is then settled too, but a code branch's `Resolves:` trailer closes only `bug` and `deferred` items, so the draft deletes the flag on its spec branch.
