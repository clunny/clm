---
component: upgrade
base: { contract: "d73ae844d1ba84e4", design: "1009b4acce800b62" }
---

## MODIFIED Contract
Bringing a project set up by an older `clm` up to date with the one that runs (`upgrade-migrations`). Crate `clm-upgrade`:

- `migrations()`: the crate's own migrations, in order. The list is closed: it holds the migrations written before `20261114-migrations-with-templates`, and every later one is `init`'s (`init/all-migrations`), which gives its callers the whole list, this one followed by its own. Every call below works over the list its caller gives it, with the pairs that say which migration supersedes which note, and never reads a list of its own.
- `supersedes()`: the crate's own pairs, each `(migration, note it supersedes)` (`superseded-notes`), which `init` gives its callers followed by its own.
- `Migration { id, level, done, kind }`, a value a caller copies into a list of its own and builds by literal: `id`, the migration's ID; `level`, a `Level`, `Required` or `Advised`; `done(root) -> bool`, whether the project under `root` already has what the migration brings; and `kind`, a `Kind`, one of:
  - `Kind::Apply(apply, description)`, an *apply* migration: `apply(root)` returns `Ok(())`, or the reason it failed, which `--apply` reports (`upgrade-apply`), and `clm upgrade` lists the first line of `description`, empty for none. Every `Required` migration is one;
  - `Kind::Note(Note { text, file, old, new })`, a *note* migration, always `Advised`: `text` is the note, in Markdown, whose first line `clm upgrade` lists and which `clm upgrade --note` prints; `old` (any number of texts) and `new` (one, or none) are the exact texts `clm init` wrote before and writes since of the file `file` names under the root, which `--apply` rewrites from an `old` text to `new`.
- `latest_in(list) -> Option<&str>`: the last migration's ID, which `init` writes as `upgraded` in a new `clm.toml`; none for an empty list.
- `pending_in(list, supersedes, root) -> Result<Vec<&Migration>, TooNew>`: the migrations a project still needs, in list order, or `TooNew(id)` when its `upgraded` names a migration the list doesn't hold.
- `require_current_in(list, supersedes, root) -> Result<(), String>`: `Ok` when no `required` migration is pending outside the clone's `skip_migrations`; otherwise an error naming each such migration and saying to run `/clm:upgrade` or `clm upgrade`, or, for `TooNew`, saying the project needs a newer `clm`. The root calls it before every command not on its allow-list (`clm/upgrade-blocks-commands`).
- `list_in(list, supersedes, root)`, `apply_in(list, supersedes, root)`, `note_in(list, supersedes, root, id)` and `decline_in(list, supersedes, root, id)`: what `clm upgrade`, `clm upgrade --apply`, `clm upgrade --note <id>` and `clm upgrade --decline <id>` do, which the root dispatches with `init`'s list (`clm/upgrade-command`). Each returns `Ok` with the text the command prints on standard output, exiting 0, or `Err` with the refusal, which the root prints on standard error, exiting 1.
- `check_lock(crate_dir, ids) -> Result<(), String>` and `append_lock(crate_dir, version, ids) -> Result<Vec<String>, String>`: the lock of the migrations under one crate's `src/migrations/`, checked against the IDs the crate holds, and appended to, returning the IDs it locked. `clm upgrade --lock <version>` runs `append_lock` for this crate and for `init`'s (`clm/upgrade-command`), and `init` holds its own migrations to `check_lock` (`init/own-migrations-frozen`).

It reads `clm.toml`'s `upgraded` and `declined_migrations` itself, as plain TOML, since the settings a migration fixes may be ones `config`'s loader rejects. It reads `clm.local.toml`'s `skip_migrations` from the file `config`'s `local_settings_path` names, also as plain TOML. It writes the working tree only and commits nothing.

## MODIFIED Design
**Migrations.** Each migration is a directory `src/migrations/<id>/`, holding `mod.rs` and any text it embeds with `include_str!` (its `old` and `new`), pulled in with `#[path]`, since an ID such as `20261105-clm-upgrade` isn't a Rust identifier. `src/migrations.rs` lists them in the order they landed on `main`, not by their IDs' dates. The list is closed (`own-list-closed`, `migrations-live-with-init`): a migration written since lives in `init`, in a directory of the same shape under `crates/clm-init/src/migrations/`, and the whole list is this one followed by `init`'s in the order they landed, so no migration is ever added before one a project's `upgraded` may name. The engine (`pending_in`, `require_current_in`, `list_in`, `apply_in`, `note_in`, `decline_in`, `latest_in`) takes the list and the supersedes pairs from its caller, since `init` depends on this crate and not the other way round. An ID is the ID of the change that needed the migration, suffixed `-2`, `-3` for a change that needs more than one. `done` reads only files under `root`, so it is cheap enough to run on every command for the migrations it's asked about. An apply migration exists only for what CLM can rewrite exactly (a key rename, a `.gitignore` line). What CLM must read correctly it can rewrite exactly, which is why every `required` migration is an apply migration and a note never blocks. A note migration's `old`/`new` texts are copies of `clm-init`'s templates as of the releases they name, embedded here, since `upgrade` doesn't depend on `init`; so is the list of allow rules a rules note offers.

**Pending.** For a `clm.toml` with `upgraded = "<id>"`:
- an apply migration is pending when it comes after `<id>` in the list and its `done` is false;
- a note migration is pending when its `done` is false and `declined_migrations` doesn't name it, wherever it sits in the list.

