---
status: applied
resolves:
  - a-spec-branch-s-land-skips-the-code-tests-that-read-spec-text-so
---

# A spec branch's land runs the code tests that read spec text

## Intent

A land on a tree whose code was already checked skips `[land] check`, because `code_key` leaves the doc-only paths out. Some tests read those paths in this repository's own tree (the retired-terms test reads `specs/` and the root documents), so a spec branch that broke one landed on `clm check` alone and left `main` failing `cargo test`. This change has such a test name what it reads with a `clm:reads <glob>` marker. `clm affected` then selects it when a matching path changes, and `clm affected --readers` prints only those tests. `clm land` asks `--readers` on checked code and runs `[land] check` with that selection when it names a test. Spec lands that touch nothing a test reads still skip the check (`doc-readers-marked`).

## Problem

`ledger`'s `code_key` hashes the tree without the doc-only paths, so two trees that differ only in `specs/`, `changes/`, `backlog/`, `docs/` (outside the guide) or the root documents share one key, and `clm land` runs only `clm check --no-detectors` on a tree whose key has a `checked` record. That shortcut assumes no test reads a doc-only path, and the assumption is false here. `crates/clm/tests/retired_terms.rs` reads every file under `specs/` and `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `README.md`, and `readme.rs`, `init.rs`, `plugin.rs`, `community_files.rs`, `contributing.rs`, `glossary.rs`, `says_component.rs`, `lockfiles.rs`, `dev_install.rs` and `crates/clm-init/tests/says_component.rs` read some of these too. `check.affected` drops doc-only paths as well, so a code branch that also edits `README.md` doesn't select `readme.rs` either. The design already treats this case one path at a time: `docs/guide/` and `docs/spec-format.md` count as code because tests read them. Doing the same for `specs/` and the root documents would end the shortcut for every spec land.

## Approaches

- **A `clm:reads <glob>` marker in each reading test, selected by `clm affected` and asked for by `clm land` on checked code (chosen).** It reuses the marker scan and `graph-test-selection`, and it stays free of any language or runner, since a marker is plain text, as `req:` and `clm:remove-by` are (`reuse-first`). A spec land pays only for the tests that read what it changed, in the land tree under the lock, so the target stays warm.
- **Count the read paths as code**, as `docs/guide/` is. Rejected: every spec land changes `specs/`, so every one would run the whole suite and hold the land lock, queueing drafts behind builds.
- **A second check command in `clm.toml` for spec lands.** Rejected: a list of reading tests kept away from the tests drifts as they change.
- **Detect reading tests automatically.** Rejected: which files a test opens can't be read from its text without knowing its language and runner, which `check.affected` doesn't (`ci` does).

## Outcome

- `check.affected`: a file the coverage scan reads that holds `clm:reads <glob>` is a *reader* of the changed paths, doc-only included, that its glob matches. Readers join the `file` lines (`affected-readers`, `affected-files`, `doc-only-selects-nothing`), and `--readers` prints them alone.
- `land`: on a tree whose code key is checked, the land runs `clm affected --base <sha> --readers` in its temporary worktree. Empty output goes on as today, with no lock. Any line moves the attempt into the land tree under the lock, where it runs `[land] check` with that selection, appending no second `checked` record. A failed selection runs every test (`selected-tests`, `land`, the Design's **Readers of checked code**).
- `clm` and `init`: this repository's tests that read its own doc-only files carry the markers (`clm/doc-readers-marked`, `init/doc-readers-marked`).
- `ci`'s Design, which said a checked tree runs only `clm check --no-detectors`, says the readers still run.
- `CLAUDE.md`'s gotcha on checked trees says the readers still run. `docs/decisions/doc-readers-marked.md` records the choice.

## Edge cases and workarounds

- `check.affected`'s own tests and any fixture holding the marker build its text so the literal never appears whole (`concat!("clm", ":reads …")`). Otherwise each such test file would read as a reader of what its fixture names.
- Nothing finds a reading test that lacks its marker: such a test fails on the next full run, as today. The `doc-readers-marked` requirements cover the readers this repository has when it is built. The builder finds them by reading each test that opens the workspace root (`CARGO_MANIFEST_DIR` joined with `../..`).
- On a retry after `main` moved, a checked-code land asks `--readers` again against the new base, so readers can run more than once. That is right, since the merged tree's doc files differ each time.
- A reader's glob may name code paths too (`plugin/**`). Those select it as well, which is harmless.

## Open questions

None.
