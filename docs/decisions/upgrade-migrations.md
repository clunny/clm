---
title: "Projects upgrade through ordered migrations in the binary, stamped in clm.toml; a pending required one blocks"
date: 2026-11-05
status: accepted
---

A project set up by one release of `clm` must keep working under any later one, skipping releases in between. `init-cli-and-skill` gave every file `clm init` writes to the project, and `init-rerun` never rewrites one, so nothing carried a later release's changes into an existing project; `plain-terms-renames` assumed no project but CLM's own used CLM, so none needed a migration. Since `v0.14.0` CLM is published, and that assumption no longer holds.

**Migrations.** `clm-upgrade` holds an ordered list of migrations, each named after the change that needed it, with a level (`required`: a later `clm` would misread the project without it; `advised`: the project's files only differ from what `clm init` writes today), a `done` check over the project's files, and either code (`apply`, only for what CLM can rewrite exactly) or a note for the files the project owns in words. A note may carry the old and new template texts, so an unedited file is rewritten without a person.

**The stamp.** `clm.toml`'s `upgraded` names the last migration the project took. The list's order then gives what is pending, cheaply enough to check before every command, and an `upgraded` the binary doesn't know means the project was upgraded by a newer `clm`, which an older one refuses. A project from before the stamp is judged by each migration's `done` once. Deriving everything from the files on every command (no stamp) was rejected: some checks would read every test on every command, an older binary couldn't tell a project was too new for it, and the checks would grow forever.

**Blocking.** A pending `required` migration blocks every command but a short allow-list (`status`, `guide`, `upgrade`, `defect`, `config`, `init`, `backlog`, `hook`, `view`, `help`, `--version`): the human chose the safest route. New commands are blocked unless listed. Advised drift never blocks; the session-start hook names it until the project takes or declines it.

**Skips stay on one clone.** `skip_migrations` in `clm.local.toml` lets a clone run past a broken required migration while the human reports it with `clm defect`. A committed skip with review, as `approval` has, was the first design; it would have amended `humans-review-intent`, a carried `method` constraint, and a skip that only unblocks one machine needs no other person's consent, since it can't weaken another clone.

**The stamp's own edit is spec.** A branch that edits only the files a project owns in words, and moves `upgraded`, would otherwise be `mixed`, so the gate counts a `clm.toml` that differs only in `upgraded` as a spec change needing no review (`gate/upgrade-stamp-is-spec`). `/clm:upgrade` lands mechanical migrations, which are code, on a branch of their own before the note edits.

**Keeping migrations honest** (`upgrades-carry-migrations`). A corpus holds a project as each published release set it up; a test upgrades each to the current tree and requires that nothing is pending, `clm check` passes and `clm init`'s files match what it writes today. A release adds its entry and locks the hashes of the migrations it ships, and a test refuses an edit to a locked one: a fix is a new migration. `propose`'s builder check asks about migrations so they are designed in the spec branch, not found when the corpus test fails.

**`init-next-steps`' upgrade steps go.** They covered projects set up before `20261025-forge-adapters`, which landed before `v0.14.0`, the first published release, so no released project needs them, and keeping a second upgrade mechanism beside migrations would split where upgrades live.

**Not yet.** Rewriting `specs/`, `changes/` or test markers needs a generated change, as `clm rename` makes, so `specs/` stays derived (`regenerate-dont-merge`); the first migration that needs it designs it (`yagni`). The gate replaying an upgrade branch, as it replays a rename, waits for the same need.
