---
component: orchestrator
base: { frontmatter: "8ebe9a07e78d2991" }
---

## MODIFIED Frontmatter

```yaml
serves: [capability/build-from-spec, capability/work-concurrently, architecture/component-reconciliation]
```

## MODIFIED Requirements

### Requirement: backlog-claim <!-- base: 50959cb73eb23484 -->
`clm claim backlog/<slug>` SHALL fetch `origin` and push a new commit on `origin/main` to `propose/<slug>` with the same atomicity, takeover and rejection handling as a task claim, and print the branch name. It SHALL exit 1 without pushing when `backlog/<slug>.md` isn't in `origin/main`'s tree, or when the item is taken as the Design defines, saying what holds it: `claimed by propose/<slug>`, `resolved by <id> on main`, or `proposed in <id>`. It SHALL leave out of *proposed* the change of the `spec/<id>` branch the checkout is on. It SHALL take over a lapsed claim when nothing else holds the item.

#### Scenario: racing proposers
- GIVEN `backlog/q.md` in `origin/main` and no `propose/q`
- WHEN two sessions run `clm claim backlog/q` at once
- THEN exactly one exits 0 and `origin/propose/q` is its claim commit; the other exits 1 saying the item was claimed by someone else

#### Scenario: live claim refused
- GIVEN `origin/propose/q` committed 1 hour ago
- WHEN `clm claim backlog/q` runs
- THEN it exits 1 naming `propose/q`, and the branch is unchanged

#### Scenario: proposed in a spec branch
- GIVEN `origin/spec/20260101-c` whose `changes/20260101-c/change.md` lists `q` in `resolves`, no `changes/20260101-c/` in `origin/main`, and no `propose/q`
- WHEN `clm claim backlog/q` runs from a checkout on `main`
- THEN it exits 1 naming `20260101-c`, and `origin` has no `propose/q`

#### Scenario: proposed only in an autosync snapshot
- GIVEN `origin/autosync/spec/20260101-c` whose `change.md` has `from: backlog/q`, and no `origin/spec/20260101-c`
- WHEN `clm claim backlog/q` runs
- THEN it exits 1 naming `20260101-c`

#### Scenario: own spec work
- GIVEN the same `origin/spec/20260101-c`, and the checkout on `spec/20260101-c`
- WHEN `clm claim backlog/q` runs
- THEN it exits 0 and pushes `propose/q`

#### Scenario: resolved by a change on main
- GIVEN change `20260101-d` outside the archive in `origin/main`, listing `q` in `resolves`
- WHEN `clm claim backlog/q` runs
- THEN it exits 1 naming `20260101-d` on main

#### Scenario: merged spec work no longer holds
- GIVEN `origin/spec/20260101-c` naming `q` in `from`, and `changes/archive/20260101-c/` in `origin/main`, with `backlog/q.md` still there
- WHEN `clm claim backlog/q` runs
- THEN it exits 0

#### Scenario: lapsed claim taken over
- GIVEN `origin/propose/q` committed 5 hours ago, and nothing else naming `q`
- WHEN `clm claim backlog/q` runs
- THEN it exits 0 and `origin/propose/q` is the new claim commit

#### Scenario: unknown item
- GIVEN no `backlog/q.md` in `origin/main`
- WHEN `clm claim backlog/q` runs
- THEN it exits 1 saying there is no such item, and `origin` has no `propose/q`

#### Serves
- capability/keep-a-backlog
- capability/propose-intent
- capability/work-concurrently

### Requirement: taken-items <!-- base: b29275d6c4aa6b6c -->
`taken_items(root, expiry)` SHALL fetch `origin` and return the changes that hold backlog items, each with its state (`main` or `proposed`), its ID, the items it names in `from` or `resolves` and its components as `clm proposals` computes them, and the live `propose/<slug>` claims that no such change names, each with its item and components; `clm proposals` SHALL print exactly these.

#### Scenario: same as proposals
- GIVEN change `20260101-d` on main resolving existing item `r` with a delta for `gate`; `origin/spec/20260101-c` with a delta for `check` and `resolves: [q]`, where `q` has `components: [model]`; `origin/propose/s` committed 1 hour ago, where `s` has `components: [ci]`; and `origin/propose/q` committed 1 hour ago
- WHEN `taken_items` runs with a 4-hour expiry
- THEN it returns `20260101-c` proposed with items `q` and components `check` and `model`, `20260101-d` on main with item `r` and component `gate`, and the claim `s` with component `ci`, and no claim `q`

#### Serves
- capability/keep-a-backlog
- capability/work-concurrently

### Requirement: places <!-- base: 39abce44b631fe6d -->
A place SHALL be the file `<git common dir>/clm/builds/<name>` in the clone's git common directory, whichever worktree of the clone the command runs in, holding `kind`, `taken` and one `branch` line per branch it belongs to. A place SHALL count unless its branch has landed, or its claim isn't live and it was taken longer ago than the expiry, as the Design defines for builds and drafts; a file that can't be read or parsed SHALL count. `places` SHALL NOT fetch. `take_place` SHALL, under an exclusive lock on `<git common dir>/clm/builds.lock`, delete the places that don't count and then reuse the place of the same name when it counts, take a new one when fewer than `max_builds` count, and otherwise take none and report the number held and the cap. `rename_place` SHALL, under the same lock, rename a place and refuse, changing nothing, when a place of the new name exists. `free_place` SHALL delete the place's file and succeed when there is none.

