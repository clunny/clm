---
title: Orchestrator
status: active
owners: ["@clunny"]
depends_on: [model, check, context, gate, vcs]
paths: ["crates/clm-orchestrator/**"]
successor: ""
serves: [capability/build-from-spec, capability/work-concurrently, architecture/component-reconciliation, architecture/lease-claims]
---

## Contract
`clm next` lists the components whose code is ready to be brought up to their spec, one component ID per line, most urgent first, at most as many as the clone has free build places, and nothing when it has none (`build-places`). `clm next --wait [--interval <minutes>]` looks again every interval, fetching each time, until it would print a component, then prints it and exits 0, so a session can wait for work in one background command (`next-wait`). `clm claim <component>` claims a component by pushing branch `impl/<component>` to `origin` and prints the branch name. `clm claim <component> --fix backlog/<slug>` claims it for a code-only fix of that `bug` item, or for the code-only work of that `deferred` item (`code-fix-resolves-bug`, `deferred-work-resolves-in-code`), even when the component has nothing pending, and switches the checkout to the claimed branch, or refuses before claiming when it couldn't. Both fetch `origin` first and take `--expiry <hours>` (default 4). When they refuse a component, they say why. Both exit 0 on success, 1 when the work isn't possible (with the reason), and 2 when git fails. `clm status` ends with a `claims` section listing claims, component and backlog alike, as of the last fetch. Launching a build from a claim (`clm run`) is `orchestrator.run`; ending a session (`clm wrap-up`) is `orchestrator.checkpoint`; triaging the backlog into unattended drafts (`clm triage`, `clm draft`) is `orchestrator.triage`. Each builds on the queue this component defines and calls `git` for its plumbing. For them the crate exposes, as library calls: `claim` (a claim with its refusal reason), `claim_local` (a component claim that records the local branch `impl/<component>` before it pushes, for `clm run`: `claim-local`), `refusal(component, state)`, `propose_refs`, `claim_refs`, `proposals` (what `clm proposals` prints), `taken_items(root, expiry)` (what holds backlog items, as the Design's *taken* defines, read from `origin` after fetching: each proposed or on-main change with its state, the items it names and its components, and each live bare claim with its item), `queue_from_refs` and `taken_from_refs` (below), the place calls below and `Error`. Changing one of these is a change to this Contract.

**Build places** (`build-places`). A build or a draft holds a *place* on the clone from when `clm run` or `clm draft` starts it until it lands, is finished with `--finish`, or its claim lapses, and the clone has `config`'s `max_builds` of them (default 3). A place is the file `<git common dir>/clm/builds/<name>`, in the clone's git common directory (`git rev-parse --git-common-dir`), which every worktree of the clone shares, a bare repository's included: `<name>` is the component ID for a build and the change ID for a draft, and the file names the branches the place belongs to. It is local state of the clone, never committed, since nothing under the git directory is. A place *counts* unless its branch has landed or its claim has lapsed and it was taken longer ago than the expiry, as the Design defines; one that doesn't count is free to take and is deleted by the next command that takes a place, and by `clm wrap-up`. The cap counts this clone only. Each place holds a *slot*, a number from 1 up that no other place that counts holds, so a build and its checks can key the host resources its tests take, such as a port, on it (`build-slots`, `tests-run-concurrently`); 0 is the slot of whatever runs outside a place, such as the human's own checkout. For `orchestrator.run`, `orchestrator.triage` and `orchestrator.checkpoint` the crate exposes `places(root, expiry)` (every place with its name, kind, branches, slot, the time it was taken and whether it counts, judged on the remote-tracking refs as they are: it doesn't fetch, so its callers fetch first), `take_place(root, place, max_builds, expiry)` (takes a place, or reuses the one of that name when it counts, or says the clone is full with the number held and the cap), `place_slot(root, branch)` (the slot of the place that names the branch, or 0, failing only when the git common directory can't be found), `rename_place(root, from, to)` (renames a place, or says `to` already exists), `free_place(root, name)` (deletes the file, succeeding when there is none) and `free_stale_places(root, expiry)` (under the places lock, reads the places, deletes each one that doesn't count, and returns a `Swept` for every place as judged there: the `Place` and its outcome, kept, freed, or failed with why the file couldn't be deleted).

The unit of building is the component, not the change (`node-reconciliation`). `specs/` on `main` holds every applied change's text, and a component's code is brought up to all of it at once. A requirement is *current* when a test cites it at its current version, `req:<id>@<version>` (`versioned-markers`, `check.coverage/pending-report`); otherwise it is *pending*: `unbuilt` when nothing cites it, `behind` when only older versions are cited, or `obsolete` when a marker cites a requirement a live change removed. A component is *pending* while it has a pending requirement. A backlog item *blocks* the requirements it names in `blocks` (`requirement-blocks`), and a pending requirement *blocks* the requirements of other components whose `Needs` name it (`requirement-needs`): a blocked requirement stays pending but is left out of the component's work until the item is deleted or the needed requirement is current. A component is ready when it has an unblocked pending requirement. Nothing else orders the work (`needs-alone-order`): `depends_on` says which components' code a component may use, not when it is built, so a requirement that relies on another component's unbuilt work names it in `Needs`; there are no plans, no order between changes and no waits written by hand against time or other changes. A `Needs` is a dependency the spec declares and the markers decide. Priority orders `clm next` (`priority-orders-nodes`).

A proposing session claims a backlog item before designing its fix (`propose-claims`). `clm claim backlog/<slug>` pushes branch `propose/<slug>` to `origin` the way a component claim pushes `impl/*`, with the same expiry and takeover, and prints the branch name. It refuses an item that isn't in `origin/main`'s `backlog/`, or that is already taken: claimed, or named in the `from` or `resolves` of spec work in flight on `origin`. `clm proposals` lists that spec work and the bare claims, each with its components and items, so a session can see what another is already changing. Backlog claims never enter `clm next`.

An `applied` or `implementing` change is *complete* when every requirement its deltas add or modify is current, each counted as the requirement a rename or move took it to and left out when no move reaches one the spec holds, and no marker cites one it removed that the spec doesn't hold (`check.coverage/status-report`). A later change can remove what it added or add back what it removed, and neither keeps it from completing. A change that hasn't applied is never complete, since its modified requirements are still the old text. `clm rebase` archives a complete change on any `impl/*` branch (`archive-on-sync`), and `clm wrap-up` names those still unarchived on `origin/main`, for the `build` skill to archive.

`queue_from_refs` and `taken_from_refs` return the queue and what holds backlog items from refs already loaded, without git or a checkout. `queue_from_refs` takes the graph, the markers, the `impl/*` branches with their tip commit times, the current time and the expiry. `taken_from_refs` takes `main`'s graph, the `spec/*` and `autosync/spec/*` refs, each with its tip's `changes/<id>/change.md` text, or, when the tip has none, its `changes/archive/<id>/change.md` text (or none), and the names under that change's `deltas/` (`archived-spec-work`): the caller chooses the file, so `SpecRef` keeps its fields and `taken_from_refs` reads the text the same whichever path it came from, the `propose/*` refs with their tip commit times, the current time and the expiry. With its default features disabled, the crate builds for `wasm32-unknown-unknown`, so the `web` Worker can use it.

## Design
The queue holds every component of the graph. For each component:
- Its **requirement states** come from `check`'s coverage with versions (`check.coverage/pending-report`): `current`, `behind`, `unbuilt` or `obsolete`. Its *pending* requirements are those that aren't current, the obsolete ones included, since an obsolete marker names the component in its ID; one whose ID names a component that doesn't exist counts on the component `check` gives it (`check.coverage/pending-report`).
- A requirement is **blocked** while a backlog item that exists names it in `blocks` (`requirement-blocks`), or while a requirement its `Needs` names is pending (`requirement-needs`), or, when it is obsolete, while a requirement a move took it to is pending (`moved-markers-wait`): in a split in flight the old component's obsolete markers are the moving component's work, so the old component isn't offered for them. A pending capability is blocked while a rule serving it is `unbuilt`, or `behind` and itself blocked (`serving-rules-order-capabilities`). A needed requirement that is itself blocked still blocks, since it is pending; what releases the dependant is then whatever blocks the needed requirement, and the blockers view follows the chain there (`view`). The test is one pass over `check`'s requirement states: a requirement is blocked by a need exactly when the needed requirement is pending, whatever blocks that one. Needs can't cycle (the model rejects a cycle) and never name the same component, so a needed requirement is always built by another component's session, or released when its own blocker goes.
- The component is `converged` when it has no pending requirement; `blocked` when it has pending requirements and all of them are blocked, with its blockers: each a backlog item (`item`, the slug) or a pending requirement of another component (`requirement`, as `<component>/<slug>`), with the requirements it blocks. In the serialised state, each `by` entry sets exactly one of `item` and `requirement`. A pending requirement carries its item blockers in `blocked_by` (slugs) and everything else it waits on in `waits_on` (`<component>/<slug>`): the pending requirements its `Needs` name, for an obsolete requirement the pending ones a move took it to, and for a capability the serving rules that hold it back, all as `check`'s pending list gives them (`view`'s JSON rule); otherwise the component is `ready`, or `claimed` while `origin/impl/<component>` holds a live claim, with the time the claim lapses. `depends_on` plays no part in the state.
- **Effective priority** (`priority-orders-nodes`). Urgent work pulls the components it needs forward, as a scheduler avoids priority inversion. A pending requirement blocked by a wait seeds its priority on the component of each pending requirement it waits on, whatever that component's state and whether an item blocks the requirement too, and the seeding follows the chain of waits to the work that holds it up. Every entry of `waits_on` is a wait: a `Needs`, the requirement a move took an obsolete one to, and a rule serving a capability that holds it back. So a capability added or edited by an urgent change pulls forward every component holding an `unbuilt` rule that serves it, and, through a changed serving rule that waits, the work that rule waits on, and a component whose `serves` frontmatter names the capability makes every rule on it serve (`model/serves`): the capability's test can't pass until those rules are built, so they are the work that holds the urgent change up. Priority only orders: it never makes a component ready or holds one back.
- **Complete changes** are listed by `queue_from_refs`, from the coverage the queue already reads, so `clm wrap-up` and the views can list them.

