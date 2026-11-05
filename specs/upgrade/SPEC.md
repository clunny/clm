---
title: Upgrade
status: unbuilt
owners: ["@clunny"]
depends_on: []
paths: ["crates/clm-upgrade/**"]
successor: ""
serves: [capability/upgrade-clm, architecture/upgrades-carry-migrations]
---

## Contract
Bringing a project set up by an older `clm` up to date with the one that runs (`upgrade-migrations`). Crate `clm-upgrade`, which depends on no other component's crate:

- `migrations()`: the ordered list of migrations the binary holds, each with `id`, `level` (`Required` or `Advised`), `done(root) -> bool`, and either `apply(root)` or a `note` (Markdown) with, optionally, `old` and `new`: the exact texts of a file `clm init` wrote before and writes since.
- `latest()`: the last migration's ID, which `init` writes as `upgraded` in a new `clm.toml`.
- `pending(root)`: the migrations a project still needs, or `TooNew(id)` when its `upgraded` names a migration this binary doesn't hold.
- `require_current(root)`: `Ok` when no `required` migration is pending outside `skip_migrations`; otherwise an error whose text names each such migration and says to run `/clm:upgrade` or `clm upgrade`, or, for `TooNew`, says the project needs a newer `clm`. `crates/clm` calls it before every command that isn't on its allow-list.
- `clm upgrade [--apply | --note <id> | --done <id>]`, dispatched by `crates/clm`.

It reads `clm.toml`'s `upgraded` and `clm.local.toml`'s `skip_migrations` itself, as plain TOML: the settings a migration fixes may be ones `config`'s loader rejects. It writes the working tree only, and commits nothing.

## Design
**Migrations.** `src/migrations/` holds one module per migration, named after its ID, and `src/migrations.rs` lists them in the order they landed on `main`, not by their IDs' dates. An ID is the ID of the change that needed the migration, suffixed `-2`, `-3` for a change that needs more than one. `done` is a pure function of the files under `root` that reads nothing but them, cheap enough to run for every migration on every command. `apply` exists only for a file or key CLM can rewrite exactly (a key rename, a `.gitignore` line); everything a project owns in words (`ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `scripts/clm-check.sh`) gets a `note`. A `note` migration with `old` and `new` is applied by `clm upgrade --apply` when the file equals one of `old`'s texts byte for byte, an unedited file being the project's only in name; any other text is left to the human.

**The stamp.** A `clm.toml` with `upgraded` has pending the migrations after that ID in the list. One without it, set up before this component, has pending each migration whose `done` is false, in list order. An `upgraded` naming an ID the list doesn't hold is `TooNew`. A `clm.toml` that doesn't parse as TOML counts as having no `upgraded`.

**`clm upgrade`.** With no flag it prints one line per pending migration, in order: `<id> <required|advised> <apply|note> <the note's first line>`, then nothing pending as `up to date`, and exits 0; for `TooNew` it prints the refusal and exits 1. `--apply` runs, in order, each pending migration's `apply` when its `done` is false, and the `old`/`new` rewrite of an unedited file, stopping at the first `note` migration whose `done` is still false. It then sets `upgraded` to the last migration it passed, editing that one line of `clm.toml` (or appending it), and prints `applied <id>` per migration it changed, `stopped at <id>` when it stopped. `--note <id>` prints that migration's note. `--done <id>` sets `upgraded` past the first pending migration when it is `<id>`: for a `required` one only when its `done` is true, exiting 1 otherwise; for an `advised` one in any case, since declining it is the human's call.

**Skips.** `skip_migrations`, only in `clm.local.toml` (`config`), lists required migrations `require_current` ignores on this clone, for a migration that is itself broken: the human reports it with `clm defect` and skips it until a fixed `clm` ships a fixed migration under a new ID. A skip doesn't advance `upgraded`. `clm upgrade` with no flag marks a skipped line `skipped`.

**The corpus** (`upgrades-carry-migrations`). `crates/clm-upgrade/corpus/v<version>/` holds a project as the release `v<version>` set it up, for every release from `v0.14.0`, the first one published: its `clm init --no-detectors --host none` output, and the sample in `corpus/sample/` (a component with one requirement, a test file citing it, a backlog item and an archived change) as that release's formats write it. `corpus/make.sh <version> <clm>` writes an entry with the given binary and fails unless that binary's `clm check` passes on it; the release runs it with the new tree's binary (`ci`), and the first entries are seeded by running it with each published release's binary (`gh release download`), editing the sample where an older release's format differs. The test that upgrades the corpus runs the built `clm`, so it lives in `crates/clm/tests/upgrade.rs`, where `capability`-free binary tests are.

