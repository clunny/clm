---
component: upgrade
base: { contract: "bd3fb6bd34076b66", design: "9d4db9c52e63ddee" }
---

## MODIFIED Contract
Bringing a project set up by an older `clm` up to date with the one that runs (`upgrade-migrations`). Crate `clm-upgrade`:

- `migrations()`: the crate's own migrations, in order. The list is closed: it holds the migrations written before `20261114-migrations-with-templates`, and every later one is `init`'s (`init/all-migrations`), which gives its callers the whole list, this one followed by its own. Every call below works over the list its caller gives it, with the pairs that say which migration supersedes which note, and never reads a list of its own.
- A migration: each has an `id`, a `level` (`Required` or `Advised`) and a `done(root) -> bool`, and is one of:
  - an *apply* migration, with `apply(root)`. Every `required` migration is one;
  - a *note* migration, always `advised`, with a `note` (Markdown) and, optionally, `old` (one or more texts) and `new`, the exact texts of a file `clm init` wrote before and writes since.
- `latest_in(list)`: the last migration's ID, which `init` writes as `upgraded` in a new `clm.toml`.
- `pending_in(list, supersedes, root)`: the migrations a project still needs, or `TooNew(id)` when its `upgraded` names a migration this binary doesn't hold.
- `require_current_in(list, supersedes, root)`: `Ok` when no `required` migration is pending outside the clone's `skip_migrations`; otherwise an error naming each such migration and saying to run `/clm:upgrade` or `clm upgrade`, or, for `TooNew`, saying the project needs a newer `clm`. `crates/clm` calls it before every command not on its allow-list.
- `list_in`, `apply_in`, `note_in` and `decline_in`, each taking the list and the pairs first: what `clm upgrade [--apply | --note <id> | --decline <id>]` does, dispatched by `crates/clm` with `init`'s list.
- `check_lock(crate_dir, ids)` and `append_lock(crate_dir, version, ids)`: the lock of the migrations under one crate's `src/migrations/`, checked and appended to, for this crate's and for `init`'s (`clm upgrade --lock <version>`).

It reads `clm.toml`'s `upgraded` and `declined_migrations` itself, as plain TOML, since the settings a migration fixes may be ones `config`'s loader rejects. It reads `clm.local.toml`'s `skip_migrations` from the file `config`'s `local_settings_path` names, also as plain TOML. It writes the working tree only and commits nothing.

## MODIFIED Design
**Migrations.** Each migration is a directory `src/migrations/<id>/`, holding `mod.rs` and any text it embeds with `include_str!` (its `old` and `new`), pulled in with `#[path]`, since an ID such as `20261105-clm-upgrade` isn't a Rust identifier. `src/migrations.rs` lists them in the order they landed on `main`, not by their IDs' dates. The list is closed (`own-list-closed`, `migrations-live-with-init`): a migration written since lives in `init`, in a directory of the same shape under `crates/clm-init/src/migrations/`, and the whole list is this one followed by `init`'s in the order they landed, so no migration is ever added before one a project's `upgraded` may name. The engine (`pending_in`, `require_current_in`, `list_in`, `apply_in`, `note_in`, `decline_in`, `latest_in`) takes the list and the supersedes pairs from its caller, since `init` depends on this crate and not the other way round. An ID is the ID of the change that needed the migration, suffixed `-2`, `-3` for a change that needs more than one. `done` reads only files under `root`, so it is cheap enough to run on every command for the migrations it's asked about. An apply migration exists only for what CLM can rewrite exactly (a key rename, a `.gitignore` line). What CLM must read correctly it can rewrite exactly, which is why every `required` migration is an apply migration and a note never blocks. A note migration's `old`/`new` texts are copies of `clm-init`'s templates as of the releases they name, embedded here, since `upgrade` doesn't depend on `init`.

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

**The lock.** A crate that holds migrations has a `migrations.lock` beside its `Cargo.toml`: this crate's, and `init`'s once it ships one, a missing file reading as empty. It holds one line per shipped migration: `<id> <version> <hash>`. The hash is the SHA-256, in hex, of the concatenation, over every file in the migration's directory in byte order of its relative path, of the path, a NUL byte, the file's bytes and a NUL byte. One function computes it, for `--lock` and the test alike.

