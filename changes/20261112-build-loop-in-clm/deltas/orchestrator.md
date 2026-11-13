---
component: orchestrator
base: { contract: "0c7495a3911b6bf5" }
---

## MODIFIED Contract
`clm next` lists the components whose code is ready to be brought up to their spec, one component ID per line, most urgent first, at most as many as the clone has free build places, and nothing when it has none (`build-places`); when it prints nothing only because no place is free, it says so on stderr, naming the places in use and `max_builds`, so a full cap never reads as an empty queue. `clm claim <component>` claims a component by pushing branch `impl/<component>` to `origin` and prints the branch name. `clm claim <component> --fix backlog/<slug>` claims it for a code-only fix of that `bug` item, or for the code-only work of that `deferred` item (`code-fix-resolves-bug`, `deferred-work-resolves-in-code`), even when the component has nothing pending, and switches the checkout to the claimed branch, or refuses before claiming when it couldn't. Both fetch `origin` first and take `--expiry <hours>` (default 4). When they refuse a component, they say why. Both exit 0 on success, 1 when the work isn't possible (with the reason), and 2 when git fails. `clm status` ends with a `claims` section listing claims, component and backlog alike, as of the last fetch. Launching a build from a claim (`clm run`) is `orchestrator.run`; ending a session (`clm wrap-up`) is `orchestrator.checkpoint`; triaging the backlog into unattended drafts (`clm triage`, `clm draft`) is `orchestrator.triage`. Each builds on the queue this component defines and calls `git` for its plumbing. For them the crate exposes, as library calls: `claim` (a claim with its refusal reason), `claim_local` (a component claim that records the local branch `impl/<component>` before it pushes, for `clm run`: `claim-local`), `refusal(component, state)`, `propose_refs`, `claim_refs`, `proposals` (what `clm proposals` prints), `taken_items(root, expiry)` (what holds backlog items, as the Design's *taken* defines, read from `origin` after fetching: each proposed or on-main change with its state, the items it names and its components, and each live bare claim with its item), `queue_from_refs` and `taken_from_refs` (below), the place calls below and `Error`. Changing one of these is a change to this Contract.

**Build places** (`build-places`). A build or a draft holds a *place* on the clone from when `clm run` or `clm draft` starts it until it lands, is finished with `--finish`, or its claim lapses, and the clone has `config`'s `max_builds` of them (default 3). A place is the file `<git common dir>/clm/builds/<name>`, in the clone's git common directory (`git rev-parse --git-common-dir`), which every worktree of the clone shares, a bare repository's included: `<name>` is the component ID for a build and the change ID for a draft, and the file names the branches the place belongs to. It is local state of the clone, never committed, since nothing under the git directory is. A place *counts* unless its branch has landed or its claim has lapsed and it was taken longer ago than the expiry, as the Design defines; one that doesn't count is free to take and is deleted by the next command that takes a place, and by `clm wrap-up`. The cap counts this clone only. Each place holds a *slot*, a number from 1 up that no other place that counts holds, so a build and its checks can key the host resources its tests take, such as a port, on it (`build-slots`, `tests-run-concurrently`); 0 is the slot of whatever runs outside a place, such as the human's own checkout. For `orchestrator.run`, `orchestrator.triage` and `orchestrator.checkpoint` the crate exposes `places(root, expiry)` (every place with its name, kind, branches, slot, the time it was taken and whether it counts, judged on the remote-tracking refs as they are: it doesn't fetch, so its callers fetch first), `take_place(root, place, max_builds, expiry)` (takes a place, or reuses the one of that name when it counts, or says the clone is full with the number held and the cap), `place_slot(root, branch)` (the slot of the place that names the branch, or 0, failing only when the git common directory can't be found), `rename_place(root, from, to)` (renames a place, or says `to` already exists), `free_place(root, name)` (deletes the file, succeeding when there is none) and `free_stale_places(root, expiry)` (under the places lock, reads the places, deletes each one that doesn't count, and returns a `Swept` for every place as judged there: the `Place` and its outcome, kept, freed, or failed with why the file couldn't be deleted).

The unit of building is the component, not the change (`node-reconciliation`). `specs/` on `main` holds every applied change's text, and a component's code is brought up to all of it at once. A requirement is *current* when a test cites it at its current version, `req:<id>@<version>` (`versioned-markers`, `check.coverage/pending-report`); otherwise it is *pending*: `unbuilt` when nothing cites it, `behind` when only older versions are cited, or `obsolete` when a marker cites a requirement a live change removed. A component is *pending* while it has a pending requirement. A backlog item *blocks* the requirements it names in `blocks` (`requirement-blocks`), and a pending requirement *blocks* the requirements of other components whose `Needs` name it (`requirement-needs`): a blocked requirement stays pending but is left out of the component's work until the item is deleted or the needed requirement is current. A component is ready when it has an unblocked pending requirement. Nothing else orders the work (`needs-alone-order`): `depends_on` says which components' code a component may use, not when it is built, so a requirement that relies on another component's unbuilt work names it in `Needs`; there are no plans, no order between changes and no waits written by hand against time or other changes. A `Needs` is a dependency the spec declares and the markers decide. Priority orders `clm next` (`priority-orders-nodes`).

A proposing session claims a backlog item before designing its fix (`propose-claims`). `clm claim backlog/<slug>` pushes branch `propose/<slug>` to `origin` the way a component claim pushes `impl/*`, with the same expiry and takeover, and prints the branch name. It refuses an item that isn't in `origin/main`'s `backlog/`, or that is already taken: claimed, or named in the `from` or `resolves` of spec work in flight on `origin`. `clm proposals` lists that spec work and the bare claims, each with its components and items, so a session can see what another is already changing. Backlog claims never enter `clm next`.

An `applied` or `implementing` change is *complete* when every requirement its deltas add or modify is current, each counted as the requirement a rename or move took it to and left out when no move reaches one the spec holds, and no marker cites one it removed that the spec doesn't hold (`check.coverage/status-report`). A later change can remove what it added or add back what it removed, and neither keeps it from completing. A change that hasn't applied is never complete, since its modified requirements are still the old text. `clm rebase` archives a complete change on any `impl/*` branch (`archive-on-sync`), and `clm wrap-up` names those still unarchived on `origin/main`, for the build loop to archive (`orchestrator.loop`).

`queue_from_refs` and `taken_from_refs` return the queue and what holds backlog items from refs already loaded, without git or a checkout. `queue_from_refs` takes the graph, the markers, the `impl/*` branches with their tip commit times, the current time and the expiry. `taken_from_refs` takes `main`'s graph, the `spec/*` and `autosync/spec/*` refs, each with its tip's `changes/<id>/change.md` text, or, when the tip has none, its `changes/archive/<id>/change.md` text (or none), and the names under that change's `deltas/` (`archived-spec-work`): the caller chooses the file, so `SpecRef` keeps its fields and `taken_from_refs` reads the text the same whichever path it came from, the `propose/*` refs with their tip commit times, the current time and the expiry. `land_priority(root, branch, base, head)` gives `land` the priority a land of `branch` carries, which orders the clone's land queue (`land-priority`, `land/lock-queue`). With its default features disabled, the crate builds for `wasm32-unknown-unknown`, so the `web` Worker can use it.

## MODIFIED Requirements
### Requirement: next <!-- base: f8bdf51391b29dfd -->
`clm next` SHALL print each component that is `ready` and unclaimed, one component ID per line, sorted by effective priority (`P1` first, `priority-orders-nodes`), then by ID, where a component's state is as in the Design: it is ready when it has a pending requirement that neither a backlog item nor a pending requirement it needs blocks, nor, for an obsolete one, a pending requirement a move took it to (`check.coverage/pending-report`), whatever the state of the components in its `depends_on` (`needs-alone-order`). A claim whose branch tip is older than the expiry, or is an ancestor of `origin/main`, SHALL count as unclaimed. Priority SHALL only order the output: it never makes a component ready or holds one back. It SHALL print at most `max_builds` minus the number of places that count (`places`), the most urgent first, and nothing when that is zero or less; the places SHALL NOT change which components are ready, nor what `clm claim` accepts. When it prints nothing because that is zero or less while at least one component is ready and unclaimed, it SHALL write the one line `no free build place: <held> of max_builds = <cap> in use, <n> ready` to stderr, `<held>` the number of places that count, `<cap>` `max_builds` and `<n>` the number of ready, unclaimed components, and still exit 0, so a caller tells a full cap from an empty queue; with no component ready it SHALL write nothing to stderr, however many places count.

#### Scenario: dependency doesn't order
- GIVEN components `a` and `b`, where `a` depends on `b`, each with a requirement no test cites
- WHEN `clm next` runs
- THEN it prints `a` and `b`

#### Scenario: behind requirement
- GIVEN component `b` whose only requirement `b/x` is cited at version `11111111` while its current version is `22222222`
- WHEN `clm next` runs
- THEN it prints `b`

#### Scenario: needed requirement first
- GIVEN components `a` and `b` with no `depends_on` between them, `a`'s only pending requirement `a/x` with `#### Needs` naming `b/y`, and `b/y` uncited
- WHEN `clm next` runs
- THEN it prints `b` and not `a`

#### Scenario: needed requirement current
- GIVEN the same components, after a test cites `b/y` at its current version
- WHEN `clm next` runs
- THEN it prints `a`

#### Scenario: need against the node graph
- GIVEN component `b` depending on `a`, `a` with pending requirements `a/x`, which needs uncited `b/y`, and uncited `a/w`
- WHEN `clm next` runs
- THEN it prints `a` and `b`

#### Scenario: blocked node holds nothing up
- GIVEN component `a` whose only pending requirement `a/x` needs uncited `b/y`, and component `c` depending on `a` with an uncited requirement
- WHEN `clm next` runs
- THEN it prints `b` and `c`, and not `a`

#### Scenario: live and lapsed claims
- GIVEN pending components `b` and `d`, `origin/impl/b` committed 1 hour ago, and `origin/impl/d` committed 5 hours ago
- WHEN `clm next` runs
- THEN it doesn't print `b`, and prints `d`

#### Scenario: merged claim
- GIVEN pending component `b`, and `origin/impl/b` committed 1 hour ago and merged into `origin/main` with a merge commit
- WHEN `clm next` runs
- THEN it prints `b`

#### Scenario: higher priority first
- GIVEN pending components `a` and `b` on disjoint parts of the graph, `b`'s pending requirement added by a live change with `priority: P2` and `a`'s by one with no priority
- WHEN `clm next` runs
- THEN it prints `b` then `a`

#### Scenario: capped by places
- GIVEN ready, unclaimed components `a`, `b` and `c` on disjoint parts of the graph, `max_builds` 3, and one place that counts, `d` for a live claim `origin/impl/d`
- WHEN `clm next` runs
- THEN it prints `a` and `b` and not `c`

#### Scenario: no place free
- GIVEN ready component `a`, `max_builds` 1, and one place that counts
- WHEN `clm next` and then `clm claim a` run
- THEN `clm next` prints nothing to stdout, writes `no free build place: 1 of max_builds = 1 in use, 1 ready` to stderr and exits 0, and `clm claim a` exits 0

#### Scenario: full cap, empty queue
- GIVEN no component ready, `max_builds` 1, and one place that counts
- WHEN `clm next` runs
- THEN it prints nothing to stdout or stderr and exits 0


#### Scenario: places shared by worktrees
- GIVEN ready components `a` and `b`, `max_builds` 2, a place that counts taken from the main worktree, and the current directory a linked worktree of the clone
- WHEN `clm next` runs
- THEN it prints `a` and not `b`

#### Scenario: split in flight
- GIVEN components `a` and `a.b`, an `applied` change whose delta for `a` removes `a/x` and whose delta for `a.b` adds `a.b/x`, a test citing `a/x`, `a.b/x` uncited, and every other requirement of `a` current
- WHEN `clm next` runs
- THEN it prints `a.b` and not `a`

#### Needs
- config/max-builds-setting
- view/blockers-view
- view/status-report
- check.coverage/pending-report

### Requirement: fetch-retries-race <!-- base: ba70b96bfe415968 -->
Every fetch of `origin` that `clm-orchestrator` makes, in `clm next`, `clm claim`, `clm claim backlog/<slug>`, `clm proposals`, `claim_local` and `taken_items`, SHALL go through `clm-git`'s `fetch_retrying` (`vcs/fetch-retries`) with the arguments it fetches with today (`fetch --prune --quiet origin`), so a fetch that lost a race with a build in another worktree of the same clone over a remote-tracking ref goes on. When `fetch_retrying` fails, the command SHALL fail as a failed fetch fails it today, as a git failure (exit 2), with the error's message: git's message from the last fetch for `FetchError::Git`, and `origin can't be reached` for `FetchError::Offline`. No file under `crates/clm-orchestrator/src` SHALL run `git fetch` other than through `fetch_retrying`.

#### Scenario: a claim whose fetch loses a ref race
- GIVEN a ready, unclaimed component `b`, and an executable `git` first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/impl/a'` on stderr and exits 1, and runs the real `git` for every other call
- WHEN `clm claim b` runs
- THEN it prints `impl/b`, exits 0, and `origin` holds `impl/b`

#### Scenario: a claim whose fetch keeps failing
- GIVEN the same, with a `git` that fails every `fetch` naming `origin` that way
- WHEN `clm claim b` runs
- THEN it exits 2 with a message containing `cannot lock ref`, and `origin` holds no `impl/b`

#### Needs
- vcs/fetch-retries

#### Serves
- capability/work-concurrently

### Requirement: next-wait <!-- base: bd4b8159181127e9 -->
`clm next` SHALL refuse `--wait` and `--interval` as usage errors, and SHALL hold no code that waits for work: `clm build wait` waits for everything the build loop launches (`orchestrator.loop/build-wait`), and the skill that ran `clm next --wait` runs the loop instead. This is the contract step of moving the wait into the loop (`expand-contract`), so it waits until the skill no longer names `clm next --wait`; while it waits, this pending requirement stands in for a `clm:remove-by 20261112-build-loop-in-clm` marker on the `--wait` code, which `clm pending` keeps in sight until it is built.

#### Scenario: wait refused
- WHEN `clm next --wait` runs, and then `clm next --interval 5`
- THEN each exits 2 with usage

#### Serves
- capability/build-from-spec

#### Needs
- plugin.build/build-runs-the-loop
- orchestrator.loop/build-wait
