---
title: Upgrade
status: active
owners: ["@clunny"]
depends_on: [config]
paths: ["crates/clm-upgrade/**"]
successor: ""
serves: [capability/upgrade-clm, architecture/upgrades-carry-migrations]
---

## Contract
Bringing a project set up by an older `clm` up to date with the one that runs (`upgrade-migrations`). Crate `clm-upgrade`:

- `migrations()`: the ordered list of migrations the binary holds. Each has an `id`, a `level` (`Required` or `Advised`) and a `done(root) -> bool`, and is one of:
  - an *apply* migration, with `apply(root)`. Every `required` migration is one;
  - a *note* migration, always `advised`, with a `note` (Markdown) and, optionally, `old` (one or more texts) and `new`, the exact texts of a file `clm init` wrote before and writes since.
- `latest()`: the last migration's ID, which `init` writes as `upgraded` in a new `clm.toml`.
- `pending(root)`: the migrations a project still needs, or `TooNew(id)` when its `upgraded` names a migration this binary doesn't hold.
- `require_current(root)`: `Ok` when no `required` migration is pending outside the clone's `skip_migrations`; otherwise an error naming each such migration and saying to run `/clm:upgrade` or `clm upgrade`, or, for `TooNew`, saying the project needs a newer `clm`. `crates/clm` calls it before every command not on its allow-list.
- `clm upgrade [--apply | --note <id> | --decline <id> | --lock <version>]`, dispatched by `crates/clm`.

It reads `clm.toml`'s `upgraded` and `declined_migrations` itself, as plain TOML, since the settings a migration fixes may be ones `config`'s loader rejects. It reads `clm.local.toml`'s `skip_migrations` from the file `config`'s `local_settings_path` names, also as plain TOML. It writes the working tree only and commits nothing.

## Design
**Migrations.** Each migration is a directory `src/migrations/<id>/`, holding `mod.rs` and any text it embeds with `include_str!` (its `old` and `new`), pulled in with `#[path]`, since an ID such as `20261105-clm-upgrade` isn't a Rust identifier. `src/migrations.rs` lists them in the order they landed on `main`, not by their IDs' dates. An ID is the ID of the change that needed the migration, suffixed `-2`, `-3` for a change that needs more than one. `done` reads only files under `root`, so it is cheap enough to run on every command for the migrations it's asked about. An apply migration exists only for what CLM can rewrite exactly (a key rename, a `.gitignore` line). What CLM must read correctly it can rewrite exactly, which is why every `required` migration is an apply migration and a note never blocks. A note migration's `old`/`new` texts are copies of `clm-init`'s templates as of the releases they name, embedded here, since `upgrade` doesn't depend on `init`.

**Pending.** For a `clm.toml` with `upgraded = "<id>"`:
- an apply migration is pending when it comes after `<id>` in the list and its `done` is false;
- a note migration is pending when its `done` is false and `declined_migrations` doesn't name it, wherever it sits in the list.

A `clm.toml` with no `upgraded`, or one that doesn't parse as TOML, has every migration whose `done` is false and that isn't declined pending. So the block lifts as soon as the files are right, whatever the stamp says. The stamp lets `done` skip what the project already took, and tells a binary that the project is too new for it. A note that asks the project to read it and edit nothing has a `done` that reads the stamp: true once `upgraded` names that note or a migration after it in the list, so a project `clm init` set up since never sees it, and a project upgraded past it has taken it (`stamp-taken-notes`).

**`clm upgrade`.**
- *No flag.* One line per pending migration, in list order: `<id> <required|advised> <apply|note> <the note's first line>`, with `skipped` appended for a required one in `skip_migrations`; or `up to date`. Exits 0.
- *`--apply`.* Runs, in order, each pending apply migration, and rewrites a pending note migration's file to `new` when the file equals one of its `old` texts. It then sets `upgraded` to `latest()`, and prints `applied <id>` per migration it changed and `note <id>` per note migration still pending after its rewrites, judged before it writes `upgraded`, so a note the stamp takes is still shown on the run that takes it. It writes `upgraded` by replacing its line, or else as a new line above the first table header's lead-in (at the end when there is none), so it stays a top-level key. Every other line is left as it is. It never stops early. A run with nothing to change writes nothing.
- *`--note <id>`.* Prints the migration's note.
- *`--decline <id>`.* Adds a pending note migration's ID to `declined_migrations`, the same way `upgraded` is written. The session-start reminder then stops naming it.
- *`--lock <version>`.* Appends to `migrations.lock` one line per migration the lock doesn't list (**The lock**). Only the release runs it (`ci`).

`--apply`, `--note`, `--decline` and `--lock` on `TooNew` exit 1 and write nothing. So do `--note` or `--decline` naming an ID that isn't a migration, and `--decline` naming one that isn't a pending note migration.

**A new top-level line.** A header's *lead-in* is the run of comment lines directly above it, with no blank line between, and the blank lines directly above that run (only the blank lines when no comment is there). A new top-level line goes above the first table header's lead-in, so a comment that describes a header stays next to it, and is followed by a blank line when no blank line already follows it there. A comment run that reaches the top of the file is part of the lead-in too, so the line then goes first: a file comment written with no blank line before the first header can't be told from that header's own. One function writes it for `upgraded` and `declined_migrations` alike.

**Skips.** `skip_migrations`, only in `clm.local.toml` (`config`), lists required migrations that `require_current` ignores on this clone, for a migration that is itself broken. The human reports it with `clm defect` and skips it until a fixed `clm` ships a fixed migration under a new ID. A skip writes nothing to `clm.toml`.