`next` and `claim` read the graph and the markers from `origin/main` after fetching, not from the checkout: they extract its tree with a temporary index (`git read-tree` and `git checkout-index` under `GIT_INDEX_FILE`), so every session sees the same state whatever branch it has checked out, and neither the working tree nor the real index is touched. `queue_from_refs` applies the same rules to a graph and markers already loaded from that tree, so `next` is built on it.

A component is claimed while `origin/impl/<component>` exists, its tip was committed within the expiry, and its tip isn't an ancestor of `origin/main` (`merged-claims-released`). A claim whose tip is in `origin/main` is *merged*: its branch has landed, and it is released at once, as a lapsed claim is. A claim is always a new commit on top of `origin/main`, so a claim tip reaches `origin/main` only when its branch lands as a merge. The test is `git merge-base --is-ancestor` on the refs the fetch just updated, so `next` stays a git-only command. A ref `origin/impl/<x>` is read as a claim only when `<x>` holds no `/`, since a component ID never contains `/` (`nested-branches-not-claims`): `build` archives complete changes on `impl/archive/<id>` (`archive-branches`), and that branch never reads as a claim on a component. `clm claim` and `clm run` refuse a component that isn't ready with the reason the queue gives it.

Claiming makes an empty commit on `origin/main` (`git commit-tree`, so the working tree is untouched) and pushes it with `--force-with-lease`: expecting no branch for a new claim, or the released tip, lapsed or merged, for a takeover. Git's compare-and-swap makes the claim atomic, so of two racing sessions exactly one wins. A rejected push isn't always a lost race, so `clm claim` then reads the branch's tip on `origin` again (`git ls-remote`) and judges it as `claim` says. A tip still unchanged means the push failed for another reason: a lock held by a parallel push from the same repo, or a transient error on the remote, such as a forge's `remote: fatal error in commit_refs` while other sessions push at the same moment. The same commit is pushed again, since the lease still guards the swap; the waits are fixed, with no override. The line saying the failure may be transient and that running the same command again is safe is true because the tip shows nothing was claimed: `clm claim` claims afresh and `clm run` claims or resumes the component as its Design says (`orchestrator.run`). Only that last rejection gets the transient line: when `ls-remote` fails, the push may have gone through, and a rerun would then find the claim and refuse it as `claimed`, so that git failure is reported with git's reasons alone. `clm claim backlog/<slug>`, `--fix` claims and `clm run` share this handling. A failing `ls-remote` is a git failure too. The implementer keeps the claim alive by pushing commits.

**Places** (`build-places`). Each build compiles into its own worktree's `target/`, which takes many GB, so how many run at once is what bounds the disk; free space can't, since under WSL the filesystem's free space isn't the host's. The directory is `<git common dir>/clm/builds/`, the common directory found through `git` (`git rev-parse --git-common-dir`, made absolute) and created when missing. Every worktree of the clone shares that directory, so a place taken from a linked worktree, where builds and the orchestrating session often run, is seen from every other, it works for a bare repository with linked worktrees, and it needs no lookup of the main worktree. It holds places for the whole git repository, so two CLM repositories in subdirectories of one clone share one cap. A place file is UTF-8 text, one `key value` pair per line: `kind build` or `kind draft`, `taken <RFC 3339 time>`, a `slot <n>` line (`orchestrator/place-slots`) right after it that a file written before slots lacks, and one `branch <name>` line per branch it belongs to: `impl/<component>` for a build; `spec/<id>` then each `propose/<slug>` the draft claimed, in the order claimed, for a draft. `clm draft` takes its place before it claims and before it knows the change ID, under a provisional name unique to that invocation, `draft-<pid>-<first slug given>` with its process ID, which is never reused, and with no `branch` line; it renames it with `rename_place` once it has claimed (`orchestrator.triage`); a draft place with no `branch` line has no claim and hasn't landed, so it counts by its `taken` time alone. A file that can't be read or parsed counts, so a damaged file never lets a clone exceed its cap. `places` doesn't fetch: every command that reads places (`clm run`, `clm draft`, `clm next`, `clm wrap-up`, and the `--finish` commands) fetches first, so the reads are on the remote-tracking refs it just updated:
- A build's place has *landed* when at least one of `origin/impl/<component>` and the local `impl/<component>` exists and the tip of each that exists is an ancestor of `origin/main` (`clm land` deletes the remote branch, and the local one stays in the build's worktree); a place whose component has neither branch yet, such as one `clm run` has taken but not yet claimed for, hasn't landed. Its claim is *live* while `origin/impl/<component>` is a live claim as above.
- A draft's place has *landed* when `origin/main`'s tree holds `changes/<id>/` or `changes/archive/<id>/`; its claim is *live* while any `propose/<slug>` it names is a live backlog claim, or `origin/spec/<id>` or `origin/autosync/spec/<id>` exists with a tip committed within the expiry.
- A place *counts* when it hasn't landed and either its claim is live or it was taken within the expiry. The second arm covers a build that resumes a branch whose claim has lapsed: its place counts from the moment it is taken, before the build's first push renews the claim.

