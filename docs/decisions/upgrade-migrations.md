---
title: "Projects upgrade through ordered migrations in the binary, stamped in clm.toml; a pending required one blocks"
date: 2026-11-05
status: accepted
---

A project set up by one release of `clm` must keep working under any later one, including when it skips the releases in between. `init-cli-and-skill` gave every file `clm init` writes to the project, and `init-rerun` never rewrites one, so nothing carried a later release's changes into an existing project. `plain-terms-renames` assumed that no project but CLM's own used CLM, so none needed a migration. Since `v0.14.0` CLM is published, and that assumption no longer holds.

**Migrations.** `clm-upgrade` holds an ordered list of migrations. Each is named after the change that needed it and has a `done` check over the project's files. There are two kinds:
- An **apply** migration is code that CLM runs. It is used only where CLM can rewrite exactly, such as a key rename.
- A **note** migration is advice for the files the project owns in words. It may carry the old and new template texts, so a file nobody edited is rewritten without a person. A note may also be advice that asks for no edit, such as telling a project its tests must now overlap; its `done` reads the stamp, so the upgrade that shows it marks it taken (`stamp-taken-notes`).

Every **required** migration, one without which a later `clm` would misread the project, is an apply migration. What CLM must read correctly, it can rewrite exactly. So a note is always **advised**: it records drift from what `clm init` writes today, and never blocks.

**The stamp.** `clm.toml`'s `upgraded` names the last migration the project took.
- *Apply migrations.* One is pending when it comes after the stamp and its `done` is false. A wrong stamp therefore never blocks a project whose files are right, and the block lifts as soon as they are.
- *Note migrations.* One is pending while its `done` is false and `declined_migrations` doesn't name it.
- *A newer project.* An `upgraded` the binary doesn't know means a newer `clm` upgraded the project, so an older one refuses it.

The alternative was no stamp, deriving everything from the files on every command. It was rejected for three reasons:
- some checks would read every test on every command;
- an older binary couldn't tell that a project was too new for it;
- the checks run before each command would grow forever.

**Blocking.** A pending required migration blocks every command except a short allow-list: `status`, `guide`, `upgrade`, `defect`, `config`, `init`, `backlog`, `hook`, `view`, `help` and `--version`. The human chose this as the safest route. A new command is blocked unless it is added to the list. The blocked command exits 4, a code no other refusal uses. Advised drift never blocks: the session-start hook names it until the project takes or declines it.

**Skips stay on one clone.** `skip_migrations` in `clm.local.toml` lets a clone run past a broken required migration, while the human reports it with `clm defect`. The first design was a committed skip with review, as `approval` has. It was dropped because it would have amended `humans-review-intent`, a carried `method` constraint. A skip that unblocks only one machine needs no other person's consent, since it can't weaken any other clone.

**The stamp's own edit is neutral.** `clm upgrade --apply` makes code changes, and a note's edit is spec. The gate leaves out a `clm.toml` whose only difference is `upgraded` and `declined_migrations` (`gate/upgrade-stamp-left-out`), so the stamp rides on either branch. `/clm:upgrade` lands the code part first, then the spec part.

**Keeping migrations honest** (`upgrades-carry-migrations`).
- *The corpus.* It holds, as gzipped tarballs, a project as each published release set it up. Tarballs keep the sample's test marker out of the coverage scan and out of the component's size.
- *The corpus test.* It is `clm`'s, since it runs the built binary. It upgrades each corpus project and requires three things: nothing is pending, `clm check` passes, and `clm init`'s files equal what it writes today. Because `clm` depends on `init`, a template change selects the test.
- *Each release.* It adds its own corpus entry and locks the hashes of the migrations it ships. A test refuses an edit to a locked migration, so a fix is a new migration.
- *The builder check.* `propose`'s builder check asks about migrations, so they are designed in the spec branch rather than found when the corpus test fails.

**`init-next-steps`' upgrade steps go.** They covered projects set up before `20261025-forge-adapters`. That change landed before `v0.14.0`, the first published release, so no released project needs them. Keeping them would also leave a second upgrade mechanism beside migrations.

**Not yet:**
- *Rewriting `specs/`, `changes/` or test markers.* That needs a generated change, as `clm rename` makes, so `specs/` stays derived (`regenerate-dont-merge`). The first migration that needs it designs it (`yagni`).
- *The gate replaying an upgrade branch,* as it replays a rename. It waits for the same need.