**The lock.** `crates/clm-upgrade/migrations.lock` holds one line per shipped migration: `<id> <version> <hash>`. The hash is the SHA-256, in hex, of the concatenation, over every file in the migration's directory in byte order of its relative path, of the path, a NUL byte, the file's bytes and a NUL byte. One function computes it, for `--lock` and the test alike.

**The corpus.** `crates/clm-upgrade/corpus/v<version>.tar.gz` holds a project as the release `v<version>` set it up, for every release from `v0.14.0`, the first one published. The project is `clm init --lang rust --no-detectors --host none` output, plus `corpus/sample/`: a component with one requirement, a test file citing it, a backlog item and an archived change. Each entry is a gzipped tar, so the marker its test file holds isn't read by `check.coverage`'s scan, and its text doesn't count toward the component's size. `corpus/sample/`'s test file holds the placeholder `@REQ@` where its marker's prefix goes, and `make.sh` writes the marker. `corpus/make.sh <version> <clm>` builds an entry with the given binary in a temporary directory under version control, fails unless that binary's `clm check` passes on it, and writes the tarball deterministically: sorted names, mtime 0, owner 0, no user names. The first entries are seeded with each published release's binary (`gh release download v<x> --pattern 'clm-x86_64-unknown-linux-musl.tar.gz'`), editing the copied sample where an older release's format differs. The corpus test, which runs the built `clm`, is `clm`'s (`clm/upgrade-corpus`).

## Requirements

### Requirement: migration-list
`clm-upgrade` SHALL offer `migrations()`, the migrations the binary holds in the order they landed, each with a unique `id`, a `level`, a `done` check over the project's files, and either an `apply` or a `note`, every `required` one having an `apply`. A `note` migration MAY carry `old` and `new` file texts. `latest()` SHALL be the last one's ID.

#### Scenario: well formed
- GIVEN the binary's migrations
- WHEN `migrations()` is read
- THEN no two share an ID, each has exactly one of `apply` and `note`, none is `required` with a `note`, and `latest()` is the last one's ID

#### Scenario: a fresh project is done
- GIVEN a repository that the built `clm`'s `init` has just set up
- WHEN every migration's `done` runs on it
- THEN each returns true

### Requirement: pending-migrations
`pending(root)` SHALL return, in list order, each migration whose `done` is false and that is: an apply migration after the one `clm.toml`'s `upgraded` names; or a note migration that `declined_migrations` doesn't name. When `clm.toml` sets no `upgraded`, or doesn't parse as TOML, every apply migration counts as after it. When `upgraded` names a migration the list doesn't hold, it SHALL return `TooNew` naming the ID.

#### Scenario: stamped
- GIVEN apply migrations `a`, `b` and `c` whose `done` is false, and `clm.toml` with `upgraded = "a"`
- WHEN `pending` runs
- THEN it returns `b` and `c`

#### Scenario: already right
- GIVEN the same, with `c`'s `done` true
- WHEN `pending` runs
- THEN it returns `b`

#### Scenario: never stamped
- GIVEN apply migrations `a`, `b` and `c`, a `clm.toml` with no `upgraded`, and files for which only `b`'s `done` is false
- WHEN `pending` runs
- THEN it returns `b`

#### Scenario: a declined note
- GIVEN note migrations `n` and `m`, neither done, and `declined_migrations = ["n"]`
- WHEN `pending` runs
- THEN it returns `m`

#### Scenario: upgraded by a newer clm
- GIVEN `clm.toml` with `upgraded = "z"`, and no migration `z`
- WHEN `pending` runs
- THEN it returns `TooNew` naming `z`

### Requirement: require-current
`require_current(root)` SHALL return `Ok` when no pending migration is `required` outside the `skip_migrations` of the clone's `clm.local.toml`, as `config`'s `local_settings_path` finds it. Otherwise it SHALL return an error naming each such migration and saying to run `/clm:upgrade` or `clm upgrade`. For `TooNew` the error SHALL say the project was upgraded by a newer `clm` and name the migration.

#### Scenario: required pending
- GIVEN a pending `required` migration `r` and a pending `advised` migration `a`
- WHEN `require_current` runs
- THEN it fails, naming `r` and `clm upgrade`, and not naming `a`

#### Scenario: advised only
- GIVEN only a pending `advised` migration
- WHEN `require_current` runs
- THEN it returns `Ok`

#### Scenario: skipped from a linked worktree
- GIVEN a pending `required` migration `r`, the main worktree's `clm.local.toml` with `skip_migrations = ["r"]`, and `root` in a linked worktree of the clone
- WHEN `require_current` runs
- THEN it returns `Ok`

#### Needs
- config/local-settings-path

### Requirement: upgrade-list
`clm upgrade` with no flag SHALL print one line per pending migration in list order, `<id> <required|advised> <apply|note> <the note's first line>`, with ` skipped` appended for a required one the clone skips, or `up to date` when none is pending, and exit 0. For `TooNew` it SHALL print the refusal and exit 1. It SHALL write nothing.

#### Scenario: listing
- GIVEN pending migrations `r` (required) and `a` (advised, note "CLAUDE.md names /clm:help")
- WHEN `clm upgrade` runs
- THEN it prints `r required apply` and `a advised note CLAUDE.md names /clm:help`, in that order, exits 0, and changes no file

