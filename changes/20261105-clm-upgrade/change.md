---
status: applied
---

## Intent

Let a project upgrade `clm` safely, from any earlier release to the latest, skipping the ones in between. The project records the last migration it took. Each change that alters something adopters already hold ships a migration with it. `clm upgrade` applies the mechanical migrations, and `/clm:upgrade` helps the human edit the files the project owns. A required migration that is still pending blocks the commands that judge or write. Advised drift is only raised at session start. A release freezes the migrations it ships and records an example project as that release set it up, and a test upgrades every recorded project to the current tree, so a forgotten migration fails a land instead of failing an adopter.

## Problem

Today nothing records which `clm` set up a project or last brought it up to date. `clm init` never rewrites a file it wrote (`init-rerun`, `init-cli-and-skill`), so what a newer release would write differently never reaches existing projects. For example, a project set up by 0.16.0 still holds the "Enforced by clm" copy of the carried constraints in `ARCHITECTURE.md`, which 0.18.0 replaced with `clm guide constraints`, and its `CLAUDE.md` doesn't name `/clm:help`. The one upgrade path is `init-next-steps`' fixed list of hand steps for projects set up before `20261025-forge-adapters`. Old names CLM reads are read only until their `clm:remove-by` contract step (`expand-contract`), so a project that skips the releases in that window breaks with no explanation. `plain-terms-renames` assumed that no project but CLM uses CLM, so no adopter needs a migration; since `v0.14.0` CLM is published.

## Approaches

- **A. A stamp in the project, and a check inside each migration (chosen).** `clm.toml` records the last migration the project took. Comparing it with the binary's ordered list is cheap and exact, so a command can block on it, and an older `clm` can refuse a project upgraded past it. Each migration's own check makes `clm upgrade` safe to rerun and safe after a half-finished upgrade. Cost: one more `clm.toml` key.
- **B. No stamp; derive the state from the files every time.** Nothing can fall out of sync, but some checks are costly on every command (proving every test marker was restamped means reading every test), an older binary can't tell a project is too new for it, and the checks grow with every release, forever.
- **C. Agent only.** Upgrade notes as guide pages a skill acts on. Cheapest, but nothing deterministic decides what blocks, so it can't enforce the upgrade (`deterministic-enforcement`). Dropped.

The human chose A; chose to block on a pending required migration, as the safest route, with a way to report a migration that is itself broken; and chose to only raise advised drift at session start.

## Design

The decision `upgrade-migrations` records the choices and their reasons; the deltas state them as rules.

- **Migrations** (new component `upgrade`, crate `clm-upgrade`): an ordered list in the binary, each named after the change that needed it, `required` or `advised`, with a `done` check over the project's files and either an `apply` (only for what CLM can rewrite exactly) or a `note` for the files the project owns in words. A note may carry the old and new template texts, so an unedited file is rewritten without a person.
- **The stamp**: `clm.toml`'s `upgraded`, the last migration taken. `clm init` writes it as the newest migration, so a new project starts current. A project with none is judged by each migration's `done`. An `upgraded` the binary doesn't hold means the project is too new for it. `config` checks the key's shape and that only `clm.toml` sets it; `upgrade` reads it as plain TOML, since the settings a migration fixes may be ones `config` rejects.
- **Blocking**: `clm` calls `require_current` before every command but `--version`, `help`, `status`, `guide`, `upgrade`, `defect`, `config`, `init`, `backlog`, `hook` and `view`, so a command added later blocks by default.
- **Skipping a broken migration**: `skip_migrations` in the clone's `clm.local.toml` only, after `clm defect` reports it. It unblocks one machine and can't weaken another clone, so it needs no review and leaves `humans-review-intent`, a carried `method` constraint, as it is. This replaces the committed, reviewed skip first discussed.
- **`clm upgrade`** lists what is pending; `--apply` applies the mechanical migrations and rewrites unedited files, stopping at the first note the human must take; `--note <id>` prints a note; `--done <id>` moves past a note migration, past an advised one even when declined, past a required one only once its `done` holds. It writes the working tree only.
- **The gate**: a `clm.toml` that differs only in `upgraded` counts as a spec change needing no review, so a branch of note edits plus the stamp lands as `spec`, not `mixed`. An edit to `ARCHITECTURE.md` or `CAPABILITIES.md` still needs review, which is right: the team owns those words.
- **`/clm:upgrade`**: runs `clm upgrade --apply` on an `upgrade/<YYYYMMDD>` branch, lands mechanical changes (code) on their own, then shows each note's edit beside the project's text, makes it on a yes, and lands the notes branch; reports a failing migration with `clm defect` and offers the local skip.
- **Session start**: after the status, one line with the counts of pending required and advised migrations, whether commands are blocked, and `/clm:upgrade`. It never blocks.
- **Keeping migrations honest** (new constraint `upgrades-carry-migrations`): `crates/clm-upgrade/corpus/v<version>/` holds a project as each published release from `v0.14.0` set it up, written by `corpus/make.sh` and seeded with each published binary. A test upgrades every entry and requires that nothing is pending, `clm check` passes and `clm init`'s files equal what it writes today. `migrations.lock` holds the hash of each shipped migration, and a test refuses an edit to one. `propose`'s builder check asks whether a change alters what existing projects hold.
- **Release** (decision `release-records-upgrade-corpus`): `release-prepare.sh`'s bump commit adds the release's corpus entry and lock lines.
- **First migrations**: two `advised` notes with old/new texts: `ARCHITECTURE.md` stops copying the carried constraints, and `CLAUDE.md` names `/clm:help`. Whatever else the corpus test shows a `v0.14.0` to `v0.18.0` project needs becomes a migration in the same build.
- **`init-next-steps`** stops printing upgrade steps: they covered projects from before `20261025-forge-adapters`, which landed before the first published release, so no released project needs them.

## Non-goals

- Downgrading: an older `clm` refuses a project too new for it.
- Rewriting `specs/`, `changes/` or test markers: such a rewrite would go through a generated change, as `clm rename` does, so `specs/` stays derived (`regenerate-dont-merge`). The first migration that needs it designs that path (`yagni`).
- The gate replaying an upgrade branch to trust it, as it replays a rename.
- Upgrading the plugin or the binary itself: `claude plugin update` and the shim do that (`binary-from-releases`).
- A pending-migrations line in `clm status`: the session-start line and the block cover it.

## Constraints

- `deterministic-enforcement`: what blocks is decided by the stamp, the list and each migration's `done`, never by a model.
- `init-cli-and-skill`: code rewrites only what CLM can rewrite exactly or a file still equal to a template; the rest goes through the skill with the human's yes.
- `settings-before-use`: `upgraded`'s reader lands and is installed before this repository's `clm.toml` sets it; `clm upgrade --apply`, run once the build is installed, sets it here.