A `clm.toml` with no `upgraded`, or one that doesn't parse as TOML, has every migration whose `done` is false and that isn't declined pending. So the block lifts as soon as the files are right, whatever the stamp says. The stamp lets `done` skip what the project already took, and tells a binary that the project is too new for it. A note that asks the project to read it and edit nothing has a `done` that reads the stamp: true once `upgraded` names that note or a migration after it in the list, so a project `clm init` set up since never sees it, and a project upgraded past it has taken it (`stamp-taken-notes`).

**`clm upgrade`.**
- *No flag.* One line per pending migration, in list order: `<id> <required|advised> <apply|note> <the note's first line>`, with `skipped` appended for a required one in `skip_migrations`; or `up to date`.
- *`--apply`.* Reads `clm.toml` as TOML first. Runs, in order, each pending apply migration, and rewrites a pending note migration's file to `new` when the file equals one of its `old` texts. It then sets `upgraded` to `latest_in` of the list it was given, and prints `applied <id>` per migration it changed and `note <id>` per note migration still pending after its rewrites, judged before it writes `upgraded`, so a note the stamp takes is still shown on the run that takes it. It writes `upgraded` by replacing its line, the one that sets the top-level key however the key is written, bare or quoted, with the further lines of a value TOML reads as spanning several, and never past the end of that value, so a bracket in a comment or a string ends nothing; or else as a new line above the first table header's lead-in (at the end when there is none), so it stays a top-level key. Every other line is left as it is. It never stops early: a migration that fails is gathered with the others' results, and the run stamps nothing. A run with nothing to change writes nothing.
- *`--note <id>`.* Prints the migration's note.
- *`--decline <id>`.* Adds a pending note migration's ID to `declined_migrations`, the same way `upgraded` is written. The session-start reminder then stops naming it.
- *`--lock <version>`.* Appends to each crate's `migrations.lock`, this one's and `init`'s, one line per migration of that crate the lock doesn't list (**The lock**). Only the release runs it (`ci`).

**A new top-level line.** A header's *lead-in* is the run of comment lines directly above it, with no blank line between, and the blank lines directly above that run (only the blank lines when no comment is there). A new top-level line goes above the first table header's lead-in, so a comment that describes a header stays next to it, and is followed by a blank line when no blank line already follows it there. A comment run that reaches the top of the file is part of the lead-in too, so the line then goes first: a file comment written with no blank line before the first header can't be told from that header's own. One function writes it for `upgraded` and `declined_migrations` alike.

**Skips.** `skip_migrations`, only in `clm.local.toml` (`config`), lists required migrations that `require_current_in` ignores on this clone, for a migration that is itself broken. The human reports it with `clm defect` and skips it until a fixed `clm` ships a fixed migration under a new ID. A skip writes nothing to `clm.toml`.

**The lock.** A crate that holds migrations has a `migrations.lock` beside its `Cargo.toml`: this crate's, and `init`'s once it ships one, a missing file reading as empty. It holds one line per shipped migration: `<id> <version> <hash>`. The hash is the SHA-256, in hex, of the concatenation, over every file in the migration's directory in byte order of its relative path, of the path, a NUL byte, the file's bytes and a NUL byte. One function computes it, for `--lock` and the test alike.

**The corpus.** `crates/clm-upgrade/corpus/v<version>.tar.gz` holds a project as the release `v<version>` set it up, for every release from `v0.14.0`, the first one published. The project is `clm init --lang rust --no-detectors --host none` output, plus `corpus/sample/`: a component with one requirement, a test file citing it, a backlog item and an archived change. Each entry is a gzipped tar, so the marker its test file holds isn't read by `check.coverage`'s scan, and its text doesn't count toward the component's size. `corpus/sample/`'s test file holds the placeholder `@REQ@` where its marker's prefix goes, and `make.sh` writes the marker. `corpus/make.sh <version> <clm>` builds an entry with the given binary in a temporary directory under version control, fails unless that binary's `clm check` passes on it, and writes the tarball deterministically: sorted names, mtime 0, owner 0, no user names. The first entries are seeded with each published release's binary (`gh release download v<x> --pattern 'clm-x86_64-unknown-linux-musl.tar.gz'`), editing the copied sample where an older release's format differs. The corpus test, which runs the built `clm`, is `clm`'s (`clm/upgrade-corpus`).

## MODIFIED Requirements

### Requirement: migration-list <!-- base: b7ed6123d7850172 -->
`clm-upgrade` SHALL offer `migrations()`, its own migrations in the order they landed, each with a unique `id`, a `level`, a `done` check over the project's files, and a `kind` that is either an apply step or a note, every `Required` one an apply migration. A note migration MAY carry `old` and `new` file texts. `latest_in(list)` SHALL be the ID of the last migration of the list it is given, and nothing for an empty list. A `Migration` SHALL be a value a caller can copy, unchanged, into a list of its own, so `init` can offer this list followed by its own (`init/all-migrations`). The crate SHALL offer, as public calls, `supersedes()`, its own pairs (`superseded-notes`), and the forms that take the list and the pairs from their caller: `latest_in(list)`, `pending_in(list, supersedes, root)`, `require_current_in(list, supersedes, root)`, `list_in`, `apply_in`, `note_in` and `decline_in`, each doing over that list what the requirement of its command or call states.

#### Scenario: well formed
- GIVEN the binary's migrations
- WHEN `migrations()` is read
- THEN no two share an ID, each has exactly one of `apply` and `note`, none is `required` with a `note`, and `latest_in` of it is the last one's ID