### Requirement: upgrade-apply
`clm upgrade --apply` SHALL, in list order, run each pending apply migration and rewrite each pending note migration's file to its `new` text when the file equals one of its `old` texts. It SHALL then set `clm.toml`'s top-level `upgraded` to `latest()`, by replacing its line or, failing that, adding it above the first table header's lead-in: the comment lines directly above that header, with no blank line between, and the blank lines directly above them. It SHALL follow an added line with a blank line when no blank line already follows it there, and change no other line of `clm.toml`. It SHALL print `applied <id>` for each migration it changed and `note <id>` for each note migration still pending after its rewrites, judging which are pending before it writes `upgraded`, so a note whose `done` reads the stamp is shown on the run that takes it; and it SHALL commit nothing. Run again, it SHALL change nothing. On `TooNew` it SHALL exit 1 and write nothing.

#### Scenario: mechanical, and an edited file
- GIVEN pending apply migration `m`, and pending note migration `n` with `old`/`new` whose file the project has edited, and a `clm.toml` ending in a `[land]` table
- WHEN `clm upgrade --apply` runs
- THEN `m`'s change is made, `upgraded` is a top-level key set to the newest migration, it prints `applied m` and `note n`, and a second run changes nothing and prints `note n`

#### Scenario: a commented header
- GIVEN a `clm.toml` with no `upgraded` that reads `host = "github"`, a blank line, `# Run before clm check.` and `[land]`
- WHEN `clm upgrade --apply` runs
- THEN it reads `host = "github"`, `upgraded = "<newest>"`, a blank line, `# Run before clm check.` and `[land]`, the comment still directly above `[land]`

#### Scenario: unedited file
- GIVEN pending note migration `n` with `old`/`new`, and its file equal to one of its `old` texts
- WHEN `clm upgrade --apply` runs
- THEN the file equals `new`, it prints `applied n`, and `clm upgrade` no longer lists `n`

#### Scenario: a note the stamp takes
- GIVEN a `clm.toml` with `upgraded = "a"`, and after `a` a note migration `s` whose `done` is true once `upgraded` names `s` or a migration after it
- WHEN `clm upgrade --apply` runs, then runs again
- THEN the first run prints `note s` and sets `upgraded` to the newest migration, and the second prints nothing and leaves `clm upgrade` printing `up to date`

#### Needs
- config/upgrade-settings

### Requirement: upgrade-note-and-decline
`clm upgrade --note <id>` SHALL print the migration's note. `clm upgrade --decline <id>` SHALL add `<id>` to `clm.toml`'s top-level `declined_migrations` when it names a pending note migration, replacing that key's lines or, when it has none, adding the key where `--apply` adds `upgraded`, and change no other line of `clm.toml`. Each SHALL exit 1, naming why and writing nothing, for an ID that names no migration, for `--decline` of a migration that isn't a pending note migration, and on `TooNew`.

#### Scenario: declining advised drift
- GIVEN a pending note migration `a`
- WHEN `clm upgrade --decline a` runs
- THEN `declined_migrations` holds `a`, and `clm upgrade` no longer lists it

#### Scenario: declining under a commented header
- GIVEN a pending note migration `a`, and a `clm.toml` with no `declined_migrations` whose first table header `[land]` has the comment `# Run before clm check.` directly above it
- WHEN `clm upgrade --decline a` runs
- THEN `declined_migrations = ["a"]` is a top-level line above that comment, and the comment is still directly above `[land]`

#### Scenario: declining a required migration
- GIVEN a pending apply migration `r`
- WHEN `clm upgrade --decline r` runs
- THEN it exits 1 naming `r`, and `clm.toml` is unchanged

#### Needs
- config/upgrade-settings

### Requirement: corpus-entry-script
`crates/clm-upgrade/corpus/make.sh <version> <clm>` SHALL write `crates/clm-upgrade/corpus/v<version>.tar.gz`: what `<clm> init --lang rust --no-detectors --host none` writes in an empty directory under version control, plus `corpus/sample/`, with the sample's `@REQ@` placeholder replaced by the marker prefix, archived deterministically. It SHALL fail, writing no entry, unless `<clm> check` passes on the result. Its output SHALL be the same bytes on every run with the same binary.

#### Scenario: the current tree
- GIVEN the built `clm`
- WHEN `make.sh 9.9.9 <clm>` runs twice, in a temporary copy of the corpus directory
- THEN `v9.9.9.tar.gz` unpacks to `clm.toml`, `ARCHITECTURE.md`, `CLAUDE.md` and the sample's files, both runs' files are identical, and it exits 0

#### Scenario: a failing check
- GIVEN a `<clm>` whose `check` exits 1
- WHEN `make.sh 9.9.9 <clm>` runs
- THEN it exits non-zero and writes no `v9.9.9.tar.gz`

### Requirement: corpus-seeded
`crates/clm-upgrade/corpus/` SHALL hold an entry for every published release from `v0.14.0` to `v0.18.0`, each written by `make.sh` with that release's own binary.

#### Scenario: the published releases
- GIVEN the corpus directory
- WHEN it is listed
- THEN it holds `v0.14.0.tar.gz`, `v0.15.0.tar.gz`, `v0.16.0.tar.gz`, `v0.17.0.tar.gz` and `v0.18.0.tar.gz`

### Requirement: shipped-migrations-frozen
For each line `<id> <version> <hash>` of `crates/clm-upgrade/migrations.lock`, `migrations()` SHALL hold `<id>`, and the hash of its directory, as **The lock** defines it, SHALL be `<hash>`. `clm upgrade --lock <version>` SHALL append one such line per migration the lock doesn't list, naming `<version>`, and change no existing line.

