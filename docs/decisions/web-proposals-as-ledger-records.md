---
title: "A proposal entered in the web app is a `proposed` ledger record, which `clm sync` files as a backlog item"
date: 2026-10-28
status: accepted
---

The web app's first version read and approved; a reviewer asked to enter a new proposal from it (`backlog/web-proposals`): an intent typed on a phone that a `propose` session later designs. A proposal is a backlog item in the making, and a backlog item is a file on `main`, which only `clm land` moves (`land-locally`). The app can't land, so what it writes is what it already writes for an approval: a record in the signed-in person's web ledger, `clm/ledger/<login>/web`, appended with their token (`per-writer-ledgers`). A `proposed` record carries the item's `slug`, `title`, `body` and `nodes`; a `withdrawn` record names it and ends it; the Worker lets only the proposer write one, and the fold counts any.

The record becomes a file the way split and reconsider items do (`file-split-item`, `file-reconsider-item`): `clm sync` on the next `impl/*` branch writes `backlog/<slug>.md` from it, `kind: idea`, and the branch lands it. A new item never makes a code branch mixed (`gate/backlog-additions`), so no review is spent on it, and from then on it is an ordinary item: `clm triage` groups it, `clm draft` claims it, a spec change `resolves` it. The Worker files nothing itself, and `clm land` adds nothing to a tree it checks.

A proposal is *open* while no `withdrawn` record names it and the tree being read holds neither `backlog/<slug>.md` nor a change, archived or not, whose `resolves` names the slug. That is read from git by every reader (`clm sync` from `HEAD`, the Worker from the snapshot), never kept in the ledger, so the file can land, be resolved and be deleted without a second record: the archived change holds the slug for good. An item deleted by hand, with no change resolving it, comes back on the next build's sync, as a deleted split item does; the ways to close a proposal are a change that resolves it or its proposer withdrawing it. Web proposals are `idea` items only: an idea is resolved by a spec change, whose `resolves` the archive keeps, while a `bug` can be deleted by a code fix's commit trailer (`code-fix-resolves-bug`), which no tree records, so the proposal would reopen.

- The slug is the title in kebab case, as `clm defect` slugs one. The Worker refuses a title whose slug the snapshot's backlog, a change's `resolves` (the snapshot's `resolved`) or an open proposal already has, so the person changes the title; a slug a change resolved is spent for good, since `clm sync` would never file it; `clm sync` never renames.
- The file's content comes from the record alone (title, body, nodes, proposer, time, record `id`), so two builds that file one proposal add the same bytes and their lands merge clean.
- A node the graph no longer holds is left out of the item, since `model/backlog-nodes` refuses it.
- An older `clm` skips the two kinds with a warning (`ledger`'s fold) and files nothing.

Rejected:
- **The Worker pushes a branch holding the item.** It would wait for someone to run `clm land` on it, and a `spec/*` branch with no change would sit in the app's PR list; the ledger needs no landing of its own.
- **A `filed` record appended by `clm sync` when it writes the item.** A branch that files an item and never lands would close the proposal with nothing on `main`; the tree says what is filed.
- **`clm triage` reads open proposals straight from the fold and `clm draft` writes the file on its spec branch.** `clm claim backlog/<slug>` refuses an item that isn't in `origin/main`'s `backlog/`, so every claim path would need a second source; filing first keeps one.
- **Handing the intent straight to a `propose` session.** Nothing on the user's side runs sessions; the triage loop drafts the backlog, and this puts the proposal in it.