#### Scenario: the forms that take a list are public
- GIVEN a caller outside the crate, and a project stamped with the last ID of the crate's own list
- WHEN it calls `latest_in`, `pending_in`, `require_current_in`, `list_in`, `apply_in`, `note_in` and `decline_in` with `migrations()` and `supersedes()`, and `pending_in` again over the list's first migration alone and over an empty list
- THEN each answers over the list it is given: `latest_in` names its last, `require_current_in` returns `Ok`, `list_in` prints one line per migration `pending_in` returns, `note_in` prints the first one's note, `decline_in` refuses an ID the list lacks, and `apply_in` sets `upgraded` to the list's last ID; over the first migration alone `pending_in` returns at most that one, and over an empty list nothing

#### Scenario: a fresh project is done
- GIVEN a repository that the built `clm`'s `init` has just set up
- WHEN every migration's `done` runs on it, and `pending_in` runs with `migrations()` and `supersedes()`
- THEN every `done` returns true but that of `20261105-clm-upgrade-2`, a note that `20261112-agents-md`, which is done, supersedes, and `pending_in` returns nothing

### Requirement: pending-migrations <!-- base: de5ff412169cf937 -->
`pending_in(list, supersedes, root)` SHALL return, in the order of the list it is given, each migration whose `done` is false and that is: an apply migration after the one `clm.toml`'s `upgraded` names; or a note migration that `declined_migrations` doesn't name. When `clm.toml` sets no `upgraded`, or doesn't parse as TOML, every apply migration counts as after it. An `upgraded` that isn't a string SHALL count as none, and `declined_migrations` SHALL decline only the strings of an array, so a value that isn't an array declines nothing and an entry that isn't a string is passed over. When `upgraded` names a migration the list doesn't hold, it SHALL return `TooNew` naming the ID. It SHALL read no list but the one it is given: a migration that list holds and the crate's own doesn't is judged like any other. Four of the crate's own notes ask a project only to read them, and their `done`, which can't change once a release ships it, looks for the stamp in the crate's own list: `20261107-projects-upgrading-past-the`, `20261108-toolchain-neutral-land-builds`, `20261109-a-build-s-tests` and `20261114-proof-is-real`. So `pending_in` SHALL also leave each of those four out when `upgraded` names a migration at or after it in the list it is given, which is what their `done` says of a stamp the crate's own list holds, and holds too for a stamp that names a migration of `init`'s (`stamp-taken-notes`).

#### Scenario: stamped
- GIVEN apply migrations `a`, `b` and `c` whose `done` is false, and `clm.toml` with `upgraded = "a"`
- WHEN `pending_in` runs with that list
- THEN it returns `b` and `c`

#### Scenario: already right
- GIVEN the same, with `c`'s `done` true
- WHEN `pending_in` runs with that list
- THEN it returns `b`

#### Scenario: never stamped
- GIVEN apply migrations `a`, `b` and `c`, a `clm.toml` with no `upgraded`, and files for which only `b`'s `done` is false
- WHEN `pending_in` runs with that list
- THEN it returns `b`

#### Scenario: a declined note
- GIVEN note migrations `n` and `m`, neither done, and `declined_migrations = ["n"]`
- WHEN `pending_in` runs with that list
- THEN it returns `m`

#### Scenario: odd values
- GIVEN apply migrations `a` and `b` and a note migration `n`, none done
- WHEN `pending_in` runs with that list, for a `clm.toml` reading `upgraded = 3`, then `declined_migrations = "n"`, then `declined_migrations = ["n", 1]`
- THEN the first returns `a`, `b` and `n`, the second returns `a`, `b` and `n`, and the third returns `a` and `b`

#### Scenario: upgraded by a newer clm
- GIVEN `clm.toml` with `upgraded = "z"`, and no migration `z`
- WHEN `pending_in` runs with that list
- THEN it returns `TooNew` naming `z`

#### Scenario: a migration the crate doesn't hold
- GIVEN a list of the crate's own migrations followed by one more apply migration `z` whose `done` is false, and a project whose `upgraded` names the last of the crate's own
- WHEN `pending_in` runs with that list, and again with the crate's own list alone
- THEN the first returns `z`, and the second returns nothing

#### Scenario: stamped at a migration the crate doesn't hold
- GIVEN the same list, and a project whose `upgraded` names `z` and for which `z` is done
- WHEN `pending_in` runs with that list
- THEN it returns nothing: none of the four notes whose `done` reads the stamp is pending

### Requirement: upgrade-apply <!-- base: e3cdf9eeacbcba10 -->
`clm upgrade --apply`, which is `apply_in(list, supersedes, root)` with the list its caller gives, SHALL, in list order, run each pending apply migration and rewrite each pending note migration's file to its `new` text when the file equals one of its `old` texts. It SHALL then set `clm.toml`'s top-level `upgraded` to `latest_in` of that list, by replacing its line or, failing that, adding it above the first table header's lead-in: the comment lines directly above that header, with no blank line between, and the blank lines directly above them; and, when `clm.toml` has no table header, adding it as the last line, after ending the file's last line when it has no newline. It SHALL follow an added line with a blank line when no blank line already follows it there, and change no other line of `clm.toml`. It SHALL print `applied <id>` for each migration it changed and `note <id>` for each note migration still pending after its rewrites, judging which are pending before it writes `upgraded`, so a note whose `done` reads the stamp is shown on the run that takes it; and it SHALL commit nothing. Run again, it SHALL change nothing. On `TooNew` it SHALL exit 1 and write nothing. When `clm.toml` doesn't parse as TOML it SHALL exit 1 with a message naming `clm.toml`, running no migration and writing nothing; when there is no `clm.toml` it SHALL run what is pending and write no stamp. When a migration fails, an apply step returning an error or a note's file that can't be written, it SHALL still run the rest, write no `upgraded`, and exit 1 with a message holding an `applied <id>` line for each migration it changed, in list order, then a line `` migration `<id>` failed: <why> `` for each that failed, and no `note` line; when `clm.toml` can't be written, its message SHALL hold the `applied` lines, then `can't write clm.toml: <why>`.

