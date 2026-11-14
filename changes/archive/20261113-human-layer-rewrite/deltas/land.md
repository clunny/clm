---
component: land
---

## MODIFIED Requirements

### Requirement: offline-land <!-- base: 42e209848da4fec8 -->
`clm land` SHALL land through `origin` on every host, `none` included, whose `origin` is the clone's own remote (`host/none-keeps-a-remote`), and SHALL never land on the local `main` alone. When `origin` can't be reached, `clm land` SHALL exit 1 saying it is offline and to land when `origin` is reachable, land nothing locally and append nothing. A fetch of `origin` by `clm land` or `clm reconnect` that fails SHALL be tried again, at most twice more, waiting 1 second before the first of these and 2 seconds before the second, so that a land whose fetch lost a race with another land of the same clone updating a remote-tracking ref (git's `cannot lock ref`) goes on as if the first fetch had succeeded; when the last try fails it SHALL say it is offline only when `origin` can't be reached either, and otherwise exit 1 with git's message from the last fetch and no `offline`, landing nothing and appending nothing. `clm reconnect` SHALL fetch `origin`, pruning the remote-tracking refs of branches `origin` no longer has, then push `main` when `origin/main` doesn't exist or is an ancestor of it, then every local ledger branch, and print `reconnected <n> lands`; when `origin/main` has moved it SHALL push only the ledgers, print `land again <branch>` for each local land's branch, and exit 1.

#### Scenario: no host lands through the clone's own remote
- GIVEN a repository with `host = "none"` whose `origin` is a bare repository `keep_remote` made, holding `main`, and a passing code branch `impl/a` pushed there
- WHEN `clm land impl/a` runs
- THEN `main` on `origin` is a merge commit of the branch, the ledger branch on `origin` has `landed`, and it printed `landed <sha> code` with no `(local)`

#### Scenario: offline with a remote refuses
- GIVEN a clone with `host = "github"` whose `origin` URL points nowhere, and a passing code branch
- WHEN `clm land` runs
- THEN it exits 1 saying it is offline, the local `main` is unchanged, and the ledger has no new record

#### Scenario: first push after adding a remote
- GIVEN the repository of *no host lands through the clone's own remote* after its land, its `origin` then set with `git remote set-url` to an empty bare repository, the remote-tracking refs of the old one still in the clone
- WHEN `clm reconnect` runs
- THEN `origin/main` is the local `main`, the ledger branch is on `origin`, and it printed `reconnected 1 lands`

#### Scenario: origin moved
- GIVEN the same, but `origin` already has a `main` that isn't an ancestor of the local one
- WHEN `clm reconnect` runs
- THEN `origin/main` is unchanged, the ledger branch is on `origin`, it printed `land again` with the branch, and it exited 1

#### Scenario: a fetch that loses a ref race is tried again
- GIVEN a passing code branch `impl/a`, and a script named `git` in a directory put first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/main'` on stderr and exits 1 without running the real `git`, and runs the real `git` for every other call; the test runs the `clm` binary as a child process with that `PATH` (`crates/clm/tests`)
- WHEN `clm land impl/a` runs
- THEN it exits 0 printing `landed <sha> code`, `main` on `origin` is that merge commit, and its stderr doesn't contain `offline`

#### Scenario: a fetch that keeps failing with origin reachable
- GIVEN a passing code branch `impl/a`, and a script named `git` first on `PATH`, as in *a fetch that loses a ref race is tried again*, that fails every `fetch` naming `origin` that way
- WHEN `clm land impl/a` runs
- THEN it exits 1, its message contains `cannot lock ref` and not `offline`, `main` on `origin` is what it was before the land, and the ledger has no new record

#### Needs
- host/none-keeps-a-remote

#### Serves
- capability/git-host
- capability/work-concurrently
- architecture/land-locally

### Requirement: derived-specs <!-- base: eb99146d5253e5a3 -->
When the head adds a change, as `gate.fingerprint/fingerprint` defines it (`clm-gate`'s `adds_change` over the merge base's and the head's listings), or applies one, a change whose `changes/<id>/change.md` has status `proposed` or `approved` at the base and, in the merge commit, has status `applied`, or is gone while `changes/archive/<id>/change.md` has status `done`, `clm land` SHALL, after the merge commit and before any check, regenerate `specs/` with `sync.regenerate`'s `regenerated`, without its stale-delta merge, onto the base, `origin/main`'s on every host (`land/offline-land`), from the CLM root in the tree holding the merge commit, for the added changes whose status is `applied`, or `done` for a new archived one, and the applied ones, in the order `regenerated` chooses (`sync.regenerate/regenerated-tree`), a `change.md` that doesn't parse regenerating nothing, and compare the result with the merge commit's `specs/` byte for byte over the union of both sides' paths in byte order, a path on one side only being a difference. When they differ, or `regenerated` stops, it SHALL push nothing and exit 1 with rule `land/derived-specs`, naming the first differing path or the lines `regenerated` returned, and `clm rebase`. A head that adds and applies no change SHALL land without the comparison, since its `specs/` edits are its own. No file under `crates/clm-land/src` SHALL call `clm_apply::reapply` or `clm_model::load`: the regenerate is `sync.regenerate`'s alone. `clm-land` SHALL call it from crate `clm-regenerate`, never through `clm-sync`, which reaches `clm-health` (`clm/health-isolated`).

#### Scenario: regenerated branch lands
- GIVEN `spec/x` adding applied change `x`, whose delta adds requirement `a/r`, with `specs/` as `clm rebase` regenerates it, and an `origin/main` that has since added requirement `b/s`
- WHEN `clm land spec/x` runs
- THEN it lands, and `main`'s `specs/a/SPEC.md` holds `a/r` and `specs/b/SPEC.md` holds `b/s`

#### Scenario: change built on another lands whatever its ID
- GIVEN `spec/x` adding applied changes `x` and `w`, `w` sorting first, where `x`'s delta rewrites the first line of `a`'s Design with `origin/main`'s hash as its base and `w`'s rewrites its second line with the Design's hash after `x` applies as its base, with `specs/` as `clm rebase` regenerates it
- WHEN `clm land spec/x` runs
- THEN it prints no `land/derived-specs` line and lands, and `main`'s `specs/a/SPEC.md` holds both edits

#### Scenario: applied change's deltas lost
- GIVEN an `origin/main` holding change `y` with `status: proposed` and a delta adding requirement `a/x`, and `spec/z` that adds change `z`, which has no delta, and sets `y` to `applied`, with `specs/` as `origin/main`'s, so `a/x` is missing
- WHEN `clm land spec/z` runs
- THEN it exits 1 printing `land/derived-specs`, naming `specs/a/SPEC.md` and `clm rebase`, and `main` on `origin` is unchanged

#### Scenario: hand edit under specs refused
- GIVEN the same branch with one more commit changing a line of `specs/a/SPEC.md` that no delta of `x` holds, and an `approved` record for its fingerprint
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/derived-specs`, naming `specs/a/SPEC.md` and `clm rebase`, the check command never ran, and `main` on `origin` is unchanged

#### Scenario: branch that adds no change
- GIVEN `impl/a`, which adds no change and sets `status: active` in `specs/a/SPEC.md`
- WHEN `clm land impl/a` runs
- THEN it lands with that edit

#### Scenario: no copy of the regenerate
- GIVEN the workspace
- WHEN a test in `clm-land` reads every file under `crates/clm-land/src`
- THEN no file holds `clm_apply::reapply` or `clm_model::load`

#### Serves
- capability/catch-drift
- architecture/hash-based-staleness
- architecture/humans-review-intent

#### Needs
- clm/renamed-commands
- gate.fingerprint/fingerprint
- sync.regenerate/regenerated-tree
- land.approval/approve

### Requirement: regenerates-behind-specs <!-- base: e2612b1144865f10 -->
When the head adds or applies a change, as `derived-specs` defines both, and a file under `specs/` changed both between the merge base and the base and between the merge base and the head, and every path `git merge-tree --write-tree --name-only` reports conflicted, if any, lies under `specs/`, `clm land` SHALL land the head without sending its author to `clm rebase`. It SHALL first regenerate `specs/` with `sync.regenerate`'s `regenerated`, without its stale-delta merge, from the head's tree onto the merge base, for the changes the head adds whose status at the head is `applied` or `done` and those it applies over the merge base, and compare the result with the head's `specs/` as `derived-specs` compares; it SHALL then regenerate `specs/` from the merged tree onto the base for the changes `derived-specs` names, and make the merge commit's tree the merged tree with `specs/` replaced by that result. When either regenerate stops, or the first comparison finds a difference, it SHALL push nothing and exit 1 with rule `land/derived-specs`, naming the differing path or the lines `regenerated` returned, and `clm rebase`. It SHALL run the gate it runs on the head on that merge commit instead, before any check. A head that adds and applies no change, or whose merge conflicts in a path outside `specs/`, SHALL be refused as `land` states.

#### Scenario: behind in a derived file lands
- GIVEN `spec/x` adding applied change `x`, whose delta adds requirement `a/r`, with `specs/` as `clm rebase` regenerated it, and an `origin/main` that has since landed a change adding requirement `a/s` to the same `specs/a/SPEC.md`, so the file changed on both sides
- WHEN `clm land spec/x` runs
- THEN it prints no `gate/up-to-date` or `land/derived-specs` line and lands, `main`'s `specs/a/SPEC.md` holds `a/r` and `a/s`, and the gate ran on the merge commit

#### Scenario: main moves during the check
- GIVEN that `spec/x` on an `origin/main` that hasn't moved, and a check command that, the first time it runs, lands on `origin/main` a change adding requirement `a/s` to `specs/a/SPEC.md`
- WHEN `clm land spec/x` runs
- THEN its second attempt lands with no `clm rebase` in between, and `main`'s `specs/a/SPEC.md` holds `a/r` and `a/s`

#### Scenario: hand edit on a behind branch refused
- GIVEN the behind `spec/x` with one more commit changing a line of `specs/a/SPEC.md` that no delta of `x` holds
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/derived-specs`, naming `specs/a/SPEC.md` and `clm rebase`, the check command never ran, and `main` on `origin` is unchanged

#### Scenario: delta stale against main refused
- GIVEN `spec/x` whose delta modifies requirement `a/q`, and an `origin/main` that has since landed a change modifying `a/q`
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/derived-specs` and `clm rebase`, the check command never ran, and `main` on `origin` is unchanged

#### Scenario: branch that adds no change still refused
- GIVEN `impl/a`, which adds no change and sets `status: active` in `specs/a/SPEC.md`, and an `origin/main` that has since changed `specs/a/SPEC.md`
- WHEN `clm land impl/a` runs
- THEN it exits 1 naming `clm rebase`, and `main` on `origin` is unchanged

#### Serves
- capability/work-concurrently
- architecture/hash-based-staleness