**The lock.** `crates/clm-upgrade/migrations.lock` holds one line per shipped migration, `<id> <version> <sha256 of its module's source>`, appended by the release (`ci`). A test reads it and fails when a listed module's hash differs or a listed ID is missing from `migrations()`, so a shipped migration is fixed only by a new one.

## Requirements

### Requirement: migration-list
`clm-upgrade` SHALL offer `migrations()`, the migrations the binary holds in the order they landed, each with a unique `id`, a `level` of `required` or `advised`, a `done` check over the project's files, and either an `apply` or a `note`, and `latest()`, the last one's ID. A `note` migration MAY carry `old` and `new` file texts.

#### Scenario: unique and ordered
- GIVEN the binary's migrations
- WHEN `migrations()` is read
- THEN no two share an ID, each has exactly one of `apply` and `note`, and `latest()` is the last one's ID

#### Scenario: a fresh project is done
- GIVEN a repository `clm init` has just set up
- WHEN every migration's `done` runs on it
- THEN each returns true

### Requirement: pending-migrations
`pending(root)` SHALL return the migrations after the one `clm.toml`'s `upgraded` names, in list order; when `clm.toml` sets no `upgraded` or doesn't parse as TOML, each migration whose `done` is false; and `TooNew` naming the ID when `upgraded` names a migration the list doesn't hold.

#### Scenario: stamped
- GIVEN migrations `a`, `b`, `c` and `clm.toml` with `upgraded = "a"`
- WHEN `pending` runs
- THEN it returns `b` and `c`

#### Scenario: never stamped
- GIVEN migrations `a`, `b`, `c`, a `clm.toml` with no `upgraded`, and files for which only `b`'s `done` is false
- WHEN `pending` runs
- THEN it returns `b`

#### Scenario: upgraded by a newer clm
- GIVEN `clm.toml` with `upgraded = "z"` and no migration `z`
- WHEN `pending` runs
- THEN it returns `TooNew` naming `z`

### Requirement: require-current
`require_current(root)` SHALL return `Ok` when no pending migration is `required` outside the clone's `skip_migrations`, and otherwise an error naming each such migration and saying to run `/clm:upgrade` or `clm upgrade`; for `TooNew` the error SHALL say the project was upgraded by a newer `clm` and name the migration.

#### Scenario: required pending
- GIVEN a pending `required` migration `r` and a pending `advised` migration `a`
- WHEN `require_current` runs
- THEN it fails, naming `r` and `clm upgrade`, and not naming `a`

#### Scenario: advised only
- GIVEN only a pending `advised` migration
- WHEN `require_current` runs
- THEN it returns `Ok`

#### Scenario: skipped on this clone
- GIVEN a pending `required` migration `r` and `clm.local.toml` with `skip_migrations = ["r"]`
- WHEN `require_current` runs
- THEN it returns `Ok`, and `clm upgrade` lists `r` as `skipped`

### Requirement: upgrade-list
`clm upgrade` with no flag SHALL print one line per pending migration in order, `<id> <required|advised> <apply|note> <the note's first line>`, or `up to date` when none is pending, and exit 0; for `TooNew` it SHALL print the refusal and exit 1. It SHALL write nothing.

#### Scenario: listing
- GIVEN pending migrations `r` (required, apply) and `a` (advised, note "CLAUDE.md names /clm:help")
- WHEN `clm upgrade` runs
- THEN it prints `r required apply` and `a advised note CLAUDE.md names /clm:help` in that order, exits 0 and `git status` shows no change

### Requirement: upgrade-apply
`clm upgrade --apply` SHALL, in list order, run each pending migration's `apply` when its `done` is false, and rewrite a `note` migration's file from `old` to `new` when the file equals one of its `old` texts, stopping at the first `note` migration whose `done` is still false. It SHALL then set `clm.toml`'s `upgraded` to the last migration it passed, changing no other line of `clm.toml`, print `applied <id>` for each migration it changed and `stopped at <id>` when it stopped, and commit nothing. Run again, it SHALL change nothing more.

#### Scenario: mechanical, then an edited file
- GIVEN pending migrations `m` (apply) and `n` (note with `old`/`new`), and the file `n` concerns edited by the project
- WHEN `clm upgrade --apply` runs
- THEN `m`'s change is made, `upgraded = "m"`, it prints `applied m` and `stopped at n`, and a second run prints `stopped at n` and changes nothing