**The corpus.** `crates/clm-upgrade/corpus/v<version>.tar.gz` holds a project as the release `v<version>` set it up, for every release from `v0.14.0`, the first one published. The project is `clm init --lang rust --no-detectors --host none` output, plus `corpus/sample/`: a component with one requirement, a test file citing it, a backlog item and an archived change. Each entry is a gzipped tar, so the marker its test file holds isn't read by `check.coverage`'s scan, and its text doesn't count toward the component's size. `corpus/sample/`'s test file holds the placeholder `@REQ@` where its marker's prefix goes, and `make.sh` writes the marker. `corpus/make.sh <version> <clm>` builds an entry with the given binary in a temporary directory under version control, fails unless that binary's `clm check` passes on it, and writes the tarball deterministically: sorted names, mtime 0, owner 0, no user names. The first entries are seeded with each published release's binary (`gh release download v<x> --pattern 'clm-x86_64-unknown-linux-musl.tar.gz'`), editing the copied sample where an older release's format differs. The corpus test, which runs the built `clm`, is `clm`'s (`clm/upgrade-corpus`).

## ADDED Requirements

### Requirement: own-list-closed
`migrations()` SHALL hold exactly these thirteen migrations, in this order, and no other: `20261105-clm-upgrade`, `20261105-clm-upgrade-2`, `20261106-corpus-entries-drift-from`, `20261107-projects-set-up-before`, `20261107-slow-tests-after-land`, `20261107-projects-upgrading-past-the`, `20261108-toolchain-neutral-land-builds`, `20261109-land-still-sets-cargo`, `20261109-a-build-s-tests`, `20261112-agents-md`, `20261112-a-draft-s-clm`, `20261112-build-loop-in-clm` and `20261114-proof-is-real`. `crates/clm-upgrade/src/migrations/` SHALL hold one directory for each and no other. A migration written since is `init`'s (`init/all-migrations`), so the build that changes what a project holds writes the migration in its own component (`migrations-live-with-init`).

#### Scenario: the list as it stands
- GIVEN the crate
- WHEN `migrations()` is read and `src/migrations/` is listed
- THEN both hold those thirteen IDs, the list in that order

#### Scenario: a directory added
- GIVEN a tree holding one more directory under `crates/clm-upgrade/src/migrations/`
- WHEN the test's check runs on it
- THEN it fails, naming the directory

### Requirement: forms-without-a-list-gone
`clm-upgrade` SHALL offer no call that reads the crate's own list in place of one its caller gives: no `pending(root)`, `require_current(root)`, `list(root)`, `apply(root)`, `note(root, id)`, `decline(root, id)` or `latest()`. Until this is built those forms stay, each calling its `_in` form with `migrations()` and the crate's own pairs, marked `clm:remove-by 20261114-migrations-with-templates`.

#### Scenario: no caller is left
- GIVEN the workspace
- WHEN `crates/clm-upgrade/src/` is read for public functions, and the other crates for calls of `clm_upgrade::`
- THEN the crate exports none of those seven forms, no crate calls one, and no `clm:remove-by 20261114-migrations-with-templates` marker is left

#### Needs
- init/all-migrations
- init/init-stamps-upgraded
- clm/upgrade-command
- clm/upgrade-blocks-commands
- plugin.hooks/session-upgrade-reminder

## MODIFIED Requirements

### Requirement: migration-list <!-- base: 067d119c57346541 -->
`clm-upgrade` SHALL offer `migrations()`, its own migrations in the order they landed, each with a unique `id`, a `level`, a `done` check over the project's files, and either an `apply` or a `note`, every `required` one having an `apply`. A `note` migration MAY carry `old` and `new` file texts. `latest_in(list)` SHALL be the ID of the last migration of the list it is given, and nothing for an empty list. A `Migration` SHALL be a value a caller can copy into a list of its own, so `init` can offer this list followed by its own (`init/all-migrations`).

#### Scenario: well formed
- GIVEN the binary's migrations
- WHEN `migrations()` is read
- THEN no two share an ID, each has exactly one of `apply` and `note`, none is `required` with a `note`, and `latest_in` of it is the last one's ID

#### Scenario: a fresh project is done
- GIVEN a repository that the built `clm`'s `init` has just set up
- WHEN every migration's `done` runs on it
- THEN each returns true

### Requirement: pending-migrations <!-- base: dbcfa1768d213b3e -->
`pending_in(list, supersedes, root)` SHALL return, in the order of the list it is given, each migration whose `done` is false and that is: an apply migration after the one `clm.toml`'s `upgraded` names; or a note migration that `declined_migrations` doesn't name. When `clm.toml` sets no `upgraded`, or doesn't parse as TOML, every apply migration counts as after it. When `upgraded` names a migration the list doesn't hold, it SHALL return `TooNew` naming the ID. It SHALL read no list but the one it is given: a migration that list holds and the crate's own doesn't is judged like any other.

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

