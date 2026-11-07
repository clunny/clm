---
component: land
base: { contract: "371c9fee6330ca74" }
---

## MODIFIED Contract
Landing a branch on `main`, on the machine that runs the command (`land-locally`). `clm land [<branch>]` runs the gate against `origin/main`, merges the branch in a scratch tree, runs the project's check command, handing it the tests the change can break as `clm affected` selects them (`CLM_SELECTION`, `graph-test-selection`), and `clm check` on the result unless the ledger says that code was already checked, records what it did in the ledger, and pushes the merge commit to `main` without force, retrying when `main` moved. It is the only thing in CLM that pushes `main`. What it keeps in a clone, the land tree, the shared target `.clm/land`, its lock, the check lock `.clm/check.lock` and failing checks' logs, is under the `.clm/` of the main worktree's CLM root, which `clm init` has the repository ignore, so a land leaves nothing untracked whatever the project's language (`land-places-ignored`). It prints `landed <sha> <kind>`, `<kind>` being `code`, `spec` or `spec review` as the gate's first line gives it, and exits 0; it exits 1 with the reason when it pushed nothing, and 2 on a git failure. Whether the branch is approved or held by a request for changes, and the signing of approvals and lands, are `land.approval`'s, as is `clm approve`, which `clm-land` re-exports. There is no after-merge review: a land that needs review lands only once approved (`yes-no-review`). `clm reconnect` pushes a `main` landed offline when it fast-forwards `origin/main`, with the local ledgers. `clm land --check-only` runs the same check command and `clm check` on the current working tree, and lands and records nothing, so a session learns the land check's verdict through a command that names no path (`land-check-command`). The check command of a land or a `--check-only` run waits for any other check of the same clone to end first (`check-runs-one-at-a-time`), so either can take as long as the checks queued before it. While `origin/main`'s backlog holds a `kind: bug` item with `verify_failures`, a slow test the human took in as broken after the land, `clm land` refuses a code land that changes a file of one of the item's components, unless the branch's commit resolves that item; it warns, and lands, a branch touching the components of a red `verified` result no item has taken in yet (`verify-stop`, `verify-after-land`). A host's run never holds a land by itself (`land-locally`). `clm-land` also offers the runner it runs `[land] check` through, under the clone's check lock, for `verify` to run `[verify] check` the same way (`shared-check-runner`).

## ADDED Requirements

### Requirement: verify-stop
Before merging, `clm land` SHALL take the *held* components: the `components` of every `kind: bug` item on the fetched `origin/main`'s backlog whose `verify_failures` is not empty. When the branch's code changes a file under the `paths` of a held component, as the gate judges code, it SHALL push nothing and exit 1 with rule `land/verify-stop`, naming each held component it touches and the item that holds it, and saying that the branch fixing it lands once a commit it adds carries the trailer `Resolves: backlog/<slug>` for that item; a branch one of whose commits carries that trailer for every item holding a component it touches SHALL land as before. It SHALL also read the red `verified` results on `origin/main`'s first-parent chain after the newest passing one, the newest record for each commit (`ledger/verified-record`), and for each that no backlog item's `verify_failures` names and whose `components` the branch's code touches, print on stderr `warning: slow tests failed after <sha>, culprit <land|unknown>, on <component>...; run /clm:triage to take it in` and land as before. A spec land, and a code land touching no held component, SHALL land as before.

#### Scenario: held component refused
- GIVEN `origin/main` holding `backlog/b-broke.md` with `kind: bug`, `components: [b]` and `verify_failures` naming a commit, and a code branch changing a file under `b`'s `paths`
- WHEN `clm land` runs
- THEN it exits 1 with `land/verify-stop` naming `b` and `b-broke`, and `main` on `origin` is unchanged

#### Scenario: other components land
- GIVEN the same item, and a code branch changing only a file under `a`'s `paths`
- WHEN `clm land` runs
- THEN it lands

#### Scenario: the fix lands
- GIVEN the same item, and a branch changing `b`'s file whose commit carries `Resolves: backlog/b-broke` and deletes the item
- WHEN `clm land` runs
- THEN it lands

#### Scenario: an untriaged red warns
- GIVEN no item with `verify_failures`, a red `verified` record for `origin/main`'s head naming component `b`, and a code branch changing `b`'s file
- WHEN `clm land` runs
- THEN it prints a warning naming the red commit and `b` and `/clm:triage`, and lands

#### Scenario: a red record alone holds nothing
- GIVEN a red `verified` record naming `b` written by a host's pipeline, and no backlog item naming it
- WHEN `clm land` runs on a branch changing `b`'s file
- THEN it lands

#### Needs
- ledger/verified-record
- model/parse-backlog

#### Serves
- capability/slow-tests-after-land

### Requirement: shared-check-runner
`clm-land` SHALL offer as a library call the runner that `clm land` and `clm land --check-only` run `[land] check` through: it takes the command, the directory to run it in, an optional selection file for `CLM_SELECTION`, the environment to set and unset, and the log file's path, runs the command through `sh -c` under the clone's check lock (`check-lock`), printing its waiting line once when it waits, copies its output to stderr and the log as `land-check-log` says, and returns whether it passed with the last 40 lines, leaving the log for the caller to keep or remove. `clm land` and `--check-only` SHALL call it with the environment they set today, so their behaviour is unchanged.

#### Scenario: a caller waits on the check lock
- GIVEN a command that writes `ok` and exits 0, and `.clm/check.lock` held by another process that releases it after 1 second
- WHEN the runner runs the command
- THEN it printed `waiting for another check` once, ran the command after the lock was released, wrote `ok` to the log, and returned a pass

#### Scenario: land unchanged
- GIVEN a `[land] check` that fails printing 50 numbered lines
- WHEN `clm land` runs
- THEN its refusal holds lines 11 to 50 and the log's path, as `land-check-log` says

#### Serves
- capability/slow-tests-after-land