#### Scenario: mechanical, and an edited file
- GIVEN pending apply migration `m`, and pending note migration `n` with `old`/`new` whose file the project has edited, and a `clm.toml` ending in a `[land]` table
- WHEN `clm upgrade --apply` runs
- THEN `m`'s change is made, `upgraded` is a top-level key set to the newest migration, it prints `applied m` and `note n`, and a second run changes nothing and prints `note n`

#### Scenario: a commented header
- GIVEN a `clm.toml` with no `upgraded` that reads `host = "github"`, a blank line, `# Run before clm check.` and `[land]`
- WHEN `clm upgrade --apply` runs
- THEN it reads `host = "github"`, `upgraded = "<newest>"`, a blank line, `# Run before clm check.` and `[land]`, the comment still directly above `[land]`

#### Scenario: where the stamp goes
- GIVEN a pending apply migration `m`
- WHEN `clm upgrade --apply` runs for a `clm.toml` reading `a = 1` with no final newline, and for a project with no `clm.toml`
- THEN the first `clm.toml` reads `a = 1`, then `upgraded = "<newest>"`, each line ending in a newline; and in the second `m`'s change is made, it prints `applied m`, and no `clm.toml` is written

#### Scenario: an odd stamp line
- GIVEN apply migrations `s` and, after it, `m`, neither done
- WHEN `clm upgrade --apply` runs for a `clm.toml` reading `upgraded = [`, `defects = "ask"`, a blank line, `[land]` and `check = "true"`; for one reading `upgraded = "s" # was [old`, `defects = "ask"`, a blank line, `[land]` and `check = "true"`; and for one reading `"upgraded" = "s"` and `defects = "ask"`
- THEN the first exits 1 naming `clm.toml`, makes neither migration's change and leaves `clm.toml` as it was; the second prints `applied m`, exits 0, and leaves `clm.toml` reading `upgraded = "m"` and then every other line as it was; and the third prints `applied m`, exits 0, and leaves `clm.toml` parsing as TOML with one `upgraded`, set to `m`, and `defects = "ask"` as it was

#### Scenario: a failing migration
- GIVEN apply migrations `bad`, whose step fails with `boom`, and `m` after it, a note migration `n` with `old`/`new` whose file equals an `old` text and can't be written, and an empty `clm.toml`; and a second project with only `m` pending, whose `clm.toml` can be read and not written
- WHEN `clm upgrade --apply` runs in each
- THEN in the first `m`'s change is made, it exits 1 with `applied m`, then `` migration `bad` failed: boom `` and a `` migration `n` failed: `` line, and `clm.toml` is unchanged; and in the second `m`'s change is made, and it exits 1 with `applied m`, then `can't write clm.toml: ` and the reason

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

### Requirement: upgrade-note-and-decline <!-- base: 727f691381ea19fb -->
`clm upgrade --note <id>` SHALL print the migration's note. `clm upgrade --decline <id>` SHALL add `<id>` to `clm.toml`'s top-level `declined_migrations` when it names a pending note migration, replacing that key's lines or, when it has none, adding the key where `--apply` adds `upgraded`, change no other line of `clm.toml`, and print `declined <id>`. Each SHALL exit 1, naming why and writing nothing, for an ID that names no migration, for `--note` of an apply migration, for `--decline` of a migration that isn't a pending note migration or when `clm.toml` is missing or doesn't parse as TOML, and on `TooNew`.

#### Scenario: declining advised drift
- GIVEN a pending note migration `a`
- WHEN `clm upgrade --decline a` runs
- THEN it prints `declined a`, `declined_migrations` holds `a`, and `clm upgrade` no longer lists it

#### Scenario: declining under a commented header
- GIVEN a pending note migration `a`, and a `clm.toml` with no `declined_migrations` whose first table header `[land]` has the comment `# Run before clm check.` directly above it
- WHEN `clm upgrade --decline a` runs
- THEN `declined_migrations = ["a"]` is a top-level line above that comment, and the comment is still directly above `[land]`

#### Scenario: declining a required migration
- GIVEN a pending apply migration `r`
- WHEN `clm upgrade --decline r` runs
- THEN it exits 1 naming `r`, and `clm.toml` is unchanged

#### Scenario: an odd declined line
- GIVEN pending note migrations `a` and `b`
- WHEN `clm upgrade --decline a` runs for a `clm.toml` reading `declined_migrations = ["b"] # was [old`, `defects = "ask"`, a blank line, `[land]` and `check = "true"`; and for one reading `"declined_migrations" = ["b"]` and `defects = "ask"`
- THEN each prints `declined a` and exits 0; the first leaves `clm.toml` reading `declined_migrations = ["b", "a"]` and then every other line as it was; and the second leaves it parsing as TOML with one `declined_migrations`, `["b", "a"]`, and `defects = "ask"` as it was

#### Scenario: what note and decline refuse
- GIVEN an apply migration `r` and a pending note migration `a`
- WHEN `clm upgrade --note r` runs; `clm upgrade --decline a` runs for a `clm.toml` reading `declined_migrations = ["x"`; and `clm upgrade --decline a` runs with no `clm.toml`
- THEN the first exits 1 naming `r`; the second exits 1 naming `clm.toml` and leaves it as it was; and the third exits 1 naming `clm.toml` and writes none

#### Needs
- config/upgrade-settings