*Taking a place* is done under an exclusive lock on `<git common dir>/clm/builds.lock` (an advisory lock of the operating system, so a killed command releases it), so two commands started at once never both take the last place: under the lock the command reads the places, deletes those that don't count, and then, when a place of its own name counts, reuses it (rewriting `taken` to now), when fewer than `max_builds` count, writes its file, with the smallest slot no counting place holds, through a temporary file and a rename (a reused place with no slot line gets one by the same rule; `place_slot` reads a place's slot without the lock), and otherwise takes nothing. `rename_place`, `free_place` and `free_stale_places` take the same lock, so a draft's provisional place becomes its final one atomically and never overwrites another place. `free_stale_places` is the read-and-delete half of that on its own: no read and delete are ever split across the lock, so a place another command retakes between a read and a delete is never deleted. When it can't delete a file it records why and goes on to the next, while a failure to find the git common directory or take the lock is the call's `Error`; `take_place` shares its read and judgement but fails the call when a delete fails. `free_place` deletes by name without judging, for a command freeing its own place (`--finish`, a refused claim). The command claims after taking the place, outside the lock, and frees a place it newly took when it then claims nothing, so a refused claim never holds a place. `clm next` takes no place: it prints at most `max_builds` minus the places that count, so a session launching all it prints takes them all. The places are this clone's only: two clones on one machine, or a build on another machine, never count against each other. This crate doesn't depend on `config`: `crates/clm` loads `max_builds` and passes it to `clm next`, as it passes settings to the other commands. The place calls read files and git, so they are behind the crate's default features with its git calls, and the `wasm32-unknown-unknown` build has none. Its test starts the racing takers as separate processes, never threads, since an advisory file lock is held per process on some systems; a failed delete is tested, where the platform allows, by making `builds/` read-only, on Unix and not as root.

**A claim recorded locally first** (`claim-local`). `clm run` claims through `claim_local`, so its clone keeps a trace of every claim it may have pushed (`claim-local-before-push`). Once it has made the claim commit, and before the first push, `claim_local` creates the local branch `impl/<component>` at that commit with `git branch`, which refuses a branch that exists, so it never moves one, and sets the branch's `remote` and `merge` settings to `origin` and `refs/heads/impl/<component>`, since the remote-tracking ref `git branch --track` needs doesn't exist until the push. A command interrupted after the push has therefore always left the local branch at the claim, which the next `clm run` resumes (`orchestrator.run`'s `run`); one interrupted before it leaves a local branch whose tip is a claim commit nobody pushed, which the next `clm run` removes. When the claim fails and the tip `ls-remote` reads again isn't the claim commit (a lost race, or a push still rejected after the retries), nothing was claimed, so it deletes the local branch with `git update-ref -d`, expecting the claim commit; when `ls-remote` itself fails, the push may have gone through, so the branch stays for the next `clm run` to judge after its fetch. A refusal before the commit is made (the component isn't ready, or is claimed) writes nothing. `claim` writes no local branch: a plain `clm claim` leaves the clone as it was, and `--fix` sets its branch after the push as below.

**Fix claims** (`code-fix-resolves-bug`). `clm claim <component> --fix backlog/<slug>` claims `impl/<component>` for a fix that changes no spec, which `propose` builds in its own session. A component's state in the queue says whether it has spec to catch up with; a fix has none, so the claim ignores the state, except that a component another session holds is refused as `claimed`, as always. The branch is an ordinary `impl/<component>` claim: the queue reads it as `claimed` while it lives, `clm land` deletes it after the merge, and `clm wrap-up` treats it as a build. A ready component claimed this way waits for the fix to merge before its build. `--fix` takes no other component state into account and is the only way a converged component is claimed.

**A fix claim switches the checkout** (`fix-claims-switch`). The session that claims a fix builds it in the checkout it claimed from, so `clm claim <component> --fix` leaves that checkout on `impl/<component>` at the claim commit, with `origin/impl/<component>` as its upstream, and prints the branch name only once it is there. A plain `clm claim <component>` leaves the checkout alone: `clm run` builds in a worktree of its own (`orchestrator.run`). Before it pushes anything, after the fetch, the item's checks and the refusal of a live claim, in that order, the fix claim checks that the switch can't fail, and otherwise exits 1 without pushing, saying which of these holds and what to do:
- the checkout has uncommitted changes to tracked files (`git status --porcelain --untracked-files=no` prints something): commit them or set them aside, so a spec edit never rides onto a code branch; untracked files are carried along, as `git switch` does;
- another worktree has a local `impl/<component>` checked out, naming that worktree's path from `git worktree list` (`clm-git`'s `worktrees`): switch it off the branch or remove it (`git worktree remove <path>`), then claim again; when that path doesn't exist, the remedy is `git worktree prune`. The checkout's own entry is told apart by comparing canonicalized paths with `git rev-parse --show-toplevel`. This is the stale branch a landed build leaves in its worktree, which `clm land` can't delete while it is checked out (`land/land`). The fix claim never moves a branch another worktree has checked out, and never checks out detached, since a detached build has no branch for `clm land` to land;
- a local `impl/<component>` exists whose tip isn't an ancestor of `origin/main`: it holds commits `main` doesn't, naming the branch, so the session pushes or deletes it first (`git branch -D impl/<component>`). A leftover claim commit from a lapsed claim counts too: it is cheap to delete, and a fix claim is an attended step, so telling an empty claim from work isn't worth a rule here, though `clm run`, which a build launches unattended, has one (`orchestrator.run`'s `run`).
Otherwise a local `impl/<component>`, if any, holds nothing `main` lacks. After the push the claim sets `impl/<component>` to the claim commit (`git branch -f`) with `origin/impl/<component>` as upstream, then switches to it, so a rejected ref update fails before the working tree moves; when the checkout is already on `impl/<component>`, `git branch -f` refuses the current branch, and `git switch -C` resets it instead. If the switch still fails after the push, as when another worktree checks the branch out in between or an untracked file would be overwritten, the claim stands, and the error gives the command to finish by hand (`git switch -C impl/<component> origin/impl/<component>`), so nothing fails silently.

**Backlog claims** (`propose-claims`). `clm claim backlog/<slug>` is told apart from a component claim by its first part: a component ID never contains `/`. It fetches, reads `origin/main`'s tree as `next` does, and claims `propose/<slug>` with the same commit, lease and rejection handling as a component claim, so of two racing sessions exactly one wins. The proposing session keeps nothing alive on that branch: once its spec branch names the item, the spec work holds it, however long its PR waits. An item `<slug>` is *taken* when any of these holds:
- **claimed**: `origin/propose/<slug>` exists and its tip was committed within the expiry;
- **on main**: a change outside the archive in `origin/main`'s tree lists `<slug>` in its `resolves`;
- **proposed**: it is named in the `from` or `resolves` of `changes/<id>/change.md` in the tree of `origin/spec/<id>` or `origin/autosync/spec/<id>`, for a change `<id>` that isn't under `changes/` or `changes/archive/` in `origin/main`. Spec work counts whatever its age, since a branch that needs review can wait days for approval. Abandoned work holds its items until its branches are deleted, and `clm proposals` names it so the human can tell. When both refs exist, their items and components are joined. A tip with no `changes/<id>/change.md` is read through `changes/archive/<id>/change.md` and `changes/archive/<id>/deltas/` when it holds them (`archived-spec-work`): `clm rename` applies and archives its change on its own branch (`rename-by-replay`), so a rename branch waiting for review holds its change only there, and its work is as much in flight as any other spec branch's until it lands. Any archived change of the ref's own ID counts, not only one recording `renames`: the archive on an unlanded spec branch is that branch's work whatever made it, and telling renames apart would add a rule that changes nothing today. A ref with neither file is skipped.

