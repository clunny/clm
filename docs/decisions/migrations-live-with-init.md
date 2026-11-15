---
title: "A migration written today lives in init, beside what it changes, and upgrade's own list is closed"
date: 2026-11-14
status: accepted
---

A change to a file `clm init` writes and the migration that carries it can't land apart: the upgrade corpus test fails a land of either half (`template-change-ships-its-migration`). The file was `init`'s and the migration `upgrade`'s, so `init`'s build wrote in `upgrade`'s files, and since `20261114-proof-switched-on` the land refuses that (`own-files-at-the-gate`). Things that must land together belong in one component.

**Every migration written from now on is `init`'s.** `init` is the component that knows what a project holds: `clm init` writes it, and a migration brings an older project to it. `upgrade` keeps the engine, which lists, applies, notes, declines and locks, and its own thirteen migrations, which no change adds to (`upgrade/own-list-closed`). `init` offers the whole list, `upgrade`'s followed by its own (`init/all-migrations`), and every caller of the engine passes it: the root, the session-start hook and `clm init`'s own stamp.

**Why all of them, and not only those of files `clm init` writes.** A project's `upgraded` names a migration, and an apply migration is pending when it comes after that one in the list. So the list's order may never change, and nothing may be added before a migration a project could already name. With two lists that both grow there is no order that holds: a migration added to `upgrade`'s would land before `init`'s newest, and a project stamped with that would never be offered it. One list that grows, after one that is closed, has an order that can't change. A migration for a settings key or a format is then a requirement of `init`'s that needs the requirement making the change, as it was a requirement of `upgrade`'s before.

**Why the shipped ones stay where they are.** A shipped migration never changes (`upgrades-carry-migrations`), and the lock holds each one's directory to its hash. Moving thirteen directories would move files no build will ever edit again, and rename thirteen requirements, for nothing: who owns a frozen file never matters.

**The cycle.** `init` depends on `upgrade`, to stamp `upgraded`. `template-change-ships-its-migration` rejected putting a migration in `init` for that reason: `upgrade` held the list, and couldn't read `init`'s. The engine now takes the list from its caller and reads none of its own, so the dependency stays one way. The forms that read the crate's own list go once the root, the hook and `init` have moved (`upgrade/forms-without-a-list-gone`, `expand-contract`).

**Four shipped notes read the stamp in `upgrade`'s own list.** `stamp-taken-notes` gave a note that only asks to be read a `done` that is true once `upgraded` names it or a later migration, and the four such notes look for the stamp in `clm-upgrade`'s `migrations()`. Three are locked and can't change. A stamp that names a migration of `init`'s is in no list they read, so each would turn pending again in every project on the day `init` held its first migration. The engine therefore leaves those four out itself when the stamp sits at or after them in the list it is given (`upgrade/pending-migrations`). A note of `init`'s that the stamp takes reads `init`'s own whole list, and needs nothing of the kind.

**What it leaves.**
- `clm/upgrade-corpus` names the migrations it declines before it compares, those that offer a text for a file the comparison leaves out. A new migration of that kind is built in steps: the change adds its ID to that requirement, the root's build adds it to the test, which declines only what is pending, and `init`'s requirement for the migration needs the root's. A migration that rewrites a file, or that the stamp takes, needs nothing there.
- The root's tests of `upgrade`'s commands run the binary, which holds every migration, so one that lists what is pending by ID can fail when `init` adds a migration, and `init`'s build can't edit it. It goes when each crate proves its requirements through its own entry point (`backlog/clm-s-cli-tests-prove-other-components-through-the-binary`); until then such a build leaves its requirement unbuilt with a question item, as the land's message says.

Rejected:
- **New migrations of `init`'s files in `init`, the others still in `upgrade`.** Two lists that grow, which the stamp can't order.
- **Moving every migration to `init`.** The frozen files, as above.
- **`upgrade` owning the files `clm init` writes.** Their requirements, most of `init`'s, would move with them.
- **The migration files in the root's `wiring`**, and **an exemption rule**: `own-files-at-the-gate` records why not.
