---
status: done
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

- **Migrations** (new component `upgrade`, crate `clm-upgrade`). They form an ordered list in the binary, each named after the change that needed it, with a `done` check over the project's files. There are two kinds:
  - an *apply* migration is code `clm upgrade --apply` runs. Every `required` migration is one, since what CLM must read correctly it can rewrite exactly;
  - a *note* migration is always `advised`: guidance for a file the project owns in words. It may carry old and new template texts, so a file nobody edited is rewritten without a person.
- **The stamp.** `clm.toml`'s `upgraded` names the last migration taken, and `clm init` writes the newest one.
  - An apply migration is pending when it comes after the stamp and its `done` is false. A note migration is pending while its `done` is false and `declined_migrations` doesn't name it.
  - A project with no stamp is judged by `done` alone, so the block lifts as soon as the files are right.
  - An unknown stamp means the project is too new for this binary.
  - `config` checks both keys' shape, and that only `clm.toml` sets them. `upgrade` reads them as plain TOML, since the settings a migration fixes may be ones `config` rejects.
- **Blocking.** `clm` calls `require_current` before every command except `--version`, `help`, `status`, `guide`, `upgrade`, `defect`, `config`, `init`, `backlog`, `hook` and `view`, and exits 4 when a required migration is pending. It checks the command's `--root`, or else the repository found from the current directory. A command added later is blocked by default.
- **Skipping a broken migration.** `skip_migrations` may only be set in the clone's `clm.local.toml`, which `config`'s new `local_settings_path` locates, and the migration should be reported with `clm defect` first. The skip unblocks one machine and can't weaken another clone. So it needs no review, and it leaves `humans-review-intent`, a carried `method` constraint, untouched. This replaces the committed, reviewed skip first discussed.
- **`clm upgrade`.**
  - With no flag, it lists what is pending.
  - `--apply` runs every pending apply migration, rewrites unedited files for the notes, sets `upgraded` to the newest migration, and names the notes still to take. It never stops early.
  - `--note <id>` prints a note.
  - `--decline <id>` records a declined note.
  - `--lock <version>` is the release's.

  It writes the working tree only.
- **The gate.** A `clm.toml` whose only difference is `upgraded` and `declined_migrations` is left out of what the gate classifies. So the stamp rides on a code branch of mechanical migrations, or on a spec branch of note edits, without making either `mixed`. Edits to `ARCHITECTURE.md` and `CAPABILITIES.md` still need review.
- **`/clm:upgrade`.**
  - It runs `--apply` on an `upgrade/<YYYYMMDD>` branch.
  - It walks each note with the human, editing on a yes and declining on a no.
  - It lands the code part, then the spec part (`upgrade/<YYYYMMDD>-notes`).
  - A failing migration is reported with `clm defect`, and the skill offers the local skip.
- **Session start.** After the status, one line gives the counts of pending required and advised migrations, says whether commands are blocked (a skipped migration isn't counted as blocking), and suggests `/clm:upgrade`. It never blocks.
- **Keeping migrations honest** (new constraint `upgrades-carry-migrations`).
  - *The corpus.* `crates/clm-upgrade/corpus/v<version>.tar.gz` holds a project as each published release from `v0.14.0` set it up. The project is `clm init --lang rust --no-detectors --host none` plus a sample. Entries are written by `corpus/make.sh <version> <clm>`, which is deterministic, and seeded with each published binary. Tarballs keep the sample's marker out of the coverage scan and out of the component's size.
  - *The corpus test* (`clm/upgrade-corpus`, in `crates/clm/tests/upgrade.rs`). It upgrades every entry and requires three things: nothing is pending, `clm check` passes, and `clm init`'s files equal what it writes today. It belongs to `clm`, which depends on `init`, so a template change selects it.
  - *The lock.* `migrations.lock` holds each shipped migration's directory hash, and a test refuses an edit to one.
  - *The builder check.* `propose`'s builder check asks whether a change alters what existing projects hold.
- **Release** (decision `release-records-upgrade-corpus`, with `release-by-dispatch` amended).
  - `release-prepare.sh` builds the bumped tree's `clm`, writes the release's corpus entry and lock lines, and puts them into the bump commit before its tests run.
  - A release without a bump needs a corpus entry for its version already.
- **First migrations.** Two advised notes, each carrying old and new texts:
  - `ARCHITECTURE.md` stops copying the carried constraints;
  - `CLAUDE.md` names `/clm:help`.

  If the corpus test shows that a `v0.14.0` to `v0.18.0` project needs anything more, the build files a `question` item blocking `clm/upgrade-corpus`, since a new migration is a new requirement.
- **Removed: `init-next-steps`' upgrade steps.** `clm init` stops printing them. They covered projects set up before `20261025-forge-adapters`, which landed before the first published release, so no released project needs them. The build removes the tests behind them.
- **Docs.** The guide's reference page lists `clm upgrade` and the three settings. The glossary defines a migration and its kinds. The decisions `plain-terms-renames`, `init-cli-and-skill`, `local-settings` and `release-by-dispatch` carry amendments.
- **This repository.** `backlog/stamp-this-repository-upgraded` stamps this repository once the build is installed, on a branch of its own (`settings-before-use`).

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

## Open questions

- `git-is-truth` (left from the architecture audit): `require_current` reads the untracked `clm.local.toml`'s `skip_migrations` to decide whether `clm check` and `clm land` run. Left as is: `local-settings` already lets that file set repository keys such as `land`, a skip only lifts the block for one clone without changing what any check decides, and the alternative, a committed skip, would amend `humans-review-intent`.
- Seeding the corpus needs each published release's binary. An unattended build that can't download them files a `human-task` item blocking `upgrade/corpus-seeded`, and builds the rest.