*Named in `from`* means `from` is `backlog/<slug>`; `resolves` holds bare slugs. Only those two frontmatter keys are read, and other keys are ignored, so a branch that adds a new frontmatter field still counts. A `change.md` whose frontmatter doesn't parse names no items; `clm proposals` still lists the change with its delta components and prints a warning naming the ref on stderr. Items, and their `components`, are looked up in `origin/main`'s `backlog/`, since the spec branch may already have deleted its `from` item; a name not there adds no components.

The session's own change is left out, so it never trips over its own work: when the checkout is on `spec/<id>`, change `<id>` is left out of *proposed*, and the items its working-tree `changes/<id>/change.md` names, or `changes/archive/<id>/change.md` when there is no live one, are left out of *claimed*. A lapsed claim is taken over when nothing else holds the item. It takes `--expiry`.

**`clm proposals [--component <id>]...`** lists each change that holds an item or is proposed, then each claim no such change names (`proposals`). For a change on main, `<components>` are the components it has deltas for, and the items are those of its `resolves` still in `backlog/`; for proposed spec work, `<components>` are those of its `deltas/<component>.md` joined with the `components` of the backlog items it names, so overlap shows before any delta is written; for a claim, they are the item's `components`. *On main* means outside `changes/archive/`. Components are comma-separated and sorted, as in `clm backlog`, and an empty list is `[]`.

`clm status` and `clm view` (`view`) don't fetch, so they stay offline and fast. They read the remote-tracking refs as they were at the last fetch; `clm next` and `clm proposals` are the authoritative views.

## Requirements

### Requirement: next
`clm next` SHALL print each component that is `ready` and unclaimed, one component ID per line, sorted by effective priority (`P1` first, `priority-orders-nodes`), then by ID, where a component's state is as in the Design: it is ready when it has a pending requirement that neither a backlog item nor a pending requirement it needs blocks, nor, for an obsolete one, a pending requirement a move took it to (`check.coverage/pending-report`), whatever the state of the components in its `depends_on` (`needs-alone-order`). A claim whose branch tip is older than the expiry, or is an ancestor of `origin/main`, SHALL count as unclaimed. Priority SHALL only order the output: it never makes a component ready or holds one back. It SHALL print at most `max_builds` minus the number of places that count (`places`), the most urgent first, and nothing when that is zero or less; the places SHALL NOT change which components are ready, nor what `clm claim` accepts.

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
- THEN `clm next` prints nothing and exits 0, and `clm claim a` exits 0

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

### Requirement: claim
`clm claim <component>` SHALL push a new commit on `origin/main` to `impl/<component>` atomically, failing if the branch appeared or moved since it was read. It SHALL exit 1 without pushing when the component isn't one `clm next` would print, saying why: the items and requirements that block it, that it is converged, or that it is claimed. When it is converged, it SHALL also say that a small change needing no spec change takes no claim and lands from a branch of its own with `clm land`. It SHALL take over a lapsed claim, and a claim whose tip is an ancestor of `origin/main`. When the push is rejected, it SHALL read the branch's tip on `origin` again. If the tip is the claim commit, it SHALL succeed. If the tip differs from the one it expected (none for a new claim, the released tip for a takeover), it SHALL exit 1 saying someone else claimed the component first. Otherwise it SHALL push the same commit again, at most twice more, waiting 1 second before the first of these and 2 seconds before the second, and judge each rejection the same way, except that a tip still unchanged after the last push SHALL exit 2 with git's reason and a line saying the failure may be transient and to run the same command again, without saying the component was claimed by someone else.

#### Scenario: race
- GIVEN two sessions that both see component `b` as ready
- WHEN both run `clm claim b`
- THEN exactly one exits 0 and the other exits 1

#### Scenario: already claimed
- GIVEN `origin/impl/b` committed 1 hour ago
- WHEN `clm claim b` runs
- THEN it exits 1 and pushes nothing

#### Scenario: takeover
- GIVEN `origin/impl/b` committed 5 hours ago
- WHEN `clm claim b` runs
- THEN it exits 0 and the branch tip is the new claim commit

#### Scenario: merged takeover
- GIVEN `origin/impl/b` committed 1 hour ago and merged into `origin/main` with a merge commit
- WHEN `clm claim b` runs
- THEN it exits 0 and the branch tip is a new claim commit whose parent is `origin/main`

#### Scenario: rejected for another reason
- GIVEN an `origin` whose `pre-receive` hook rejects every push, printing `declined by test`
- WHEN `clm claim b` runs
- THEN the hook ran three times, it exits 2, its message contains `declined by test` and `may be transient` and not `claimed by someone else`, and `origin` has no `impl/b`

#### Scenario: rejected twice
- GIVEN an `origin` whose `pre-receive` hook rejects the first two pushes, printing `fatal error in commit_refs`
- WHEN `clm claim b` runs
- THEN it exits 0 and `origin/impl/b` is the claim commit

#### Scenario: why it isn't ready
- GIVEN component `c` whose only pending requirement `c/x` is in the `blocks` of an existing `backlog/q.md`, and component `d` with every requirement current
- WHEN `clm claim c` and `clm claim d` run
- THEN each exits 1, the first naming `backlog/q`, and the second saying `d` is converged and naming `clm land` for a change that needs no spec change

#### Scenario: pending dependency
- GIVEN component `a` with an uncited requirement, depending on component `b` with an uncited requirement
- WHEN `clm claim a` runs
- THEN it exits 0

#### Scenario: blocked by a need
- GIVEN component `a` whose only pending requirement `a/x` needs uncited `b/y`
- WHEN `clm claim a` runs
- THEN it exits 1 without pushing, saying `a` is blocked by `b/y`

#### Needs
- view/blockers-view
- view/status-report

### Requirement: status-claims
`clm status` SHALL end with a `claims` section, laid out as `view`'s `status-report` says: one row per `refs/remotes/origin/impl/<component>` ref, sorted, with the cells `<component>` and `merged` when its tip is an ancestor of `refs/remotes/origin/main`, otherwise `live` when its tip was committed within the default expiry and `lapsed` otherwise, then one row per `refs/remotes/origin/propose/<slug>` ref, sorted, with `backlog/<slug>` and `live` or `lapsed` by the expiry alone, or the single row `none` when there are no such refs or the root isn't a git repository. It SHALL NOT fetch.

#### Scenario: live and lapsed claims
- GIVEN remote-tracking refs `origin/impl/b` committed 1 hour ago and `origin/impl/d` committed 5 hours ago
- WHEN `clm status` runs
- THEN its `claims` rows are `b`, `live` then `d`, `lapsed`

#### Scenario: merged claim
- GIVEN remote-tracking refs `origin/impl/b` committed 1 hour ago, and `origin/main` a merge commit with `origin/impl/b` as its second parent
- WHEN `clm status` runs
- THEN its `claims` row is `b`, `merged`

#### Scenario: backlog claims after node claims
- GIVEN remote-tracking refs `origin/impl/b` and `origin/propose/q`, both committed 1 hour ago
- WHEN `clm status` runs
- THEN its `claims` rows are `b`, `live` then `backlog/q`, `live`

#### Scenario: no claims
- GIVEN a repository with no `origin/impl/*` or `origin/propose/*` refs
- WHEN `clm status` runs
- THEN its `claims` section has the single row `none`