#### Scenario: a shipped migration edited
- GIVEN a lock line for migration `m` and a later edit to a file in `m`'s directory
- WHEN the lock is checked
- THEN the check fails, naming `m` and the version that shipped it

#### Scenario: locking a release
- GIVEN a lock listing `a`, and migrations `a` and `b`
- WHEN `clm upgrade --lock 0.3.0` runs
- THEN the lock gains one line, for `b` and naming `0.3.0`, and `a`'s line is unchanged

### Requirement: migration-carried-constraints-copy
An `advised` note migration SHALL bring an `ARCHITECTURE.md` that copies CLM's carried constraints under `## Enforced by clm`, as `clm init` wrote it up to `v0.16.0`, to the pointer at `clm guide constraints` that it writes today. Its `done` SHALL be true when `ARCHITECTURE.md` holds no `## Enforced by clm` heading, and its `old` and `new` SHALL be the two templates' texts.

#### Scenario: an unedited 0.16 file
- GIVEN an `ARCHITECTURE.md` equal to what `clm init` `v0.16.0` wrote
- WHEN `clm upgrade --apply` runs
- THEN it equals what the current `clm init` writes

#### Scenario: an edited one
- GIVEN that file with a constraint of the project's own added
- WHEN `clm upgrade --apply` runs
- THEN the file is unchanged, and it prints `note` and this migration's ID

#### Needs
- clm/upgrade-command

### Requirement: migration-claude-md-names-help
An `advised` note migration SHALL bring a `CLAUDE.md` that `clm init` wrote before it named `/clm:help` to the text it writes today. Its `done` SHALL be true when `CLAUDE.md` names `/clm:help` or doesn't exist, and its `old` and `new` SHALL be the two templates' texts.

#### Scenario: an unedited file
- GIVEN a `CLAUDE.md` equal to what `clm init` `v0.16.0` wrote
- WHEN `clm upgrade --apply` runs
- THEN it equals what the current `clm init` writes

#### Needs
- clm/upgrade-command

### Requirement: migration-check-script-from-root
An `advised` note migration SHALL bring a `scripts/clm-check.sh` that `clm init` wrote before its script ran from the repository's root, as `v0.14.0` and `v0.15.0` wrote it, to the text `v0.16.0` wrote. Its `done` SHALL be true when `scripts/clm-check.sh` doesn't exist or holds the line `cd "$(dirname "${BASH_SOURCE[0]}")/.."`. Its `old` SHALL be the script those releases' `clm init --lang rust` wrote, a bare `cargo test` after `set -euo pipefail`, and its `new` the script `clm init --lang rust` wrote from `v0.16.0` to `v0.23.0`, which `migration-starter-land-dir` then brings to the text written today, so an unedited script is rewritten and any other is left to the note. Its note SHALL say what changed for every language: the script changes to the repository's root first, and runs each language's build and tests only when that language's manifest exists.

#### Scenario: an unedited 0.14 script
- GIVEN a `scripts/clm-check.sh` equal to what `clm init --lang rust` `v0.14.0` wrote
- WHEN `clm upgrade --apply` runs
- THEN it equals what the current `clm init --lang rust` writes, and `clm upgrade` no longer lists the migration

#### Scenario: an edited one
- GIVEN that script with a line of the project's own added
- WHEN `clm upgrade --apply` runs
- THEN the script is unchanged, and it prints `note` and this migration's ID

### Requirement: migration-offer-verify
An `advised` apply migration SHALL bring a project set up before `clm verify` the commented-out `[verify]` section `clm init` writes (`init/init-verify-comment`), appending it to `clm.toml`, after one blank line, when it holds no line that is `[verify]` or `# [verify]` ignoring surrounding whitespace. Its `done` SHALL be true when `clm.toml` holds such a line or doesn't exist. Its description's first line SHALL read `Slow tests can run after the land: offer [verify]`, so `/clm:upgrade`, seeing it applied, asks the human whether to set it (`plugin.upgrade/upgrade-skill-offers-verify`). It changes no setting: the section stays commented until the human sets it.

#### Scenario: offered to an older project
- GIVEN a `clm.toml` with no `[verify]` line
- WHEN `clm upgrade` runs, then `clm upgrade --apply`
- THEN the first lists the migration as `advised apply Slow tests can run after the land: offer [verify]`, the second prints `applied` with its ID, and `clm.toml` then ends with the commented section and loads with no `verify` setting

#### Scenario: done once set or commented
- GIVEN a `clm.toml` holding the line `# [verify]`
- WHEN `clm upgrade` runs
- THEN it doesn't list the migration

#### Needs
- clm/upgrade-command
- init/init-verify-comment

#### Serves
- capability/slow-tests-after-land