### Requirement: shipped-migrations-frozen <!-- base: 46f86d46b791cc4d -->
For each line `<id> <version> <hash>` of `crates/clm-upgrade/migrations.lock`, `migrations()` SHALL hold `<id>`, and the hash of its directory, as **The lock** defines it, SHALL be `<hash>`. `append_lock(crate_dir, version, ids)`, which `clm upgrade --lock <version>` runs for each crate that holds migrations (`clm/upgrade-command`), SHALL append one such line per ID the lock doesn't list, naming `<version>`, change no existing line, and return the IDs it locked. The crate SHALL offer both as public calls over any crate's directory, `check_lock(crate_dir, ids)`, which returns `Ok` when every locked migration is among `ids` and hashes as locked, and otherwise an error naming every one that is gone or was edited, one line each in the lock's order, with the version that shipped it, and `append_lock`, reading `<crate_dir>/migrations.lock`, a missing file as empty, and `<crate_dir>/src/migrations/<id>/`, so `init` holds its own migrations to the same rule (`init/own-migrations-frozen`). A lock line that isn't three words separated by white space SHALL fail both calls, naming the line, a blank line being passed over; and `append_lock` SHALL end a last line that has no newline before it appends.

#### Scenario: a shipped migration edited
- GIVEN a lock line for migration `m` and a later edit to a file in `m`'s directory
- WHEN the lock is checked
- THEN the check fails, naming `m` and the version that shipped it

#### Scenario: locking a release
- GIVEN a lock listing `a`, and migrations `a` and `b`
- WHEN `append_lock` runs over that crate with `0.3.0` and both IDs
- THEN the lock gains one line, for `b` and naming `0.3.0`, `a`'s line is unchanged, and it returns `b`

#### Scenario: no lock yet
- GIVEN a crate directory with no `migrations.lock`, holding migration `z`
- WHEN `check_lock` runs over it with no IDs, then `append_lock` with `0.1.0` and `z`
- THEN `check_lock` returns `Ok`, and `append_lock` writes a `migrations.lock` holding one line, for `z` and naming `0.1.0`, and returns `z`

#### Scenario: two differences
- GIVEN a crate directory holding migration `b`, whose lock has a line for `a`, shipped in `0.1.0`, and one for `b`, shipped in `0.2.0`, and a later edit to a file in `b`'s directory
- WHEN `check_lock` runs over it with `b`
- THEN it fails with two lines, the first naming `a` as gone and `0.1.0`, the second naming `b` as edited and `0.2.0`

#### Scenario: a malformed lock
- GIVEN a crate directory holding migration `a`, whose lock holds the line `m 0.2.0`; and another, holding migrations `a` and `b`, whose lock reads `a 0.1.0 x` with no final newline
- WHEN `check_lock` with `a`, and `append_lock` with `0.2.0` and `a`, run over the first, and `append_lock` with `0.2.0`, `a` and `b` over the second
- THEN over the first both fail naming `m 0.2.0`, and its lock is unchanged; and the second lock reads `a 0.1.0 x`, a newline, and then one line, for `b` and naming `0.2.0`

### Requirement: migration-check-script-from-root <!-- base: 46e5be9997bf36ca -->
An `advised` note migration SHALL bring a `scripts/clm-check.sh` that `clm init` wrote before its script ran from the repository's root, as `v0.14.0` and `v0.15.0` wrote it, to the text `v0.16.0` wrote. Its `done` SHALL be true when `scripts/clm-check.sh` doesn't exist or holds the line `cd "$(dirname "${BASH_SOURCE[0]}")/.."`. Its `old` SHALL be the script those releases' `clm init --lang rust` wrote, a bare `cargo test` after `set -euo pipefail`, and its `new` the script `clm init --lang rust` wrote from `v0.16.0` to `v0.23.0`, which `migration-starter-land-dir` then brings to the text written today, so an unedited script is rewritten and any other is left to the note. `clm upgrade --note 20261106-corpus-entries-drift-from` SHALL print a note that says what changed for every language: the script changes to the repository's root first, and runs each language's build and tests only when that language's manifest exists.

#### Scenario: an unedited 0.14 script
- GIVEN a `scripts/clm-check.sh` equal to what `clm init --lang rust` `v0.14.0` wrote
- WHEN `clm upgrade --apply` runs
- THEN it equals what the current `clm init --lang rust` writes, and `clm upgrade` no longer lists the migration

#### Scenario: an edited one
- GIVEN that script with a line of the project's own added
- WHEN `clm upgrade --apply` runs
- THEN the script is unchanged, and it prints `note` and this migration's ID

### Requirement: migration-offer-verify <!-- base: 04857c466bf1fecb -->
An `advised` apply migration SHALL bring a project set up before `clm verify` the commented-out `[verify]` section `clm init` writes (`init/init-verify-comment`), appending it to `clm.toml`, after one blank line, when it holds no line that is `[verify]` or `# [verify]` ignoring surrounding whitespace. Its `done` SHALL be true when `clm.toml` holds such a line or doesn't exist. While it is pending, `clm upgrade` SHALL list it as `20261107-slow-tests-after-land advised apply Slow tests can run after the land: offer [verify]`. It changes no setting: the section stays commented until the human sets it.

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

