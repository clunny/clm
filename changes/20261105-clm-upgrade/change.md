---
status: proposed
---

## Intent

Let a project upgrade `clm` safely, from any earlier release to the latest, skipping the ones in between. The project records the last migration it took. Each change that alters something adopters already hold ships a migration with it. `clm upgrade` applies the mechanical migrations, and the `/clm:upgrade` skill helps the human edit the files the project owns. A required migration that is still pending blocks the commands that judge or write. Advised drift is only raised at session start. A release freezes the migrations it ships and records an example project as that release set it up, and a test upgrades every recorded project to the current tree. That test is how a forgotten migration fails a land instead of failing an adopter.

## Problem

Today nothing records which `clm` set up a project or last brought it up to date. `clm init` never rewrites a file it wrote (`init-rerun`, `init-cli-and-skill`), so what a newer release would write differently never reaches existing projects. For example, a project set up by 0.16.0 still holds the "Enforced by clm" copy of the carried constraints in `ARCHITECTURE.md`, which 0.18.0 replaced with `clm guide constraints`. Its `CLAUDE.md` also doesn't name `/clm:help`. The one upgrade path is `init-next-steps`: a fixed list of hand steps for projects set up before `20261025-forge-adapters`, printed on a rerun of `clm init`. Old names CLM reads are read only until their `clm:remove-by` contract step (`expand-contract`), so a project that skips the releases in that window breaks with no explanation. `plain-terms-renames` assumed that no project but CLM uses CLM, so no adopter needs a migration. This change drops that assumption.

## Approaches

- **A. A stamp in the project, and a check inside each migration (chosen).** `clm.toml` records the last migration the project took. Comparing it with the binary's ordered list of migrations is cheap and exact, so a command can block on it. It also lets an older `clm` refuse a project upgraded past it. Each migration's own check makes `clm upgrade` safe to rerun and safe after a half-finished upgrade. Cost: one more `clm.toml` key.
- **B. No stamp; derive the state from the files every time.** Nothing can fall out of sync, but some checks are costly on every command (proving every test marker was restamped means reading every test). An older binary can't tell that a project is too new for it. The set of checks also grows with every release, forever.
- **C. Agent only.** Upgrade notes as guide pages, which a skill reads and acts on. Cheapest to build, but nothing deterministic decides what blocks, so it can't enforce the upgrade the human asked for (`deterministic-enforcement`). Dropped.

The human chose A. They chose blocking on a pending required migration, as the safest route, with a way to report a migration that is itself broken. They chose only raising advised drift at session start, without blocking.

## Design

