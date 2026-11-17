---
component: sync
---

## MODIFIED Requirements



### Requirement: warnings-name-the-command <!-- base: 32d2f35d3a1aea5e -->
The warnings the rebase prints on stderr that name the command (those `file-proposal-items` prints when the ledger can't be read, for each ledger warning and for each proposal it skips, and the error for a temporary worktree it couldn't remove) SHALL name the command: each is printed as `clm rebase: <warning>`. The warnings `clm_sync::sync`'s report returns carry no prefix, and `clm-sync`'s call of `clm rebase`, which writes everything the command prints (`commands-in-crate`), writes each with it. The call takes no argument that names the command: the old subcommand, the one other name it was ever run by, no longer runs (`clm/old-command-names-retired`).

#### Scenario: a skipped proposal's warning
- GIVEN branch `impl/a` and the ledger fold holding an open proposal whose slug is `Not Kebab`
- WHEN `clm_sync::sync` runs
- THEN its report's warnings hold one starting `skipped proposal` and naming that proposal, and none starts with `clm`

#### Serves
- architecture/plain-terms

#### Scenario: a skipped proposal
- GIVEN branch `impl/a` and the ledger fold holding an open proposal whose slug is `Not Kebab`
- WHEN `clm rebase` runs
- THEN its stderr holds `clm rebase: skipped proposal` naming that proposal