### Requirement: migration-unattended-allow-rules <!-- base: 49231d4a88e71f01 -->
An `advised` note migration, ID `20261107-projects-set-up-before`, SHALL offer a project the allow rules for unattended `clm` commands that `clm init` seeds on a first run (`init/init-seeds-permissions`) and that the project lacks. Its rules SHALL be those that requirement listed when this migration shipped, whatever `clm init` seeds since: for each of the `clm` subcommands `apply`, `branch`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase`, `verify` and `workaround`, the rules `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)`, twenty-two in all. Its `done` SHALL be true when the union of the `permissions.allow` arrays of `.claude/settings.json` and `.claude/settings.local.json` under the root, each when present, holds every one of those rules, read and compared as `init/init-check-permissions` reads and compares them: as exact strings, a file that isn't JSON of that shape allowing nothing; both paths are plain paths under the root, not `config`'s `local_settings_path`, as `clm init --check-permissions` reads them. `clm upgrade --apply` SHALL write neither settings file for it: they are the human's, and a rule a project removed on purpose is not put back (`unattended-steps-run-clm`). While it is pending, `clm upgrade` SHALL list it as `20261107-projects-set-up-before advised note Unattended sessions need clm's allow rules in .claude/settings.json`. `clm upgrade --note 20261107-projects-set-up-before` SHALL print a note that lists the rules; says that `clm init --check-permissions` prints those the project lacks and that the human adds them to `.claude/settings.json` by hand; says that `clm upgrade --decline 20261107-projects-set-up-before` keeps a project that leaves rules out on purpose as it is; and says that when the root component's `paths` don't list `.claude/settings.json`, as in a project set up before `clm init` seeded the rules, a new settings file needs adding there through a spec change before `clm check` passes.

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

### Requirement: migration-tests-overlap <!-- base: 69dca72c253ad5ea -->
An `advised` note migration, ID `20261107-projects-upgrading-past-the`, SHALL tell a project set up before builds' pre-land checks stopped waiting for the clone's check lock (`land/check-lock`) that its tests must now pass while other runs of them overlap (`tests-run-concurrently`). It SHALL edit no file: `clm upgrade --apply` changes nothing for it but `clm.toml`'s `upgraded` line. Its `done` SHALL be true when `clm.toml` doesn't exist, or parses as TOML and its `upgraded` names this migration or one after it in the crate's own `migrations()`, and false otherwise; for a stamp that names a later migration of `init`'s, `pending_in` leaves this note out (`pending-migrations`), so a project `clm init` set up since this migration is done, and an older one is done once `clm upgrade --apply` has stamped it (`stamp-taken-notes`). While it is pending, `clm upgrade` SHALL list it as `20261107-projects-upgrading-past-the advised note Builds' checks now overlap: your tests must pass side by side`. `clm upgrade --note 20261107-projects-upgrading-past-the` SHALL print a note that says:
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

### Requirement: migration-land-dir <!-- base: 3295cbd65c14d1e3 -->
An `advised` note migration, ID `20261108-toolchain-neutral-land-builds`, SHALL tell a project set up before it that lands now hand its check a build place of its own and check only where a branch and `main` overlap. It SHALL edit no file: `clm upgrade --apply` changes nothing for it but `clm.toml`'s `upgraded` line. Its `done` SHALL be true when `clm.toml` doesn't exist, or parses as TOML and its `upgraded` names this migration or one after it in the crate's own `migrations()`, and false otherwise; for a stamp that names a later migration of `init`'s, `pending_in` leaves this note out (`pending-migrations`), as `migration-tests-overlap`'s is. While it is pending, `clm upgrade` SHALL list it as `20261108-toolchain-neutral-land-builds advised note Lands hand your check CLM_LAND_DIR and re-check only the overlap`. `clm upgrade --note 20261108-toolchain-neutral-land-builds` SHALL print a note that says:
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

### Requirement: migration-build-setup <!-- base: 4fdbe90baf2377c1 -->
An `advised` note migration, ID `20261109-a-build-s-tests`, SHALL tell a project set up before it that a build's tests must run against its own worktree's code and dependencies, and how to declare the setup that makes them. It SHALL edit no file: `clm upgrade --apply` changes nothing for it but `clm.toml`'s `upgraded` line. Its `done` SHALL be true when `clm.toml` doesn't exist, or parses as TOML and its `upgraded` names this migration or one after it in the crate's own `migrations()`, and false otherwise; for a stamp that names a later migration of `init`'s, `pending_in` leaves this note out (`pending-migrations`), as `migration-tests-overlap`'s is. While it is pending, `clm upgrade` SHALL list it as `20261109-a-build-s-tests advised note Builds run your tests in a fresh worktree: declare [build] setup`. `clm upgrade --note 20261109-a-build-s-tests` SHALL print a note that says:
- that each build runs in a fresh worktree inside the main checkout, with nothing installed, so a toolchain whose lookup walks up the parent directories, or an environment the checkouts share, can resolve the main checkout's packages or source instead of the branch's, and a build's red or green test then means nothing (`tests-run-in-their-own-tree`, which `clm guide constraints` states);
- that the project declares in `clm.toml` `[build] setup`, a shell command that makes a fresh worktree ready to run its tests, such as one installing its dependencies into it, and that `clm setup` runs it, which builds and drafts do before their first test run, while `[land] check` stays a command that needs no setup before it;
- how to check the project: create a worktree under the main checkout, run its tests there without installing anything, and see whether they fail for want of their dependencies or resolve them from outside the worktree; for example, a project whose tests use npm packages can set `setup = "test ! -f package.json || npm ci"`;
- that taking the note edits no file and leaves nothing to decline: the upgrade's stamp marks it taken.

#### Scenario: a project set up before
- GIVEN a project whose `clm.toml` sets `upgraded` to the migration before this one, with no `scripts/clm-check.sh`, every other migration done
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