**Migrations** (component `upgrade`, crate `clm-upgrade`). A migration is one entry in an ordered list compiled into the binary. Its ID is the ID of the change that needed it. A change that needs two migrations suffixes the second (`<change-id>-2`). Each entry has:
- `level`: `required` when a `clm` built after it would misread or reject a project without it, and `advised` when the project's own files only differ from what a fresh `clm init` writes;
- `done(root) -> bool`: a pure, cheap check of the project's files saying whether the migration's result already holds;
- one of:
  - `apply(root)`: code that makes the change. Used only for files or keys CLM owns, or rewrites as exact as a key rename;
  - `note`: Markdown for the agent, naming the file, what changed and why, and the result to reach, for files the project owns (`ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `scripts/clm-check.sh`).

The list's order is the order the migrations landed on `main`, not the order of their dates. Appending is the only edit to an unreleased entry's place. A shipped entry is frozen (**Release**).

**The stamp.** `clm.toml` gains `[upgrade]` with `last = "<migration-id>"`, the last migration the project took, and an optional `skip` (below). `clm init` writes `last` as the newest migration the running binary holds, so a new project starts current. For a project with no `last` (one set up before this change), the pending migrations are those whose `done` is false, checked over the whole list once. After that, the pending migrations are those after `last` in the list. `config` reads the key. Under `settings-before-use`, the reader lands and is installed before this repository's `clm.toml` sets it.

**Too new.** When `last` names a migration the running binary doesn't hold, the project was upgraded by a newer `clm`. Every command except `clm --version`, `clm guide` and `clm defect` then refuses to run, and the refusal says the project needs a newer `clm`.

**Blocking.** While any pending migration is `required` and isn't skipped, the commands that judge or write the project's state refuse to run: `check`, `gate`, `land`, `apply`, `rebase`, `claim`, `run`, `approve`, `rename`, `archive` and the rest. The refusal names each pending required migration and says to run `/clm:upgrade` or `clm upgrade`. These always run: `--version`, `status`, `guide`, `upgrade`, `defect`, `config`, `init`, `backlog`, `hook` (so the guard and autosync never stop), and the read-only views. `clm status` adds a line saying how many required and advised migrations are pending. The check lives in one function in `clm-upgrade`. `crates/clm` calls it before dispatching any command not on the allow-list, so a new command blocks by default.

**Skipping a broken migration.** `[upgrade] skip = ["<migration-id>"]` takes a required migration out of the block. Each entry needs a comment naming the defect reported against it, and `clm check` warns while the list isn't empty. Changing `skip` makes a branch need review, as changing `approval` does (`humans-review-intent`): skipping turns off a correctness block, so a human decides. `clm defect` always runs, so the broken migration can be reported first. The skip doesn't advance `last`. `clm upgrade` tries the migration again once a fixed `clm` holds a fixed migration with a new ID.

**`clm upgrade`.**
- With no flags, it lists the pending migrations in order: ID, level, `apply` or `note`, and the first line of the note.
- `clm upgrade --apply` runs `apply` for each pending migration whose `done` is false, in order, and sets `last` past every pending migration that has `apply`, stopping at the first `note` migration whose `done` is false.
- `clm upgrade --note <id>` prints that migration's note.
- `clm upgrade --done <id>` advances `last` past a `note` migration once the human has taken or declined its edit. For a `required` migration it refuses while `done` is false. For an `advised` one it advances anyway, since declining advised drift is the human's call.

It writes the working tree and commits nothing, as `clm rename` does.

**The skill.** `/clm:upgrade` (plugin) cuts a branch `upgrade/<YYYYMMDD>` from `origin/main` in a worktree of its own. It runs `clm upgrade --apply` and commits. For each `note` migration it shows the human the note and its proposed edit beside the project's current text, makes the edit on a yes, and runs `clm upgrade --done <id>`. It commits, runs `clm check`, and lands by the gate's first line, as any branch does. An edit to `ARCHITECTURE.md` or `CAPABILITIES.md` makes the branch need review, which is right, since the team owns those words. A failure the skill can't resolve is reported with `clm defect`, and the skill offers the skip.

**The session-start reminder.** `clm hook session-start` (`plugin.hooks`) adds, after the status, one line when any migration is pending: how many are required and how many advised. When some are required, it says that commands are blocked. It ends by suggesting `/clm:upgrade`. It never blocks the session.

**Making sure new changes carry a migration.** Two rules, one deterministic and one advisory:
- *The corpus test* (`upgrade`). `crates/clm-upgrade/corpus/v<version>/` holds, for each release from `v0.14.0`, the first one published, a small project as that release set it up: its `clm init` output with the default flags, plus a sample component with one requirement, a test citing it, a backlog item and an archived change, written in that release's formats. A test copies each one, runs every migration's `apply` and, for `note` migrations, writes the note's stated result. It then asserts that `clm check` passes, every migration's `done` holds, and every file CLM owns equals what the current `clm init` writes. A change that alters a template, drops an old name at its contract step, or changes a format CLM reads without adding a migration fails this test at land time. A new architectural constraint, `upgrades-carry-migrations`, is enforced by it.
- *The builder check*. `/clm:propose`'s builder check also asks whether the change alters a file, key or format that existing projects already hold. When it does, the change adds the migration as a requirement of `upgrade`, so the migration is designed on the spec branch rather than discovered when the land fails.

**Release** (`ci`). `release-prepare.sh`, in the bump commit it already makes, also:
- writes `crates/clm-upgrade/corpus/v<version>/` from the new tree's `clm init` and the sample, made by a script the test suite runs too;
- appends to `crates/clm-upgrade/migrations.lock` one entry per migration that release ships for the first time: its ID and a hash of its source text.

A test fails when a locked migration's text no longer matches its hash, so a shipped migration is never edited; a fix is a new migration. This widens `release-by-dispatch`'s bump commit from three version fields to those fields plus two generated artefacts, still made by the workflow and never by hand.

**The first migrations.** The four steps `init-next-steps` prints for projects set up before `20261025-forge-adapters` become migrations, all `required` with `apply`, except deleting a generated workflow, which is `required` with `note`, since deleting a workflow is outward-facing. These are:
- adding `host`;
- deleting `merge`;
- adding `[land] check`;
- deleting a generated workflow.

The upgrade-step part of `init-next-steps` and its Design text are then removed (`no-dead-code`). Two more migrations are added, both `advised` with `note`:
- `ARCHITECTURE.md` no longer copies the carried constraints (`f5692b5`);
- `CLAUDE.md` names `/clm:help` (`da97b43`).

So do the `.gitignore` lines `clm init` adds on a rerun, as `required` with `apply`. `clm init` keeps adding them for new projects.

**Seeding the corpus.** The builder downloads each published release's binary (`v0.14.0` to the latest) with `gh release download`, runs its `clm init` on an empty repository, and adds the sample in that release's format. Every later release adds its own entry through `release-prepare.sh`.

## Non-goals

- Downgrading. An older `clm` refuses a project that is too new for it.
- Rewriting `specs/`, `changes/` or test markers. No pending format change needs it yet, and such a rewrite would have to go through a generated change, as `clm rename` does, so `specs/` stays derived (`regenerate-dont-merge`). The first migration that needs it designs that path (`yagni`).
- The gate replaying an upgrade branch to trust it, as it replays a rename. Until then, an upgrade branch is judged like any other, by what it edits.
- Upgrading the plugin or the binary itself: `claude plugin update` and the shim already do that (`binary-from-releases`).

## Constraints

- `deterministic-enforcement`: what blocks is decided by the stamp, the list and each migration's `done`, never by a model. The skill only helps with the edits the project owns.
- `init-cli-and-skill`: files the project owns are never rewritten by code. Only CLM-owned files and keys are, and the rest go through the skill with the human's yes.
- `settings-before-use`, `expand-contract`: the `[upgrade]` reader lands first, and `init-next-steps`' steps are removed only once the migrations that replace them are built.
- `humans-review-intent`: `[upgrade] skip` joins `approval` as a `clm.toml` setting whose change needs review.
