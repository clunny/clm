---
status: done
resolves: [a-template-and-its-migration-live-in-different-components, a-mutation-pass-that-fails-before-any-has-been-recorded-loses]
---

## Intent

Two things phase 3 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md` left blocking. A migration written from now on lives in `init`, beside the file it carries a change of, so the build that changes what a project holds writes the migration in its own component and the land's ownership rule has nothing to refuse. And a mutation pass that fails before any pass has been recorded no longer loses the requirements it was to judge.

It removes no requirement, scenario or test. It removes seven library calls of `upgrade`, in steps, once nothing calls them.

## Problem

- A change to a file `clm init` writes and the migration that carries it can't land apart, since the upgrade corpus test fails a land of either half. The file is `init`'s and the migration `upgrade`'s. Since `20261114-proof-switched-on` the land refuses `init`'s build when it writes in `upgrade`'s files, so every such requirement waits behind a question item.
- With no `mutated` record, a mutation pass starts from its own verification's `since`. On 2026-11-14 the first real pass covered 17 lands, failed and recorded nothing; the next verification's pass started from that commit and covered 2. The tests of the 17 lands were never due again.

## Approaches

**Where a migration lives** (`migrations-live-with-init`).
- *Taken: every migration written from now on is `init`'s; `upgrade`'s own list is closed; `init` offers the whole list and every caller of `upgrade`'s engine passes it.* A project's `upgraded` names a migration and rests on the list's order, so nothing may ever be added before a migration a project could name. One list that grows, after one that is closed, has an order that can't change.
- *Not taken: only the migrations of `init`'s files in `init`,* which the backlog item sketched. Two lists that both grow have no order the stamp can rest on.
- *Not taken: moving all thirteen shipped migrations.* They are frozen, so who owns them never matters, and it would rename thirteen requirements.
- *Not taken: `upgrade` owning the files `clm init` writes.* Most of `init`'s requirements would move with them.

**How the engine reaches `init`'s list.** `init` depends on `upgrade`. *Taken: the engine takes the list from its caller;* it already does so inside the crate, in forms the tests use. *Not taken: `upgrade` depending on `init`,* a cycle.

**Where the first pass starts** (`mutation-after-land`).
- *Taken: the last commit before `clm.toml` set `[verify] mutate`,* read from git, so it doesn't move while passes fail.
- *Not taken: the oldest `verified` record,* which would judge every test written before the project switched the pass on, a decision `backlog/tests-written-before-the-mutation-pass-were-never-judged` keeps for itself.
- *Not taken: a record of a failed pass.* A new ledger record for what git already says.

## Outcome

**`upgrade`: the engine takes its list.**
- `migration-list`, `pending-migrations`, `require-current`, `upgrade-apply` and `superseded-notes` state the forms that take the list and the supersedes pairs: `latest_in`, `pending_in`, `require_current_in`, `apply_in`, with `list_in`, `note_in` and `decline_in` in the Contract. They exist inside the crate today and become public.
- `own-list-closed`: the crate's own list is its thirteen migrations and no other.
- Four shipped notes decide they are done by finding the stamp in the crate's own list, and three of them are locked. A stamp naming a migration of `init`'s is in no list they read, so `pending_in` leaves those four out itself when the stamp sits at or after them in the list it is given. `migration-tests-overlap`, `migration-land-dir`, `migration-build-setup` and `migration-markers-in-tests` say so, and no longer name `latest()`.
- `shipped-migrations-frozen`: the lock's check and its append are public calls over any crate's directory.
- `forms-without-a-list-gone`: the seven forms that read the crate's own list go, once the root, the hook and `init` have moved. Until then they stay, marked `clm:remove-by` this change.

**`init`: the whole list, and the migrations to come.**
- `all-migrations`: `migrations()` and `supersedes()`, `upgrade`'s followed by `init`'s own. `init` holds none yet.
- `own-migrations-frozen`: `crates/clm-init/migrations.lock`, held to the rule `upgrade`'s lock is.
- `init-stamps-upgraded` stamps the last ID of the whole list.

**The callers.** `clm/upgrade-corpus` no longer tells the build that writes a migration to edit the root's test: the change adds the ID to that requirement, the root's build adds it to the test, and the migration's requirement needs the root's. `clm/upgrade-command` and `clm/upgrade-blocks-commands` pass `init`'s list, and `clm upgrade --lock` appends to each crate's lock. `plugin.hooks/session-upgrade-reminder` reads what is pending over `init`'s list, and `plugin.hooks` now depends on `init`. `ci/release-records-upgrade-corpus` adds `init`'s lock to the release commit when there is one.

**`verify/mutation-pass`.** With no `mutated` record, the pass starts from the first parent of the oldest commit of the unbroken run, ending at the verified commit, whose `clm.toml` sets `[verify] mutate`; with no such parent every cited requirement is due.

**`ARCHITECTURE.md`.** `upgrades-carry-migrations` names `init/own-migrations-frozen` under `#### Enforced by`, beside `upgrade`'s. Its statement is unchanged.