### Requirement: migration-unattended-allow-rules
An `advised` note migration, ID `20261107-projects-set-up-before`, SHALL offer a project the allow rules for unattended `clm` commands that `clm init` seeds on a first run (`init/init-seeds-permissions`) and that the project lacks. It SHALL hold its own copy of those rules as that requirement lists them when this migration ships, since `upgrade` doesn't depend on `init`: for each of the `clm` subcommands `apply`, `branch`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase`, `verify` and `workaround`, the rules `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)`, twenty-two in all. Its `done` SHALL be true when the union of the `permissions.allow` arrays of `.claude/settings.json` and `.claude/settings.local.json` under the root, each when present, holds every one of those rules, read and compared as `init/init-check-permissions` reads and compares them: as exact strings, a file that isn't JSON of that shape allowing nothing; both paths are plain paths under the root, not `config`'s `local_settings_path`, as `clm init --check-permissions` reads them. It parses the JSON with the workspace's `serde_json`, which `clm-upgrade` then depends on. Its `file` SHALL be `.claude/settings.json`, and it SHALL carry no `old` or `new`, so `clm upgrade --apply` never writes the settings: they are the human's, and a rule a project removed on purpose is not put back (`unattended-steps-run-clm`). Its note's first line SHALL read `Unattended sessions need clm's allow rules in .claude/settings.json`. The note SHALL list the rules; say that `clm init --check-permissions` prints those the project lacks and that the human adds them to `.claude/settings.json` by hand; say that `clm upgrade --decline 20261107-projects-set-up-before` keeps a project that leaves rules out on purpose as it is; and say that when the root component's `paths` don't list `.claude/settings.json`, as in a project set up before `clm init` seeded the rules, a new settings file needs adding there through a spec change before `clm check` passes. The capability test `capability/upgrade-clm`'s *skipping releases* takes this note's edit as `/clm:upgrade` would have the human take it: it writes the rules to `.claude/settings.json` and adds the file to the root's `paths`, then expects `up to date`. So do the other tests whose fixture stands for an up-to-date project and that this migration would leave pending: `crates/clm-upgrade/src/shipped_tests.rs`'s `project()` and the `clm_repo` fixture of `crates/clm/tests/hooks.rs`'s `session_start_reminds_to_upgrade` give their project a `.claude/settings.json` allowing the rules.

#### Scenario: a project set up before the rules
- GIVEN a project with no `.claude/settings.json` and no `.claude/settings.local.json`
- WHEN `clm upgrade` runs, then `clm upgrade --apply`
- THEN the first lists `20261107-projects-set-up-before advised note Unattended sessions need clm's allow rules in .claude/settings.json`, the second prints `note 20261107-projects-set-up-before`, and `.claude/` still doesn't exist

#### Scenario: a project lacking only the newest rules
- GIVEN a `.claude/settings.json` that sets `enabledPlugins` and allows every rule but `Bash(clm verify)` and `Bash(clm verify *)`
- WHEN `clm upgrade --apply` runs
- THEN it prints `note 20261107-projects-set-up-before`, and `.claude/settings.json` is unchanged

#### Scenario: split across the two files
- GIVEN a `.claude/settings.json` allowing every rule but `Bash(clm land)`, and a `.claude/settings.local.json` allowing `Bash(clm land)`
- WHEN `clm upgrade` runs
- THEN it doesn't list the migration

#### Scenario: the older rule form
- GIVEN a `.claude/settings.json` allowing every rule but the two `clm land` ones, and `Bash(clm land:*)`
- WHEN `clm upgrade` runs
- THEN it lists the migration

#### Scenario: declined
- GIVEN a project lacking the rules, whose `clm.toml` sets `declined_migrations = ["20261107-projects-set-up-before"]`
- WHEN `clm upgrade` runs
- THEN it doesn't list the migration

#### Needs
- clm/upgrade-command
- init/init-seeds-permissions
- init/init-check-permissions

