---
component: upgrade
base: { design: "4fd44e9b514c86a0" }
---

## MODIFIED Design

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

## MODIFIED Requirements

### Requirement: upgrade-apply <!-- base: f30eac0c7c9583e9 -->
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

### Requirement: upgrade-note-and-decline <!-- base: d61ca0b9065c9bc5 -->
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