#### Scenario: lapsed and landed places don't count
- GIVEN places `a`, for `origin/impl/a` committed 5 hours ago and taken 5 hours ago; `b`, whose `impl/b` tips are ancestors of `origin/main`; and `c`, for `origin/impl/c` committed 1 hour ago
- WHEN `places` is read with an expiry of 4 hours
- THEN only `c` counts

#### Scenario: taken before the claim
- GIVEN a place `a` taken 1 minute ago whose component has no `impl/a` branch, local or on `origin`
- WHEN `places` is read
- THEN `a` counts

#### Scenario: full
- GIVEN `max_builds` 2 and two places that count, `a` and `b`
- WHEN `take_place` runs for `c`, and then for `a`
- THEN the first takes nothing and reports 2 held of 2, and the clone has no place `c`; the second reuses `a`

#### Scenario: racing takers
- GIVEN `max_builds` 1 and no places
- WHEN two subprocesses run `take_place` at once, for `a` and for `b`
- THEN exactly one of them takes a place, and the clone's places directory holds one file

#### Scenario: rename onto an existing place
- GIVEN places `draft-1-q` and `20261027-q`
- WHEN `rename_place` runs from `draft-1-q` to `20261027-q`
- THEN it says `20261027-q` exists, and both places are unchanged

#### Scenario: places in a bare repository
- GIVEN a bare repository with a linked worktree, and a place taken from that worktree
- WHEN `places` is read from the same worktree
- THEN the place is listed, and its file is under the bare repository's directory

#### Scenario: draft place
- GIVEN a place `20261027-x` of kind `draft` naming `spec/20261027-x` and `propose/q`, where `origin/propose/q` was committed 5 hours ago, `origin/spec/20261027-x` 1 hour ago, and `origin/main` has no `changes/20261027-x/`
- WHEN `places` is read with an expiry of 4 hours
- THEN it counts; and once `origin/main` holds `changes/20261027-x/`, it doesn't

#### Scenario: damaged file
- GIVEN a file `z` in the clone's places directory holding `not a place`
- WHEN `places` is read
- THEN `z` counts

#### Serves
- capability/build-from-spec
- architecture/component-reconciliation

### Requirement: archived-spec-work <!-- base: bdfbe8d6674d3698 -->
`clm claim backlog/<slug>`, `clm proposals` and `taken_items` SHALL read a `spec/<id>` or `autosync/spec/<id>` ref whose tip holds no `changes/<id>/change.md` through its `changes/archive/<id>/change.md`, when the tip holds one and change `<id>` is under neither `changes/` nor `changes/archive/` in `origin/main`: its `from` and `resolves` SHALL hold items as a live `change.md`'s do, and the names under `changes/archive/<id>/deltas/` SHALL give its components. When the checkout is on `spec/<id>` and has no working-tree `changes/<id>/change.md`, the items its working-tree `changes/archive/<id>/change.md` names SHALL be left out of *claimed*. A tip holding both files SHALL be read through the live one alone.

#### Scenario: rename branch holds its items
- GIVEN `origin/spec/20260101-r` whose tip holds `changes/archive/20260101-r/change.md` with `status: done`, `renames` and `resolves: [q]`, a file `changes/archive/20260101-r/deltas/sync.md`, and no `changes/20260101-r/change.md`; no `changes/20260101-r/` or `changes/archive/20260101-r/` in `origin/main`; `backlog/q.md` with `components: [model]`; and no `propose/q`
- WHEN `clm proposals` runs from a checkout on `main`, then `clm claim backlog/q`
- THEN `clm proposals` prints `20260101-r proposed [model,sync] q`, and `clm claim` exits 1 naming `20260101-r`, with no `propose/q` on `origin`

#### Scenario: landed rename holds nothing
- GIVEN the same `origin/spec/20260101-r`, and `changes/archive/20260101-r/` in `origin/main`, with `backlog/q.md` still there
- WHEN `clm claim backlog/q` runs
- THEN it exits 0

#### Scenario: live change wins
- GIVEN `origin/spec/20260101-c` whose tip holds `changes/20260101-c/change.md` with `resolves: [q]` and `changes/archive/20260101-c/change.md` with `resolves: [s]`
- WHEN `clm proposals` runs from a checkout on `main`
- THEN its line for `20260101-c` names `q` and not `s`

#### Scenario: own rename branch
- GIVEN the checkout on `spec/20260101-r` holding only `changes/archive/20260101-r/change.md` with `resolves: [q]` in its working tree, and `origin/propose/q` committed 1 hour ago
- WHEN `clm proposals` runs
- THEN it prints no line for `20260101-r` or `q`

#### Serves
- capability/keep-a-backlog
- capability/work-concurrently
