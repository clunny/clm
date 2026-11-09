---
title: "Required migration not reachable at the boundary"
kind: workaround
components: [capability]
requirement: capability/upgrade-clm@7ef3a507
---

The scenario's pending required migration can't be set up at the boundary: the shipped clm holds only advised migrations. The test reaches the refusal through a stamp naming an unknown migration (exit 4, names it), so 'points at clm upgrade' is not checked.

The human chose to wait (`20261106-required-migration-test-hook`): "A: wait — Keep the workaround until a real required migration ships, then use it in the test." No test hook or boundary migration list is added.

Condition: the first `required` migration ships in `crates/clm-upgrade`. Then the capability test sets up a project with that migration pending and checks that the refusal exits 4 naming the migration and `clm upgrade` (`clm/upgrade-blocks-commands`, scenario "required migration pending"), and this flag is deleted. A code branch's `Resolves:` trailer closes only `bug` and `deferred` items, so the draft deletes this flag on its spec branch.