#### Serves
- capability/see-project-state
- capability/work-concurrently
- architecture/git-is-truth

### Requirement: queue-from-refs
`queue_from_refs` SHALL return, for every component of the graph, one of:
- `converged`;
- `ready`, with its pending requirements (`queue-pending-entries`);
- `claimed`, with the time its claim lapses;
- `blocked`, with what blocks its pending requirements, each blocker a backlog item or a pending requirement of another component (`requirement-needs`), and which requirements it blocks.

A component's `depends_on` SHALL NOT affect its state (`needs-alone-order`), and there SHALL be no `waiting` state.

#### Scenario: every state
- GIVEN components `a` depending on `b`, where `b` is converged and `a` has an uncited requirement, `e` with an uncited requirement and claimed by a branch pushed an hour ago, and `f` whose only pending requirement `f/x` is in the `blocks` of an existing `human-task` backlog item
- WHEN `queue_from_refs` runs with a 4-hour expiry
- THEN it reports `b` converged, `a` ready, `e` claimed lapsing in 3 hours, and `f` blocked by that item, blocking `f/x`

#### Scenario: blocked by a need
- GIVEN component `a` whose only pending requirement `a/x` needs `b/y`, uncited on component `b`, which `a` doesn't depend on
- WHEN `queue_from_refs` runs
- THEN it reports `a` blocked by `b/y`, blocking `a/x`, and `b` ready

#### Needs
- check.coverage/pending-report
- view/blockers-view
- view/status-report
- model/parse-backlog

### Requirement: orchestrator-builds-for-wasm
The `clm-orchestrator` crate SHALL build for `wasm32-unknown-unknown` with its default features disabled.

#### Scenario: wasm build
- GIVEN the workspace with the `wasm32-unknown-unknown` target installed
- WHEN `cargo build -p clm-orchestrator --no-default-features --target wasm32-unknown-unknown` runs
- THEN it succeeds

#### Serves
- capability/follow-from-browser
- capability/watch-beside-chat
- architecture/web-optional

### Requirement: backlog-claim
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
- architecture/lease-claims

### Requirement: proposals
`clm proposals` SHALL fetch `origin` and print, sorted, one line per change on main whose `resolves` names an item still in `backlog/`, as `<id> main [<components>] <slug>...`, and per change with proposed spec work, as `<id> proposed [<components>] <slug>...`, followed by one line per live `propose/<slug>` claim that no listed change names, as `backlog/<slug> claimed [<components>]`, with components and items as the Design defines. When the checkout is on `spec/<id>`, it SHALL leave out change `<id>`, and the claims of the items its working-tree `changes/<id>/change.md` names in `from` or `resolves`. With `--component <id>`, given one or more times, it SHALL print only the lines whose components include one given. It SHALL exit 0, and 2 when git fails.

#### Scenario: nodes before deltas
- GIVEN `origin/autosync/spec/20260101-c` whose `change.md` has `from: backlog/q`, no deltas, and `backlog/q.md` with `components: [sync]`
- WHEN `clm proposals --component sync` runs
- THEN it prints `20260101-c proposed [sync] q`

#### Scenario: each kind of line
- GIVEN change `20260101-d` on main resolving existing item `r` with a delta for `gate`; `origin/spec/20260101-c` with a delta for `check` and `resolves: [q]`, where `q` has `components: [model]`; and `origin/propose/s` committed 1 hour ago, where `s` has `components: [ci]`
- WHEN `clm proposals` runs from a checkout on `main`
- THEN it prints `20260101-c proposed [check,model] q`, `20260101-d main [gate] r` and `backlog/s claimed [ci]`, in that order

#### Scenario: node filter
- GIVEN the same refs
- WHEN `clm proposals --component ci --component gate` runs
- THEN it prints only the `20260101-d` and `backlog/s` lines

#### Scenario: own work and lapsed claims left out
- GIVEN the checkout on `spec/20260101-c`, whose working-tree `change.md` has `resolves: [q]`, the same `origin/spec/20260101-c`, `origin/propose/q` committed 1 hour ago, and `origin/propose/t` committed 5 hours ago
- WHEN `clm proposals` runs
- THEN it prints no line for `20260101-c`, `q` or `t`

#### Scenario: item without nodes
- GIVEN `origin/propose/u` committed 1 hour ago, where `backlog/u.md` in `origin/main` has no `components`
- WHEN `clm proposals` runs
- THEN it prints `backlog/u claimed []`

#### Scenario: unreadable change
- GIVEN `origin/spec/20260101-e` whose `change.md` has no frontmatter, with a delta for `sync`
- WHEN `clm proposals` runs
- THEN it prints `20260101-e proposed [sync]`, prints a warning naming `origin/spec/20260101-e` on stderr, and exits 0

#### Scenario: unknown frontmatter keys
- GIVEN `origin/spec/20260101-f` whose `change.md` has `resolves: [q]` and a key `reviewers` CLM doesn't define
- WHEN `clm claim backlog/q` runs
- THEN it exits 1 naming `20260101-f`

#### Serves
- capability/propose-intent
- capability/work-concurrently
- capability/triage-unattended

