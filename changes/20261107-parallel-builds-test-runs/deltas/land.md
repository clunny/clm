---
component: land
base: { frontmatter: "5eb6562cfa0095c7", contract: "5a6b4cc6720b87bf" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [ledger, host, gate, check, check.affected, config, vcs, model, apply, effort, land.approval, sync.regenerate, orchestrator]
```

## MODIFIED Contract
Landing a branch on `main`, on the machine that runs the command (`land-locally`). `clm land [<branch>]` runs the gate against `origin/main`, merges the branch in a scratch tree, runs the project's check command, handing it the tests the change can break as `clm affected` selects them (`CLM_SELECTION`, `graph-test-selection`), and `clm check` on the result unless the ledger says that code was already checked, records what it did in the ledger, and pushes the merge commit to `main` without force, retrying when `main` moved. It is the only thing in CLM that pushes `main`. What it keeps in a clone, the land tree, the shared target `.clm/land`, its lock, the check lock `.clm/check.lock` and failing checks' logs, is under the `.clm/` of the main worktree's CLM root, which `clm init` has the repository ignore, so a land leaves nothing untracked whatever the project's language (`land-places-ignored`). It prints `landed <sha> <kind>`, `<kind>` being `code`, `spec` or `spec review` as the gate's first line gives it, and exits 0; it exits 1 with the reason when it pushed nothing, and 2 on a git failure. Whether the branch is approved or held by a request for changes, and the signing of approvals and lands, are `land.approval`'s, as is `clm approve`, which `clm-land` re-exports. There is no after-merge review: a land that needs review lands only once approved (`yes-no-review`). `clm reconnect` pushes a `main` landed offline when it fast-forwards `origin/main`, with the local ledgers. `clm land --check-only` runs the same check command and `clm check` on the current working tree, and lands and records nothing, so a session learns the land check's verdict through a command that names no path (`land-check-command`). The check command of a land or a `--check-only` run waits for any other check of the same clone to end first (`check-runs-one-at-a-time`), so either can take as long as the checks queued before it. Both run it with `CLM_SLOT` set to the slot of the build place that names the branch checked, as `orchestrator`'s `place_slot` gives it, or 0, so the project's tests key their ports and paths apart from those of the tests other builds run at the same time (`build-slots`, `tests-run-concurrently`). While `origin/main`'s backlog holds a `kind: bug` item with `verify_failures`, a slow test the human took in as broken after the land, `clm land` refuses a code land that changes a file of one of the item's components, unless the branch's commit resolves that item; it warns, and lands, a branch touching the components of a red `verified` result no item has taken in yet (`verify-stop`, `verify-after-land`). A host's run never holds a land by itself (`land-locally`). `clm-land` also offers the runner it runs `[land] check` through, under the clone's check lock, for `verify` to run `[verify] check` the same way (`shared-check-runner`).

## ADDED Requirements

### Requirement: check-slot
Every run of `[land] check`, by `clm land <branch>` or by `clm land --check-only`, SHALL set `CLM_SLOT` in the command's environment to what `orchestrator`'s `place_slot` returns for a branch: for a land, the branch it lands; for `--check-only`, the branch the checkout has checked out, or 0 without calling it when the checkout's `HEAD` is detached. It SHALL replace any `CLM_SLOT` the caller's environment holds, setting `CLM_SLOT=0` too. When `place_slot` fails, it SHALL exit 2 with its error and run no check. The check lock (`check-lock`) and `CLM_SELECTION` SHALL be unchanged by it.

#### Scenario: a check-only run in a build's place
- GIVEN a place `a` naming `impl/a` with slot 2, a checkout on `impl/a`, and a `[land] check` that writes `$CLM_SLOT` to a file outside the checkout
- WHEN `clm land --check-only` runs
- THEN it prints `checks pass` and the file holds `2`

#### Scenario: a land
- GIVEN the same place and `[land] check`, and no `checked` record for the merged tree's code key
- WHEN `clm land impl/a` runs
- THEN it lands and the file holds `2`

#### Scenario: outside a place
- GIVEN a checkout on a branch `x` that no place names, the same `[land] check`, and `CLM_SLOT=5` in the caller's environment
- WHEN `clm land --check-only` runs
- THEN the file holds `0`

#### Needs
- orchestrator/place-slots

#### Serves
- architecture/tests-run-concurrently
- capability/work-concurrently