**Decisions.** `migrations-live-with-init` is new. `template-change-ships-its-migration`, `own-files-at-the-gate`, `coupled-changes-sequence`, `upgrade-migrations` and `mutation-after-land` each gain what this change settles.

**Backlog.** Both items are resolved. `the-build-prompt-and-the-propose-skill-send-a-migration-to-upgrade` is added for the two texts that must follow once `init/all-migrations` is built.

## Build order

By `#### Needs`: `upgrade`'s ten modified requirements and `own-list-closed` first; then `init`, the root, `plugin.hooks` and `ci` in any order; then `upgrade/forms-without-a-list-gone`. `verify/mutation-pass` waits on nothing.

## What it removes

No requirement, scenario or test. Seven public library calls of `clm-upgrade` (`pending`, `require_current`, `list`, `apply`, `note`, `decline`, `latest`), replaced by forms that take the list, in three steps (`expand-contract`). No command, output or file changes.

## What a land pays

Nothing: no check is added to `clm land`.

## What an adopter's project sees

Nothing. `clm upgrade`, the session-start reminder and the check before each command print and do what they did, over the same thirteen migrations. No file `clm init` writes, no settings key and no format changes, so no migration. A project that switches the mutation pass on and whose first pass fails keeps its requirements due.

## What a build in flight meets

- No claim is live. A requirement that changes a file `clm init` writes still waits behind a question item until `init/all-migrations` is built; from then its migration is a requirement of `init`'s.
- This repository already has a `mutated` record, so the new starting point changes nothing here.

## Open questions

From the architecture audit:
- **`upgrades-carry-migrations` becomes current again on an empty case.** `init/own-migrations-frozen` has no lock to check until a release ships a migration of `init`'s, so its only provable scenario today is the missing lock.
- **The propose skill and the build prompt are corrected by a deferred item, not a requirement.** The audit asked for requirements of `plugin` and `orchestrator.run` so the order is enforced. Skill and prompt text is source, which no requirement states (`prompt-text-is-source`), so the item stays, and the session that sees `init/all-migrations` built does its work.
- **`init/init-seeds-permissions` still says its build wrote two note migrations as `upgrade`'s requirements state them.** That is true of those two, which are among the thirteen.

From the design:

- **`ARCHITECTURE.md`'s `upgrades-carry-migrations` reads as not enforced until `init/own-migrations-frozen` is built**, since a checked entry is current when every rule it names is.
- **A root test that lists pending migrations by ID can fail when `init` adds a migration**, and `init`'s build can't edit it. It goes with phase 4 (`backlog/clm-s-cli-tests-prove-other-components-through-the-binary`); until then such a build files a question.
- **The build prompt still tells a build to leave a template change unbuilt, and the propose skill still says a requirement of `upgrade`**, until the deferred item's work lands after `init`'s build.

## Constraints

- The order of the thirteen shipped migrations, and their files, don't change.
- The freeze holds: this is the plan's own work.