### Requirement: git-through-layer
`clm-orchestrator` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `gh`, `run` or repo queries (`git-plumbing-layer`), and its own wrapper is gone (`no-dead-code`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-orchestrator` reads `crates/clm-orchestrator/Cargo.toml` and every file under `crates/clm-orchestrator/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")` or `Command::new("gh")`

#### Serves
- architecture/reuse-first
- architecture/no-dead-code

### Requirement: claim-fix
`clm claim <component> --fix backlog/<slug>` SHALL claim `impl/<component>` as `claim` does, with the same atomicity, takeover and rejection handling, whether the component is ready, waiting, blocked or converged. It SHALL accept an item of kind `bug` or `deferred`, so a branch that does a deferred item's code-only work has the claim it deletes the item from (`deferred-work-resolves-in-code`). It SHALL exit 1 without pushing when the component is claimed, or when `backlog/<slug>.md` isn't in `origin/main`'s tree, is of neither kind, or doesn't list `<component>` in its `components`, saying which. Once claimed, it SHALL switch the checkout to a local `impl/<component>` at the claim commit, tracking `origin/impl/<component>`, before printing the branch name, resetting a local `impl/<component>` whose tip is an ancestor of `origin/main`. After refusing a live claim, it SHALL exit 1 without pushing, saying why, when the checkout has uncommitted changes to tracked files, when another worktree has `impl/<component>` checked out (naming that worktree's path), or when a local `impl/<component>` holds commits `origin/main` doesn't (naming the branch). It SHALL move the local ref before the working tree, so that when the switch fails after the push it leaves the checkout on its branch with its files unchanged, prints nothing on stdout, and exits 2 saying the component is claimed on `origin` but the checkout couldn't switch, with git's reason.

#### Scenario: converged node
- GIVEN `origin/main` with converged component `a` and `backlog/x.md` of kind `bug` whose `components` are `[a]`, and a clean checkout on `spec/s`
- WHEN `clm claim a --fix backlog/x` runs
- THEN it pushes `impl/a`, prints it and exits 0, and the checkout is on `impl/a` at `origin/impl/a`'s tip with `origin/impl/a` as its upstream, where `clm claim a` would exit 1 saying `a` is converged and leave the checkout on `spec/s`

#### Scenario: deferred item
- GIVEN the same `a`, with `backlog/k.md` of kind `deferred` whose `components` are `[a]` in place of `x`
- WHEN `clm claim a --fix backlog/k` runs
- THEN it pushes `impl/a`, prints it and exits 0, and the checkout is on `impl/a`

#### Scenario: fix on a claimed node
- GIVEN the same, with `origin/impl/a` committed 1 hour ago
- WHEN `clm claim a --fix backlog/x` runs
- THEN it exits 1 saying `a` is claimed, without pushing, even when the checkout is on a local `impl/a` from an earlier fix claim

#### Scenario: item of another kind or node
- GIVEN the same `a`, with `backlog/y.md` of kind `idea` naming `[a]`, `backlog/q.md` of kind `question` naming `[a]` and `backlog/z.md` of kind `bug` naming `[b]`
- WHEN `clm claim a --fix backlog/y`, `clm claim a --fix backlog/q` and `clm claim a --fix backlog/z` run
- THEN each exits 1 without pushing, the first two saying the item is neither a bug nor deferred work and naming its kind, the third that `z` doesn't name `a`

#### Scenario: stale local branch reset
- GIVEN the same `a` and `x`, with a local `impl/a` checked out nowhere whose tip is an ancestor of `origin/main`
- WHEN `clm claim a --fix backlog/x` runs
- THEN it exits 0 and the checkout is on `impl/a` at the new claim commit, not the old tip

#### Scenario: branch held by another worktree
- GIVEN the same `a` and `x`, with a second worktree at path `w` that has a local `impl/a` checked out
- WHEN `clm claim a --fix backlog/x` runs from the first
- THEN it exits 1 naming `impl/a` and `w`, `origin/impl/a` doesn't exist, and the checkout is still on `spec/s`

#### Scenario: unmerged local commits
- GIVEN the same `a` and `x`, with a local `impl/a` checked out nowhere that holds a commit `origin/main` doesn't
- WHEN `clm claim a --fix backlog/x` runs
- THEN it exits 1 naming `impl/a`, without pushing, and the local `impl/a` still holds that commit

#### Scenario: uncommitted changes
- GIVEN the same `a` and `x`, with an uncommitted edit to a tracked file in the checkout
- WHEN `clm claim a --fix backlog/x` runs
- THEN it exits 1 saying the checkout has uncommitted changes, without pushing, and the edit is still there

#### Scenario: switch fails after the push
- GIVEN the same `a` and `x`, with a `reference-transaction` hook, installed through the repository's own `core.hooksPath`, that rejects any update to `refs/heads/impl/a` in its `prepared` state
- WHEN `clm claim a --fix backlog/x` runs
- THEN it exits 2 printing nothing on stdout, `origin/impl/a` is the claim commit, the checkout is still on `spec/s` with no uncommitted changes, and stderr says `impl/a` is claimed on `origin` but the checkout couldn't switch, with git's reason

#### Serves
- capability/report-defects
- capability/work-concurrently
- architecture/fold-small-defer-large

### Requirement: taken-items
`taken_items(root, expiry)` SHALL fetch `origin` and return the changes that hold backlog items, each with its state (`main` or `proposed`), its ID, the items it names in `from` or `resolves` and its components as `clm proposals` computes them, and the live `propose/<slug>` claims that no such change names, each with its item and components; `clm proposals` SHALL print exactly these.

#### Scenario: same as proposals
- GIVEN change `20260101-d` on main resolving existing item `r` with a delta for `gate`; `origin/spec/20260101-c` with a delta for `check` and `resolves: [q]`, where `q` has `components: [model]`; `origin/propose/s` committed 1 hour ago, where `s` has `components: [ci]`; and `origin/propose/q` committed 1 hour ago
- WHEN `taken_items` runs with a 4-hour expiry
- THEN it returns `20260101-c` proposed with items `q` and components `check` and `model`, `20260101-d` on main with item `r` and component `gate`, and the claim `s` with component `ci`, and no claim `q`

#### Serves
- capability/keep-a-backlog
- capability/work-concurrently
- architecture/lease-claims

### Requirement: nested-branches-not-claims
`claim_refs`, `queue_from_refs`, `clm next` and `clm status`'s `claims` section SHALL read a ref `origin/impl/<x>` as a claim only when `<x>` contains no `/`: a nested branch such as `origin/impl/archive/<id>` SHALL claim no component, hold no component `claimed` and have no `claims` row.

#### Scenario: archive branch beside a claim
- GIVEN remote-tracking refs `origin/impl/b` and `origin/impl/archive/20261026-x`, both committed 1 hour ago
- WHEN `clm status` runs
- THEN its `claims` rows are `b`, `live` only, and no row names `archive`

#### Scenario: archive branch in the queue
- GIVEN a graph with ready components `archive` and `b`, and live claim refs for the branches `impl/archive/20261026-x` (name `archive/20261026-x`) and `impl/b` (name `b`)
- WHEN `queue_from_refs` runs
- THEN `b` is `claimed` and `archive` is `ready`

### Requirement: places
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
- capability/unattended-runner
- capability/build-from-spec
- architecture/component-reconciliation

### Requirement: shared-plumbing
With its `git` feature on, `clm-orchestrator` SHALL export `REMOTE` as `clm-git`'s `REMOTE` re-exported, define no constant of its own for the remote's name, and read the time for claims and places from `clm-git`'s `now()`: no file under `crates/clm-orchestrator/src` SHALL hold a string literal that begins `"origin`, nor a function of its own that returns the seconds since the Unix epoch (`shared-plumbing-in-git`). With default features off it SHALL still build for `wasm32-unknown-unknown` (`orchestrator-builds-for-wasm`), without `REMOTE`.

#### Scenario: one remote's name
- GIVEN the crate with its default features
- WHEN a test in a target that requires the `git` feature (such as `tests/orchestrator.rs`) compares `clm_orchestrator::REMOTE` with `clm_git::REMOTE`
- THEN they are equal

#### Scenario: no copy of the remote's name or the clock
- GIVEN the workspace
- WHEN a test in `clm-orchestrator` reads every file under `crates/clm-orchestrator/src`
- THEN no file holds `"origin` or a `fn now(`

#### Serves
- architecture/reuse-first
- architecture/no-dead-code

#### Needs
- vcs/remote-name
- vcs/unix-now

### Requirement: taken-from-refs
`taken_from_refs` SHALL return, for `main`'s graph, the spec refs with their `change.md` texts and delta names, the `propose/*` refs with their tip times, a time and an expiry, exactly what `taken_items` returns for the same `main` tree, branches and time from a checkout that isn't on a spec branch, warnings included, and SHALL read no files and run no git. `taken_items` SHALL compute its result with it.

#### Scenario: same as taken_items
- GIVEN change `20260101-d` on main resolving existing item `r` with a delta for `gate`; ref `spec/20260101-c` with a delta for `check` and `resolves: [q]`, where `q` has `components: [model]`; ref `autosync/spec/20260101-e` whose `change.md` has no readable frontmatter; ref `propose/s` committed 1 hour ago, where `s` has `components: [ci]`; and ref `propose/q` committed 1 hour ago
- WHEN `taken_from_refs` runs with a 4-hour expiry, and `taken_items` runs with the same expiry at the same time on a repo with that tree and those branches, from a checkout on `main`
- THEN both return `20260101-c` proposed with items `q` and components `check` and `model`, `20260101-d` on main with item `r` and component `gate`, `20260101-e` proposed with no items, the claim `s` with component `ci` and no claim `q`, and one warning naming `20260101-e`

#### Serves
- capability/keep-a-backlog
- capability/work-concurrently
- architecture/git-is-truth

### Requirement: free-stale-places
`free_stale_places(root, expiry)` SHALL, under the exclusive lock on `<git common dir>/clm/builds.lock` that `take_place` takes, read the clone's places, delete each one that doesn't count as `places` judges it with that expiry, and return one `Swept` per place it read, sorted by name, holding the `Place` as `places` would return it and an outcome: kept, for one that counts; freed, for one it deleted; or failed with why, for one it couldn't delete, after which it SHALL go on to the next. It SHALL return `Error` only when it can't find the places directory or take the lock. It SHALL keep each place that counts, a file it can't read or parse included. It SHALL NOT fetch. A place written while it waits for the lock SHALL be judged as written.

#### Scenario: sweep
- GIVEN places `a`, for `origin/impl/a` committed 5 hours ago and taken 5 hours ago; `b`, whose `impl/b` tips are ancestors of `origin/main`; `c`, for `origin/impl/c` committed 1 hour ago; and a file `z` holding `not a place`
- WHEN `free_stale_places` runs with an expiry of 4 hours
- THEN the files `a` and `b` are gone and returned as deleted, and `c` and `z` are still there and returned as counting, `z` with why it can't be parsed

#### Scenario: retaken while waiting
- GIVEN a place `a` that doesn't count, and a subprocess holding `builds.lock`
- WHEN `free_stale_places` starts while the lock is held, and the subprocess then rewrites `a` taken now and releases the lock
- THEN `free_stale_places` returns `a` as counting, and the file `a` is still there

#### Serves
- capability/wrap-up-session
- capability/work-concurrently

### Requirement: queue-pending-entries
A `ready` component's pending requirements, as `queue_from_refs` returns them, SHALL include its blocked ones, and each SHALL carry its state (`unbuilt`, `behind` with the versions its markers cite, or `obsolete` with its markers' locations), its version and its priority (`priority-orders-nodes`), `blocked_by`, the slugs of the items that block it, and `waits_on`, its other blockers as `check.coverage/pending-report` gives them, as `<component>/<slug>`. The serialised queue SHALL hold `blocked_by` and `waits_on` under those names, with no field `needs_pending`.

#### Scenario: entry states
- GIVEN component `a` whose `a/z` is uncited, whose `a/y` is cited at `11111111` and by a bare marker while its current version differs, and a test citing `a/gone`, which an `applied` change removed
- WHEN `queue_from_refs` runs
- THEN `a` is ready, with `a/gone` obsolete with that test's location, `a/y` behind citing `11111111` and `none` at its current version, and `a/z` unbuilt

#### Scenario: waits serialised
- GIVEN component `a` with two uncited requirements: `a/x`, which needs `b/y`, uncited, and `a/w`, which nothing blocks
- WHEN `queue_from_refs` runs and its result is serialised to JSON
- THEN `a` is `ready`, and its entry for `a/x` has `waits_on` `["b/y"]` and `blocked_by` `[]`, and no `needs_pending`

#### Needs
- check.coverage/pending-report
- check.coverage/state-names

### Requirement: queue-complete-changes
`queue_from_refs` SHALL return the list of complete changes: the `applied` and `implementing` ones that are complete as `check.coverage/status-report` defines it.

#### Scenario: complete change
- GIVEN an `applied` change `x` whose delta for `a` adds `a/p` and removes `a/q`, `a/p` cited at its current version, and no marker citing `a/q`
- WHEN `queue_from_refs` runs
- THEN `x` is among the complete changes

#### Scenario: incomplete change
- GIVEN the same change, with a test still citing `a/q`
- WHEN `queue_from_refs` runs
- THEN `x` is not complete, and `a` is pending with `a/q` obsolete

#### Scenario: unapplied change
- GIVEN a `proposed` change `y` whose delta modifies `a/p`, which is cited at its current version
- WHEN `queue_from_refs` runs
- THEN `y` is not among the complete changes

#### Needs
- check.coverage/status-report

### Requirement: queue-matches-next
The components `queue_from_refs` reports `ready` SHALL be exactly those `clm next` lists before its cap on build places (`places`), and the components it reports `claimed` exactly the live claims, for the same `main` tree, branches and time.

#### Scenario: same as next
- GIVEN components `a` depending on `b`, `c` depending on `d`, `e` and `f`, where `b` is converged, `d` has an uncited requirement, `e` has an uncited requirement and is claimed by a branch pushed an hour ago, `f`'s only pending requirement is in the `blocks` of an existing `human-task` backlog item, and `a` and `c` each have an uncited requirement
- WHEN `queue_from_refs` runs with a 4-hour expiry, and `clm next` runs with free build places on a repo with the same tree and branches at the same time
- THEN `queue_from_refs` reports `a`, `c` and `d` ready and `e` alone claimed, and `clm next` lists `a`, `c` and `d`

#### Needs
- model/parse-backlog

### Requirement: next-wait
`clm next --wait` SHALL look for work as `clm next` does and, while a look would print nothing, wait `--interval <minutes>` (a positive decimal number, default 5) and look again, fetching `origin` at each look, until a look would print at least one component; then it SHALL print what that look prints, as `clm next` would, and exit 0. When the first look fails, it SHALL exit as `clm next` would, 1 when the work isn't possible and 2 when git fails, with its reason. When a later look fails, it SHALL print `clm next failed: <reason>` to stderr and wait for the next look, never exiting for it. An `--interval` that isn't a positive number SHALL exit 1 before the first look, saying so. `--interval` without `--wait` SHALL be refused as a usage error. `clm next` without `--wait` SHALL behave as before.

#### Scenario: work arrives
- GIVEN a clone where no component is ready, and `clm next --wait --interval 0.01` running
- WHEN a commit adding an uncited requirement to component `a` is pushed to `origin/main`
- THEN it prints `a` and exits 0

#### Scenario: work already there
- GIVEN a clone where component `a` is ready and unclaimed
- WHEN `clm next --wait` runs
- THEN it prints `a` at once and exits 0, as `clm next` would

#### Scenario: a place frees
- GIVEN ready component `a`, `max_builds` 1, one place that counts for a live claim `origin/impl/b`, and `clm next --wait --interval 0.01` running
- WHEN `impl/b` lands on `origin/main`
- THEN it prints `a` and exits 0

#### Scenario: a later look fails
- GIVEN `clm next --wait --interval 0.01` running after a first look that printed nothing, and `origin` then unreachable
- WHEN it looks again, and `origin` is reachable again with component `a` ready
- THEN it prints `clm next failed: ` with git's reason to stderr, keeps waiting, then prints `a` and exits 0

#### Scenario: first look fails
- GIVEN a clone whose `origin` can't be fetched
- WHEN `clm next --wait` runs
- THEN it exits 2 with git's reason, as `clm next` does

#### Scenario: bad interval
- GIVEN any clone
- WHEN `clm next --wait --interval 0` runs
- THEN it exits 1 saying the interval must be a positive number, without fetching

#### Serves
- capability/build-from-spec

### Requirement: archived-spec-work
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
- architecture/lease-claims

### Requirement: says-component
The messages `clm next`, `clm claim`, `clm proposals` and `clm status`'s `claims` rows print SHALL say component and components, never the old words, as `component-words` defines them: among them ``no component `<id>` ``, ``` `<id>` isn't a component ID ```, and the claim refusals that name a component's state. A test reads the string literals in `crates/clm-orchestrator/src/` and fails naming the file, the line and the word for each old word `component-words` doesn't keep.

#### Scenario: claiming an unknown component
- GIVEN no component `zz`
- WHEN `clm claim zz` runs
- THEN it exits 1 printing a message holding ``no component `zz` ``

#### Serves
- architecture/plain-terms

### Requirement: priority-orders-components
A pending requirement's priority SHALL be the highest priority among the changes outside the archive whose deltas add, modify or remove it, or `P5` when there is none. A component's *effective priority* SHALL be the highest of the priorities of its unblocked pending requirements and of the priorities seeded on it: a pending requirement blocked by a wait seeds its priority on the component of each pending requirement it waits on, and a requirement waited on that is itself blocked by a wait passes it on to the requirements it waits on, through components of any state, so a `claimed` or `blocked` component in between passes a dependant's priority on. A wait SHALL be each entry of the requirement's `waits_on` (`check.coverage/pending-report`): a requirement its `Needs` names, for an `obsolete` requirement one a move took it to, and for a capability a rule serving it that holds it back (an `unbuilt` one, or a `behind` one itself blocked), so the rules serving a capability, and the work a changed one waits on, inherit its priority. `depends_on` SHALL carry no priority (`needs-alone-order`). `queue_from_refs` SHALL report each component's effective priority, and `clm next` SHALL sort by it.

#### Scenario: urgent work pulls the node it needs forward
- GIVEN component `a` whose only pending requirement `a/x` comes from a change with `priority: P1` and needs uncited `b/y`, `b/y` from a change with no priority, and a pending component `c` on another part of the graph from a change with `priority: P3`
- WHEN `clm next` runs
- THEN it prints `b` then `c`, and not `a`

#### Scenario: passes through a claimed node
- GIVEN component `c` whose `P1` pending requirement `c/z` needs `b/y`, component `b` claimed, whose pending requirement `b/y` needs `a/x`, `a/x` uncited from a change with no priority, and a pending component `d` elsewhere at `P3`
- WHEN `clm next` runs
- THEN it prints `a` then `d`

#### Scenario: dependency lends no priority
- GIVEN component `a` depending on `b`, both pending, `a`'s pending requirement from a change with `priority: P1` and `b`'s from a change with no priority, and a pending component `c` elsewhere at `P3`
- WHEN `clm next` runs
- THEN it prints `a`, `c` then `b`

#### Scenario: blocked requirement doesn't lend its priority
- GIVEN component `b` whose `P1` pending requirement is in the `blocks` of an existing backlog item, and whose other pending requirement is `P5`, and component `c` pending at `P3`
- WHEN `clm next` runs
- THEN it prints `c` then `b`

#### Scenario: capability pulls its serving rules forward
- GIVEN component `capability` whose `capability/c` is uncited and comes from a change with `priority: P1`, component `w` with `serves: [capability/c]` holding `w/x` uncited from a change with no priority, and a pending component `d` elsewhere at `P3`
- WHEN `clm next` runs
- THEN it prints `w` then `d`, and not `capability`

#### Scenario: capability pulls a changed serving rule's needs forward
- GIVEN component `capability` whose `capability/c` is cited at an older version and comes from a change with `priority: P1`, component `w` with `serves: [capability/c]` holding `w/x` cited at an older version with `#### Needs` naming `b/y`, `b/y` uncited, both from changes with no priority, and a pending component `d` elsewhere at `P3`
- WHEN `clm next` runs
- THEN it prints `b` then `d`, and neither `capability` nor `w`

#### Needs
- check.coverage/pending-report
- view/blockers-view
- view/status-report

### Requirement: claim-local
`claim_local(root, component, expiry)` SHALL claim the component as `claim` does for a component, with the same refusals, atomicity, takeover and rejection handling, except that after making the claim commit and before its first push it SHALL create the local branch `impl/<component>` at that commit, with `origin/impl/<component>` as its upstream, and SHALL return a git error without pushing when it can't, such as when a local `impl/<component>` exists, leaving that branch unchanged. On success the local branch SHALL be at the claim commit, tracking `origin/impl/<component>`. When the claim fails and the branch's tip on `origin`, read again, isn't the claim commit, it SHALL delete the local branch before returning; when that tip can't be read, it SHALL leave the branch. `claim` SHALL create no local branch. The crate SHALL expose no other call that claims a component: `claim_node`, which `claim` calls for a component, SHALL NOT be public.

#### Scenario: branch before the push
- GIVEN a ready component `b`, no local `impl/b`, and a `pre-push` hook in the clone that records the tip of the local `impl/b`
- WHEN `claim_local` runs for `b`
- THEN it succeeds, the hook recorded the claim commit, and the local `impl/b` is `origin/impl/b`'s tip and has `origin/impl/b` as its upstream

#### Scenario: lost race
- GIVEN a ready component `b`, and another clone's claim pushed to `origin`'s `impl/b` after `claim_local`'s fetch and before its push
- WHEN `claim_local` runs for `b`
- THEN it is refused, saying `b` was claimed by someone else first, `origin`'s `impl/b` is the other clone's claim, and the clone has no local `impl/b`

#### Scenario: rejected for another reason
- GIVEN a ready component `b` and an `origin` whose `pre-receive` hook rejects every push
- WHEN `claim_local` runs for `b`
- THEN it returns a git error saying the failure may be transient, `origin` has no `impl/b`, and the clone has no local `impl/b`

#### Scenario: tip unreadable
- GIVEN a ready component `b` and a `pre-push` hook in the clone that sets `remote.origin.url` to a path that doesn't exist and exits 1
- WHEN `claim_local` runs for `b`
- THEN it returns a git error, and the local `impl/b` is still at the claim commit

#### Scenario: existing local branch
- GIVEN a ready component `b` and a local `impl/b` at a commit of `origin/main`
- WHEN `claim_local` runs for `b`
- THEN it returns an error, `origin` has no `impl/b`, and the local `impl/b` is unchanged

#### Scenario: plain claim
- GIVEN a ready component `b` and no local `impl/b`
- WHEN `clm claim b` runs
- THEN it exits 0 and the clone has no local `impl/b`

### Requirement: place-slots
Each place SHALL hold a slot. When `take_place` takes a new place it SHALL write into its file, after the `taken` line and before the `branch` lines, a line `slot <n>`, `<n>` being the smallest integer from 1 up that no other place that counts holds, chosen under the `builds.lock` it holds; when it reuses a place it SHALL keep that place's slot, or, when the file holds none, write one by the same rule. Drafts' places hold slots as builds' do. `take_place`'s result type SHALL be unchanged. `places` SHALL return each place's slot, or none for a file that holds none or can't be parsed, and a file with no `slot` line SHALL parse as before. `place_slot(root, branch)` SHALL return the slot of the place whose `branch` lines name `branch`, the first such place by name when several do, and 0 when none does or that place holds no slot; it SHALL NOT fetch, take the lock or judge whether the place counts, and SHALL return `Error` only when it can't find the git common directory.

#### Scenario: the lowest free slot
- GIVEN `max_builds` 4, places `a` with slot 1 and `c` with slot 3 that count, and a place `b` with slot 2 that doesn't count
- WHEN `take_place` runs for `d`
- THEN it returns slot 2, the file `d` holds `slot 2`, and the clone has no place `b`

#### Scenario: a reused place keeps its slot
- GIVEN a place `a` with slot 3 that counts
- WHEN `take_place` runs for `a`
- THEN it returns slot 3 and the file `a` still holds `slot 3`

#### Scenario: a place written before slots
- GIVEN a place `a` that counts whose file holds no `slot` line, and a place `b` with slot 1 that counts
- WHEN `take_place` runs for `a`
- THEN it returns slot 2 and the file `a` holds `slot 2`

#### Scenario: the slot of a branch
- GIVEN a place `x` of kind `build` naming `impl/x` with slot 2
- WHEN `place_slot` runs for `impl/x`, and for `impl/y`
- THEN it returns 2, and then 0

#### Serves
- architecture/tests-run-concurrently
- capability/work-concurrently