### Requirement: migration-starter-land-dir <!-- base: 131020e2d21b2db8 -->
An `advised` note migration, ID `20261109-land-still-sets-cargo`, SHALL bring a `scripts/clm-check.sh` that `clm init --lang rust` wrote from `v0.16.0` to `v0.23.0` to the text it writes since `init/starter-check-land-dir`, since `clm land` no longer sets cargo's variables for a project's check (`land/shared-land-target`). Its `file` SHALL be `scripts/clm-check.sh`, its `old` the script those releases' `clm init --lang rust` wrote (one text, the same in each), and its `new` the script the current `clm init --lang rust` writes, so an unedited script is rewritten and any other is left to the note. Its `done` SHALL be true when `scripts/clm-check.sh` doesn't exist, holds `CLM_LAND_DIR`, or doesn't hold `cargo`, and false otherwise. While it is pending, `clm upgrade` SHALL list it as `20261109-land-still-sets-cargo advised note Lands no longer set CARGO_TARGET_DIR: point cargo at CLM_LAND_DIR in your check`. `clm upgrade --note 20261109-land-still-sets-cargo` SHALL print a note that says:
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

### Requirement: superseded-notes <!-- base: e41a6e668b537d2d -->
A migration SHALL be able to name the earlier note migrations it supersedes, as a pair of IDs among the `supersedes` pairs the caller gives, and `pending_in` SHALL leave out a note migration that a migration whose `done` is true supersedes, since its edit is to a text the later migration has moved. A shipped migration's own files stay as they shipped (`shipped-migrations-frozen`).

#### Scenario: a superseded note
- GIVEN note migration `n` not done, and a later migration `m` that supersedes `n` and is done
- WHEN `pending_in` runs with that list and that pair
- THEN it doesn't return `n`

#### Scenario: not yet superseded
- GIVEN the same, with `m` not done
- WHEN `pending_in` runs with that list and that pair
- THEN it returns `n`

### Requirement: migration-agents-md <!-- base: 75947c8b1bcd6fda -->
An `advised` `apply` migration, ID `20261112-agents-md`, SHALL move a project's instructions to `AGENTS.md`, which every supported agent harness reads (`platform`). Its `done` SHALL be true when `CLAUDE.md` doesn't exist or holds the line `@AGENTS.md`. Its `apply` SHALL, when `AGENTS.md` is missing, write to `AGENTS.md` the text the current `clm init` writes there when `CLAUDE.md`'s text equals one a released `clm init` wrote, and `CLAUDE.md`'s text unchanged otherwise, and replace `CLAUDE.md` with the one line `@AGENTS.md`, so an unedited project ends as `clm init` writes it today (`clm/upgrade-corpus`) and an edited one keeps its edits; and, when both files exist, add the line `@AGENTS.md` as `CLAUDE.md`'s first line, followed by a blank line, changing nothing else, so no instruction is lost or moved under the human's feet. It SHALL supersede `20261105-clm-upgrade-2`, whose `done` reads `CLAUDE.md` for `/clm:help`, which the moved text no longer holds there. While it is pending, `clm upgrade` SHALL list it as `20261112-agents-md advised apply Move CLAUDE.md's instructions to AGENTS.md, which every agent harness reads`.

#### Scenario: CLAUDE.md alone
- GIVEN a project whose edited `CLAUDE.md` holds the instructions and no `AGENTS.md`
- WHEN `clm upgrade` runs, then `clm upgrade --apply`
- THEN the first lists `20261112-agents-md advised apply Move CLAUDE.md's instructions to AGENTS.md, which every agent harness reads`, and after the second `AGENTS.md` holds that text and `CLAUDE.md` is `@AGENTS.md`

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

#### Serves
- capability/upgrade-clm
- capability/work-from-any-agent-harness

### Requirement: migration-setup-allow-rules <!-- base: 7a85824eb9919282 -->
An `advised` note migration, ID `20261112-a-draft-s-clm`, SHALL offer a project the two allow rules for `clm setup` that `init/init-seeds-permissions` adds to `clm init`'s seeding, `Bash(clm setup)` and `Bash(clm setup *)`, when the project lacks either. Its rules SHALL be those two, whatever `clm init` seeds since, and `20261107-projects-set-up-before`, which is frozen once shipped (`shipped-migrations-frozen`), keeps offering the other twenty-two. Its `done` SHALL be true when the union of the `permissions.allow` arrays of `.claude/settings.json` and `.claude/settings.local.json` under the root, each when present, holds both rules, read and compared as `migration-unattended-allow-rules` reads and compares them. `clm upgrade --apply` SHALL write neither settings file for it (`unattended-steps-run-clm`, `allow-rules-offered-by-note`). While it is pending, `clm upgrade` SHALL list it as `20261112-a-draft-s-clm advised note Unattended sessions need clm's allow rules for clm setup in .claude/settings.json`. `clm upgrade --note 20261112-a-draft-s-clm` SHALL print a note that lists the two rules; says that builds and drafts run `clm setup` before their first test run on a code branch (`tests-run-in-their-own-tree`) and that Claude Code's auto mode denies it when no rule allows it; says that the human adds them to `.claude/settings.json` by hand and that `clm init --check-permissions` prints the rules the project lacks; and says that `clm upgrade --decline 20261112-a-draft-s-clm` keeps a project that leaves them out on purpose as it is.

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

### Requirement: migration-build-allow-rules <!-- base: 1c2d0022a45eb4ad -->
An `advised` note migration, ID `20261112-build-loop-in-clm`, SHALL offer a project the two allow rules for `clm build` that `init/init-seeds-permissions` adds to `clm init`'s seeding, `Bash(clm build)` and `Bash(clm build *)`, when the project lacks either. Its rules SHALL be those two, whatever `clm init` seeds since, and the shipped migrations that offer the other rules stay as they are (`shipped-migrations-frozen`). Its `done` SHALL be true when the union of the `permissions.allow` arrays of `.claude/settings.json` and `.claude/settings.local.json` under the root, each when present, holds both rules, read and compared as `migration-unattended-allow-rules` reads and compares them. `clm upgrade --apply` SHALL write neither settings file for it (`unattended-steps-run-clm`, `allow-rules-offered-by-note`). While it is pending, `clm upgrade` SHALL list it as `20261112-build-loop-in-clm advised note Unattended sessions need clm's allow rules for clm build in .claude/settings.json`. `clm upgrade --note 20261112-build-loop-in-clm` SHALL print a note that lists the two rules; says that `/clm:build` runs every step of its loop through `clm build` and that Claude Code's auto mode denies a step when no rule allows it; says that the human adds them to `.claude/settings.json` by hand and that `clm init --check-permissions` prints the rules the project lacks; and says that `clm upgrade --decline 20261112-build-loop-in-clm` keeps a project that leaves them out on purpose as it is.