#### Scenario: unedited file
- GIVEN pending migration `n` with `old`/`new`, and the file it concerns equal to its `old` text
- WHEN `clm upgrade --apply` runs
- THEN the file equals `new` and `upgraded = "n"`

### Requirement: upgrade-note-and-done
`clm upgrade --note <id>` SHALL print the migration's note. `clm upgrade --done <id>` SHALL set `upgraded` to `<id>` when `<id>` is the first pending migration and is `advised`, or is `required` and its `done` is true; otherwise it SHALL exit 1 naming why and change nothing.

#### Scenario: declining advised drift
- GIVEN the first pending migration `a`, advised, whose `done` is false
- WHEN `clm upgrade --done a` runs
- THEN `upgraded = "a"` and it exits 0

#### Scenario: required not done
- GIVEN the first pending migration `r`, required, whose `done` is false
- WHEN `clm upgrade --done r` runs
- THEN it exits 1 naming `r`, and `clm.toml` is unchanged

### Requirement: corpus-entry-script
`crates/clm-upgrade/corpus/make.sh <version> <clm>` SHALL write `crates/clm-upgrade/corpus/v<version>/`: what `<clm> init --no-detectors --host none` writes in an empty git repository, plus `corpus/sample/`, and SHALL fail, leaving no entry, unless `<clm> check` passes on the result.

#### Scenario: the current tree
- GIVEN the built `clm`
- WHEN `make.sh 9.9.9 <clm>` runs in a temporary copy of the corpus directory
- THEN `v9.9.9/` holds `clm.toml`, `ARCHITECTURE.md`, `CLAUDE.md` and the sample's files, and it exits 0

### Requirement: corpus-upgrades
For every entry under `crates/clm-upgrade/corpus/`, one per published release from `v0.14.0`, upgrading a copy with the current `clm upgrade --apply`, and writing each `note` migration's `new` text where it stops, SHALL leave a project where nothing is pending, `clm check` passes, and each file `clm init` writes equals what the current `clm init` writes for the same flags, `clm.toml`'s `upgraded` line aside.

#### Scenario: every release upgrades
- GIVEN the corpus entries `v0.14.0` to the latest release
- WHEN each is upgraded so
- THEN `clm upgrade` prints `up to date` and `clm check` exits 0 for each

#### Scenario: a template changes without a migration
- GIVEN the current tree with a word changed in the `CLAUDE.md` template and no migration added
- WHEN the corpus is upgraded so
- THEN the test fails, naming `CLAUDE.md` and the corpus entry

### Requirement: shipped-migrations-frozen
For each line `<id> <version> <hash>` of `crates/clm-upgrade/migrations.lock`, `migrations()` SHALL hold `<id>`, and the SHA-256 of its module's source SHALL be `<hash>`.

#### Scenario: a shipped migration edited
- GIVEN a lock line for migration `m` and a later edit to `m`'s module
- WHEN the tests run
- THEN one fails, naming `m` and the release that shipped it

### Requirement: migration-carried-constraints-copy
An `advised` `note` migration SHALL bring an `ARCHITECTURE.md` that copies CLM's carried constraints under `## Enforced by clm`, as `clm init` wrote it up to `v0.16.0`, to the pointer at `clm guide constraints` it writes today. Its `done` SHALL be true when `ARCHITECTURE.md` holds no `## Enforced by clm` heading, and its `old`/`new` SHALL be the two templates' texts.

#### Scenario: an unedited 0.16 file
- GIVEN an `ARCHITECTURE.md` equal to what `clm init` `v0.16.0` wrote
- WHEN `clm upgrade --apply` runs
- THEN it equals what the current `clm init` writes

#### Scenario: an edited one
- GIVEN that file with a constraint of the project's own added
- WHEN `clm upgrade --apply` runs
- THEN the file is unchanged and it prints `stopped at` this migration

### Requirement: migration-claude-md-names-help
An `advised` `note` migration SHALL bring a `CLAUDE.md` that `clm init` wrote before it named `/clm:help` to the text it writes today. Its `done` SHALL be true when `CLAUDE.md` names `/clm:help` or doesn't exist, and its `old`/`new` SHALL be the two templates' texts.

#### Scenario: an unedited file
- GIVEN a `CLAUDE.md` equal to what `clm init` `v0.16.0` wrote
- WHEN `clm upgrade --apply` runs
- THEN it equals what the current `clm init` writes