#### Scenario: a migration the crate doesn't hold
- GIVEN a list of the crate's own migrations followed by one more apply migration `z` whose `done` is false, and a project whose `upgraded` names the last of the crate's own
- WHEN `pending_in` runs with that list, and again with the crate's own list alone
- THEN the first returns `z`, and the second returns nothing

### Requirement: require-current <!-- base: d9893e2201390410 -->
`require_current_in(list, supersedes, root)` SHALL return `Ok` when no pending migration is `required` outside the `skip_migrations` of the clone's `clm.local.toml`, as `config`'s `local_settings_path` finds it. Otherwise it SHALL return an error naming each such migration and saying to run `/clm:upgrade` or `clm upgrade`. For `TooNew` the error SHALL say the project was upgraded by a newer `clm` and name the migration.

#### Scenario: required pending
- GIVEN a pending `required` migration `r` and a pending `advised` migration `a`
- WHEN `require_current_in` runs with that list
- THEN it fails, naming `r` and `clm upgrade`, and not naming `a`

#### Scenario: advised only
- GIVEN only a pending `advised` migration
- WHEN `require_current_in` runs with that list
- THEN it returns `Ok`

#### Scenario: skipped from a linked worktree
- GIVEN a pending `required` migration `r`, the main worktree's `clm.local.toml` with `skip_migrations = ["r"]`, and `root` in a linked worktree of the clone
- WHEN `require_current_in` runs with that list
- THEN it returns `Ok`

#### Needs
- config/local-settings-path

### Requirement: upgrade-apply <!-- base: cfe8e2bff5413dbf -->
`clm upgrade --apply`, which is `apply_in(list, supersedes, root)` with the list its caller gives, SHALL, in list order, run each pending apply migration and rewrite each pending note migration's file to its `new` text when the file equals one of its `old` texts. It SHALL then set `clm.toml`'s top-level `upgraded` to `latest_in` of that list, by replacing its line or, failing that, adding it above the first table header's lead-in: the comment lines directly above that header, with no blank line between, and the blank lines directly above them. It SHALL follow an added line with a blank line when no blank line already follows it there, and change no other line of `clm.toml`. It SHALL print `applied <id>` for each migration it changed and `note <id>` for each note migration still pending after its rewrites, judging which are pending before it writes `upgraded`, so a note whose `done` reads the stamp is shown on the run that takes it; and it SHALL commit nothing. Run again, it SHALL change nothing. On `TooNew` it SHALL exit 1 and write nothing.

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

### Requirement: superseded-notes <!-- base: 4161b9aebef6419a -->
A migration SHALL be able to name the earlier note migrations it supersedes, as a pair of IDs among the `supersedes` pairs the caller gives, and `pending_in` SHALL leave out a note migration that a migration whose `done` is true supersedes, since its edit is to a text the later migration has moved. A shipped migration's own files stay as they shipped (`shipped-migrations-frozen`).

#### Scenario: a superseded note
- GIVEN note migration `n` not done, and a later migration `m` that supersedes `n` and is done
- WHEN `pending` runs
- THEN it doesn't return `n`

#### Scenario: not yet superseded
- GIVEN the same, with `m` not done
- WHEN `pending` runs
- THEN it returns `n`

### Requirement: shipped-migrations-frozen <!-- base: b7c058406dd3eb93 -->
For each line `<id> <version> <hash>` of `crates/clm-upgrade/migrations.lock`, `migrations()` SHALL hold `<id>`, and the hash of its directory, as **The lock** defines it, SHALL be `<hash>`. `clm upgrade --lock <version>` SHALL append one such line per migration the lock doesn't list, naming `<version>`, and change no existing line. The crate SHALL offer both as public calls over any crate's directory, `check_lock(crate_dir, ids)`, which returns what differs, and `append_lock(crate_dir, version, ids)`, reading `<crate_dir>/migrations.lock`, a missing file as empty, and `<crate_dir>/src/migrations/<id>/`, so `init` holds its own migrations to the same rule (`init/own-migrations-frozen`).

#### Scenario: a shipped migration edited
- GIVEN a lock line for migration `m` and a later edit to a file in `m`'s directory
- WHEN the lock is checked
- THEN the check fails, naming `m` and the version that shipped it

#### Scenario: locking a release
- GIVEN a lock listing `a`, and migrations `a` and `b`
- WHEN `clm upgrade --lock 0.3.0` runs
- THEN the lock gains one line, for `b` and naming `0.3.0`, and `a`'s line is unchanged