#### Scenario: a project lacking the build rules
- GIVEN a `.claude/settings.json` that sets `enabledPlugins` and allows every other rule `clm init` seeds but neither `clm build` rule
- WHEN `clm upgrade` runs, then `clm upgrade --apply`
- THEN the first lists `20261112-build-loop-in-clm advised note Unattended sessions need clm's allow rules for clm build in .claude/settings.json`, the second prints `note 20261112-build-loop-in-clm`, and `.claude/settings.json` is unchanged

#### Scenario: split across the two files
- GIVEN a `.claude/settings.json` allowing `Bash(clm build)`, and a `.claude/settings.local.json` allowing `Bash(clm build *)`
- WHEN `clm upgrade` runs
- THEN it doesn't list `20261112-build-loop-in-clm`

#### Scenario: declined
- GIVEN a project lacking the build rules, whose `clm.toml` sets `declined_migrations = ["20261112-build-loop-in-clm"]`
- WHEN `clm upgrade` runs
- THEN it doesn't list `20261112-build-loop-in-clm`

#### Needs
- init/init-seeds-permissions

### Requirement: migration-markers-in-tests <!-- base: 25966374992f4603 -->
An `advised` note migration, ID `20261114-proof-is-real`, SHALL tell a project set up before a marker counted only inside a test (`check.coverage/markers-in-tests`) what changes for it. It SHALL edit no file: `clm upgrade --apply` changes nothing for it but `clm.toml`'s `upgraded` line. Its `done` SHALL be true when `clm.toml` doesn't exist, or parses as TOML and its `upgraded` names this migration or one after it in the crate's own `migrations()`, and false otherwise; for a stamp that names a later migration of `init`'s, `pending_in` leaves this note out (`pending-migrations`), so a project `clm init` set up since is done, and an older one is done once `clm upgrade --apply` has stamped it (`stamp-taken-notes`). While it is pending, `clm upgrade` SHALL list it as `20261114-proof-is-real advised note A marker now counts only inside a test: see what stops counting before the next build`, and `clm upgrade --note 20261114-proof-is-real` SHALL print a note that names `clm coverage`, whose `uncounted` lines list the markers that stopped counting, `wiring`, the root key that switches the ownership rule on, and `[verify] mutate`, the setting of the mutation pass, both off until the project sets them. Of the ownership rule it SHALL say that `clm land` refuses a build's branch, `impl/<component>`, that changes a file the rule doesn't count as its own (`land/build-holds-own-files`), which no review lets through; that naming `wiring` also has `clm check` report, under `tests-cite-their-owner`, each `req:` marker in a file a component owns that cites a requirement of a component other than the file's owner (`check/tests-cite-their-owner`); and that a project whose tests share files first gives each test file to its component with a `tests` glob (`model/test-ownership-keys`) before it names `wiring`. It SHALL NOT say that such a branch needs review.

#### Scenario: a project set up before
- GIVEN a project whose `clm.toml` sets `upgraded` to the migration before this one, every other migration done
- WHEN `clm upgrade` runs, then `clm upgrade --apply`, then `clm upgrade`
- THEN the first lists `20261114-proof-is-real advised note A marker now counts only inside a test: see what stops counting before the next build`, the second prints `note 20261114-proof-is-real` and changes no file but `clm.toml`'s `upgraded` line, and the third prints `up to date`

#### Scenario: the note
- GIVEN the migration
- WHEN `clm upgrade --note 20261114-proof-is-real` runs on a project it is pending in
- THEN the note names `clm coverage`, `uncounted`, `wiring`, `clm land` refuses, `clm check`, `tests-cite-their-owner`, `tests` glob and `[verify] mutate`, and doesn't hold `needs review`

#### Needs
- check.coverage/markers-in-tests
- check/coverage-lists
- config/verify-setting

### Requirement: own-list-closed <!-- base: 6b42898f99a71b5f -->
`migrations()` SHALL hold exactly these thirteen migrations, in this order, and no other: `20261105-clm-upgrade`, `20261105-clm-upgrade-2`, `20261106-corpus-entries-drift-from`, `20261107-projects-set-up-before`, `20261107-slow-tests-after-land`, `20261107-projects-upgrading-past-the`, `20261108-toolchain-neutral-land-builds`, `20261109-land-still-sets-cargo`, `20261109-a-build-s-tests`, `20261112-agents-md`, `20261112-a-draft-s-clm`, `20261112-build-loop-in-clm` and `20261114-proof-is-real`. A migration written since is `init`'s (`init/all-migrations`), so the build that changes what a project holds writes the migration in its own component (`migrations-live-with-init`).

#### Scenario: the list as it stands
- GIVEN the crate
- WHEN `migrations()` is read
- THEN it holds those thirteen IDs, in that order

## REMOVED Requirements

### Requirement: forms-without-a-list-gone <!-- base: 991edcce20a4f358 -->