### Requirement: migration-tests-overlap
An `advised` note migration, ID `20261107-projects-upgrading-past-the`, SHALL tell a project set up before builds' pre-land checks stopped waiting for the clone's check lock (`land/check-lock`) that its tests must now pass while other runs of them overlap (`tests-run-concurrently`). It edits no file: its `file` SHALL be `clm.toml`, where `[land] check` names the check, and it SHALL carry no `old` or `new`. Its `done` SHALL be true when `clm.toml` doesn't exist, or parses as TOML and its `upgraded` names this migration or one after it in `migrations()`, and false otherwise, so a project `clm init` set up since this migration is done, and an older one is done once `clm upgrade --apply` has stamped it (`stamp-taken-notes`). The test fixture that stands for an up-to-date project and has a `clm.toml` with no `upgraded`, `crates/clm-upgrade/src/shipped_tests.rs`'s `project()` (with `a_declined_rules_migration_is_not_listed`'s rewritten `clm.toml`), SHALL stamp it with `latest()`. Its note's first line SHALL read `Builds' checks now overlap: your tests must pass side by side`. The note SHALL say:
- that a build's `clm land --check-only` no longer waits for other checks of the clone, so it runs at the same time as other builds' checks, the tests builds run by hand and a land's check, while lands' own checks still run one at a time;
- that the project's tests and check command must therefore pass while other runs of them overlap on one host (`tests-run-concurrently`, which `clm guide constraints` states): no fixed port, no fixed path outside the checkout and the system's temporary directory, no server or database two runs share; and that a port or path a run must know in advance, such as the port of a server the check starts, is keyed on `CLM_SLOT`, the number CLM gives each build and its checks, 0 when unset;
- how to check the project: look through the check script and the tests for literal ports and `localhost` or `127.0.0.1` addresses, absolute paths outside the checkout and the temporary directory, and servers or databases the check starts or reaches at a fixed address; then run the check in two checkouts at once with different `CLM_SLOT` values, both of which must pass; and record each collision found as a `bug` backlog item naming `tests-run-concurrently`;
- what to do meanwhile: a project that keeps its runs apart another way, such as a container per run, replaces the constraint with an entry of its own `ARCHITECTURE.md` holding `#### Replaces` naming `tests-run-concurrently` and keeping its guarantee, a change a human reviews; until its tests are fixed, `max_builds = 1` in the clone's `clm.local.toml` makes overlapping runs rarer, though not impossible;
- that taking the note edits no file and leaves nothing to decline: the upgrade's stamp marks it taken.

#### Scenario: a project set up before the lift
- GIVEN a project whose `clm.toml` sets `upgraded = "20261107-projects-set-up-before"`, every other migration done
- WHEN `clm upgrade` runs, then `clm upgrade --apply`, then `clm upgrade`
- THEN the first lists `20261107-projects-upgrading-past-the advised note Builds' checks now overlap: your tests must pass side by side`, the second prints `note 20261107-projects-upgrading-past-the` and changes no file but `clm.toml`'s `upgraded` line, and the third prints `up to date`

#### Scenario: never stamped
- GIVEN a project whose `clm.toml` sets no `upgraded`
- WHEN `clm upgrade` runs
- THEN it lists the migration

#### Scenario: the note
- GIVEN the migration
- WHEN `clm upgrade --note 20261107-projects-upgrading-past-the` runs on a project it is pending in
- THEN the note names `CLM_SLOT`, `tests-run-concurrently`, `clm guide constraints`, `#### Replaces`, `max_builds` and `bug`

#### Needs
- land/check-lock
- land/check-slot
- orchestrator.run/prompt-slot
- clm/upgrade-command

### Requirement: migration-land-dir
An `advised` note migration, ID `20261108-toolchain-neutral-land-builds`, SHALL tell a project set up before it that lands now hand its check a build place of its own and check only where a branch and `main` overlap. It edits no file: its `file` SHALL be `clm.toml`, where `[land] check` names the check, and it SHALL carry no `old` or `new`. Its `done` SHALL be true when `clm.toml` doesn't exist, or parses as TOML and its `upgraded` names this migration or one after it in `migrations()`, and false otherwise, as `migration-tests-overlap`'s is. The fixture `migration-tests-overlap` names for an up-to-date project SHALL stay stamped with `latest()`. Its note's first line SHALL read `Lands hand your check CLM_LAND_DIR and re-check only the overlap`. The note SHALL say:
- that `clm land` sets `CLM_LAND_DIR` for a check it runs in the clone's land tree, a place that lasts from land to land, and that a project whose check builds should point its toolchain's output there in its own check script (for cargo, `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL=0`), since a later `clm` stops setting the cargo variables itself;
- that a land whose branch's head was checked whole by `clm land --check-only` now runs only the tests both the branch and what `main` gained since can affect, and none when there are none, so its verdict rests on the project's tests declaring what they read (`land-checks-stay-fast`, which `clm guide constraints` states): an owner and `depends_on`, `req:` markers, and `clm:reads` markers for the files they read;
- that tests that are slow, flaky or need a resource no other run may share belong in `[verify]`, after the land, and that a failure there is reported with how the culprit's land re-checked, so the project can judge whether a fact, the code or the test was wrong;
- that taking the note edits no file and leaves nothing to decline: the upgrade's stamp marks it taken.

#### Scenario: a project set up before
- GIVEN a project whose `clm.toml` sets `upgraded` to the migration before this one, every other migration done
- WHEN `clm upgrade` runs, then `clm upgrade --apply`, then `clm upgrade`
- THEN the first lists `20261108-toolchain-neutral-land-builds advised note Lands hand your check CLM_LAND_DIR and re-check only the overlap`, the second prints `note 20261108-toolchain-neutral-land-builds` and changes no file but `clm.toml`'s `upgraded` line, and the third prints `up to date`

#### Scenario: the note
- GIVEN the migration
- WHEN `clm upgrade --note 20261108-toolchain-neutral-land-builds` runs on a project it is pending in
- THEN the note names `CLM_LAND_DIR`, `CARGO_TARGET_DIR`, `land-checks-stay-fast`, `clm guide constraints`, `clm:reads` and `[verify]`

#### Needs
- land/land-dir-env
- land/overlap-recheck
- clm/upgrade-command

### Requirement: migration-build-setup
An `advised` note migration, ID `20261109-a-build-s-tests`, SHALL tell a project set up before it that a build's tests must run against its own worktree's code and dependencies, and how to declare the setup that makes them. It edits no file: its `file` SHALL be `clm.toml`, where `[build] setup` goes, and it SHALL carry no `old` or `new`. Its `done` SHALL be true when `clm.toml` doesn't exist, or parses as TOML and its `upgraded` names this migration or one after it in `migrations()`, and false otherwise, as `migration-tests-overlap`'s is. The fixture `migration-tests-overlap` names for an up-to-date project SHALL stay stamped with `latest()`. A test that expects a project stamped before an earlier note to list only that note, such as `migration-land-dir`'s, SHALL expect this one beside it, and `migration-starter-land-dir` (`20261109-land-still-sets-cargo`) too when the fixture holds a `scripts/clm-check.sh` that runs `cargo` and names no `CLM_LAND_DIR`. Its note's first line SHALL read `Builds run your tests in a fresh worktree: declare [build] setup`. The note SHALL say:
- that each build runs in a fresh worktree inside the main checkout, with nothing installed, so a toolchain whose lookup walks up the parent directories, or an environment the checkouts share, can resolve the main checkout's packages or source instead of the branch's, and a build's red or green test then means nothing (`tests-run-in-their-own-tree`, which `clm guide constraints` states);
- that the project declares in `clm.toml` `[build] setup`, a shell command that makes a fresh worktree ready to run its tests, such as one installing its dependencies into it, and that `clm setup` runs it, which builds and drafts do before their first test run, while `[land] check` stays a command that needs no setup before it;
- how to check the project: create a worktree under the main checkout, run its tests there without installing anything, and see whether they fail for want of their dependencies or resolve them from outside the worktree; for example, a project whose tests use npm packages can set `setup = "test ! -f package.json || npm ci"`;
- that taking the note edits no file and leaves nothing to decline: the upgrade's stamp marks it taken.

#### Scenario: a project set up before
- GIVEN a project whose `clm.toml` sets `upgraded` to the migration before this one, every other migration done (its fixture holds no `scripts/clm-check.sh`, so `migration-starter-land-dir` is done)
- WHEN `clm upgrade` runs, then `clm upgrade --apply`, then `clm upgrade`
- THEN the first lists `20261109-a-build-s-tests advised note Builds run your tests in a fresh worktree: declare [build] setup`, the second prints `note 20261109-a-build-s-tests` and changes no file but `clm.toml`'s `upgraded` line, and the third prints `up to date`

#### Scenario: the note
- GIVEN the migration
- WHEN `clm upgrade --note 20261109-a-build-s-tests` runs on a project it is pending in
- THEN the note names `tests-run-in-their-own-tree`, `clm guide constraints`, `[build] setup`, `clm setup` and `[land] check`

#### Needs
- clm/upgrade-command
- config/build-setup-setting
- orchestrator.run/setup-command

#### Serves
- architecture/tests-run-in-their-own-tree

### Requirement: migration-starter-land-dir
An `advised` note migration, ID `20261109-land-still-sets-cargo`, SHALL bring a `scripts/clm-check.sh` that `clm init --lang rust` wrote from `v0.16.0` to `v0.23.0` to the text it writes since `init/starter-check-land-dir`, since `clm land` no longer sets cargo's variables for a project's check (`land/shared-land-target`). Its `file` SHALL be `scripts/clm-check.sh`, its `old` the script those releases' `clm init --lang rust` wrote (one text, the same in each), and its `new` the script the current `clm init --lang rust` writes, so an unedited script is rewritten and any other is left to the note. Its `done` SHALL be true when `scripts/clm-check.sh` doesn't exist, holds `CLM_LAND_DIR`, or doesn't hold `cargo`, and false otherwise. Its note's first line SHALL read `Lands no longer set CARGO_TARGET_DIR: point cargo at CLM_LAND_DIR in your check`. The note SHALL say:
- that `clm land` no longer sets `CARGO_TARGET_DIR` or `CARGO_INCREMENTAL` for `[land] check`, and hands it `CLM_LAND_DIR` instead, the clone's build place that lasts from land to land, while `clm land --check-only` hands none, so a build's check compiles in its own worktree;
- that a check script that runs cargo should, before its first cargo command, export `CARGO_TARGET_DIR` from `CLM_LAND_DIR` and `CARGO_INCREMENTAL=0` when `CLM_LAND_DIR` is set and non-empty, giving the lines `clm init` writes for it;
- that without them a land's cargo build goes to the land tree's own `target/`, which the land cleans before every check, so every land compiles from scratch;
- that a project whose check keeps its build output elsewhere on purpose declines the note with `clm upgrade --decline 20261109-land-still-sets-cargo`.

#### Scenario: an unedited 0.23 script
- GIVEN a `scripts/clm-check.sh` equal to what `clm init --lang rust` `v0.23.0` wrote
- WHEN `clm upgrade --apply` runs
- THEN it equals what the current `clm init --lang rust` writes, and `clm upgrade` no longer lists the migration

#### Scenario: an unedited 0.14 script
- GIVEN a `scripts/clm-check.sh` equal to what `clm init --lang rust` `v0.14.0` wrote
- WHEN `clm upgrade --apply` runs
- THEN it equals what the current `clm init --lang rust` writes, after `migration-check-script-from-root`'s rewrite and this one's

#### Scenario: an edited one
- GIVEN the `v0.23.0` script with a line of the project's own added
- WHEN `clm upgrade --apply` runs
- THEN the script is unchanged, and it prints `note` and this migration's ID

#### Scenario: a script that runs no cargo
- GIVEN a `scripts/clm-check.sh` that `clm init --lang typescript` wrote
- WHEN `clm upgrade` runs
- THEN it doesn't list the migration

#### Scenario: the note
- GIVEN the migration
- WHEN `clm upgrade --note 20261109-land-still-sets-cargo` runs on a project it is pending in
- THEN the note names `CLM_LAND_DIR`, `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL` and `--decline`

#### Needs
- clm/upgrade-command
- init/starter-check-land-dir

### Requirement: superseded-notes
A migration SHALL be able to name the earlier note migrations it supersedes, and `pending(root)` SHALL leave out a note migration that a migration whose `done` is true supersedes, since its edit is to a text the later migration has moved. A shipped migration's own files stay as they shipped (`shipped-migrations-frozen`).

#### Scenario: a superseded note
- GIVEN note migration `n` not done, and a later migration `m` that supersedes `n` and is done
- WHEN `pending` runs
- THEN it doesn't return `n`

#### Scenario: not yet superseded
- GIVEN the same, with `m` not done
- WHEN `pending` runs
- THEN it returns `n`

### Requirement: migration-agents-md
An `advised` `apply` migration SHALL move a project's instructions to `AGENTS.md`, which every supported agent harness reads (`platform`). Its `done` SHALL be true when `CLAUDE.md` doesn't exist or holds the line `@AGENTS.md`. Its `apply` SHALL, when `AGENTS.md` is missing, write to `AGENTS.md` the text the current `clm init` writes there when `CLAUDE.md`'s text equals one a released `clm init` wrote, and `CLAUDE.md`'s text unchanged otherwise, and replace `CLAUDE.md` with the one line `@AGENTS.md`, so an unedited project ends as `clm init` writes it today (`clm/upgrade-corpus`) and an edited one keeps its edits; and, when both files exist, add the line `@AGENTS.md` as `CLAUDE.md`'s first line, followed by a blank line, changing nothing else, so no instruction is lost or moved under the human's feet. It SHALL supersede `20261105-clm-upgrade-2`, whose `done` reads `CLAUDE.md` for `/clm:help`, which the moved text no longer holds there.

#### Scenario: CLAUDE.md alone
- GIVEN a project whose edited `CLAUDE.md` holds the instructions and no `AGENTS.md`
- WHEN `clm upgrade --apply` runs
- THEN `AGENTS.md` holds that text and `CLAUDE.md` is `@AGENTS.md`

#### Scenario: an unedited CLAUDE.md
- GIVEN a project whose `CLAUDE.md` equals what `clm init` `v0.16.0` wrote, and no `AGENTS.md`
- WHEN `clm upgrade --apply` runs
- THEN `AGENTS.md` and `CLAUDE.md` equal what the current `clm init` writes

#### Scenario: both files
- GIVEN a project with its own `AGENTS.md` and a `CLAUDE.md` that doesn't import it
- WHEN `clm upgrade --apply` runs
- THEN `AGENTS.md` is unchanged and `CLAUDE.md` starts with `@AGENTS.md` followed by its old text

#### Scenario: already moved
- GIVEN a project whose `CLAUDE.md` is `@AGENTS.md`
- WHEN `clm upgrade` runs
- THEN the migration isn't pending

#### Needs
- clm/upgrade-command
- init/init-files
- init/claude-md-names-the-skills

#### Serves
- capability/upgrade-clm
- capability/work-from-any-agent-harness

### Requirement: migration-setup-allow-rules
An `advised` note migration, ID `20261112-a-draft-s-clm`, SHALL offer a project the two allow rules for `clm setup` that `init/init-seeds-permissions` adds to `clm init`'s seeding, `Bash(clm setup)` and `Bash(clm setup *)`, when the project lacks either. It SHALL hold its own copy of the two rules, since `upgrade` doesn't depend on `init`, and `20261107-projects-set-up-before`, which is frozen once shipped (`shipped-migrations-frozen`), keeps offering the other twenty-two. Its `done` SHALL be true when the union of the `permissions.allow` arrays of `.claude/settings.json` and `.claude/settings.local.json` under the root, each when present, holds both rules, read and compared as `migration-unattended-allow-rules` reads and compares them. Its `file` SHALL be `.claude/settings.json`, and it SHALL carry no `old` or `new`, so `clm upgrade --apply` never writes the settings (`unattended-steps-run-clm`, `allow-rules-offered-by-note`). Its note's first line SHALL read `Unattended sessions need clm's allow rules for clm setup in .claude/settings.json`. The note SHALL list the two rules; say that builds and drafts run `clm setup` before their first test run on a code branch (`tests-run-in-their-own-tree`) and that Claude Code's auto mode denies it when no rule allows it; say that the human adds them to `.claude/settings.json` by hand and that `clm init --check-permissions` prints the rules the project lacks; and say that `clm upgrade --decline 20261112-a-draft-s-clm` keeps a project that leaves them out on purpose as it is. `init/init-seeds-permissions`' build writes this migration (`template-change-ships-its-migration`), and gives the fixtures that stand for an up-to-date project and that this migration would leave pending the two rules: `crates/clm-upgrade/src/shipped_tests.rs`'s `project()`, the `clm_repo` fixture of `crates/clm/tests/hooks.rs`'s `session_start_reminds_to_upgrade`, and the capability test `capability/upgrade-clm`'s *skipping releases*, as `migration-unattended-allow-rules` states for its own rules.

#### Scenario: a project lacking the setup rules
- GIVEN a `.claude/settings.json` that sets `enabledPlugins` and allows every rule `20261107-projects-set-up-before` offers but neither `clm setup` rule
- WHEN `clm upgrade` runs, then `clm upgrade --apply`
- THEN the first lists `20261112-a-draft-s-clm advised note Unattended sessions need clm's allow rules for clm setup in .claude/settings.json` and doesn't list `20261107-projects-set-up-before`, the second prints `note 20261112-a-draft-s-clm`, and `.claude/settings.json` is unchanged

#### Scenario: split across the two files
- GIVEN a `.claude/settings.json` allowing `Bash(clm setup)`, and a `.claude/settings.local.json` allowing `Bash(clm setup *)`
- WHEN `clm upgrade` runs
- THEN it doesn't list `20261112-a-draft-s-clm`

#### Scenario: declined
- GIVEN a project lacking the setup rules, whose `clm.toml` sets `declined_migrations = ["20261112-a-draft-s-clm"]`
- WHEN `clm upgrade` runs
- THEN it doesn't list `20261112-a-draft-s-clm`

#### Needs
- init/init-seeds-permissions
