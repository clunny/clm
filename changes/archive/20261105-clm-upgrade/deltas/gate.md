---
component: gate
---

## ADDED Requirements

### Requirement: upgrade-stamp-left-out
When the diff lists `clm.toml` and its parsed TOML at the merge base and at `HEAD` differ only in the top-level keys `upgraded` and `declined_migrations`, a missing file or key counting as unset, the gate SHALL leave the file out of the files it classifies, and it SHALL NOT make the branch need review. A branch whose only changed file is such a `clm.toml` SHALL classify as `spec`. `clm-gate` SHALL decide it with `upgrade_stamp_only(before, after)`, of the two sides' bytes as `approval_changed` takes them, false when either side isn't UTF-8 or doesn't parse. The function SHALL run no process and SHALL build for `wasm32-unknown-unknown`, so `web.worker` can give the same answer. Any other change to `clm.toml` SHALL be judged as `approval-setting` says.

#### Scenario: notes taken
- GIVEN a branch that edits `CLAUDE.md` and changes only `upgraded` and `declined_migrations` in `clm.toml`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

#### Scenario: mechanical migrations taken
- GIVEN a branch that adds a line to `.gitignore` and changes only `upgraded` in `clm.toml`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: the stamp and another key
- GIVEN a branch that edits `CLAUDE.md` and changes `upgraded` and `[land] check` in `clm.toml`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Serves
- capability/upgrade-clm

## MODIFIED Requirements

### Requirement: files-outside-spec-paths <!-- base: 9ebb011ddd22f399 -->
`clm-gate` SHALL offer a library call returning the files a revision changes against a base, from the merge base as `classify` reads them, that aren't spec paths, sorted, and returning none when `rename/replay` accepts the revision. It SHALL leave out `clm.toml` when `approval-setting` counts it as a spec change, deciding that by `approval_changed` of the file's bytes at the merge base and at the revision, so a branch that changes whose approval counts, which needs review, can land as a `spec/*` branch, and when `upgrade-stamp-left-out` leaves it out, deciding that by `upgrade_stamp_only`; any other change to `clm.toml` it SHALL return. It SHALL read the revision only through git, never the working tree or the index, as `gate/evaluate-at` does.

#### Scenario: a code file among spec files
- GIVEN a revision changing `specs/a/SPEC.md` and `crates/x/src/lib.rs` against its base
- WHEN the call runs with something else checked out
- THEN it returns `crates/x/src/lib.rs` alone

#### Scenario: only a new backlog item
- GIVEN a revision that only adds `backlog/x.md`, for which `clm gate` prints `code`
- WHEN the call runs
- THEN it returns nothing

#### Scenario: a rename branch
- GIVEN the committed result of `clm rename r`, which rewrote a marker in `crates/a/tests/t.rs`
- WHEN the call runs
- THEN it returns nothing

#### Scenario: an approval change
- GIVEN a base whose `clm.toml` sets `approval = "other"`, and a revision that changes it to `approval = "any"` and changes `specs/a/SPEC.md`
- WHEN the call runs
- THEN it returns nothing

#### Scenario: another setting changed
- GIVEN a base whose `clm.toml` sets `approval = "other"`, and a revision that changes its `[land] check` and leaves `approval` as it was
- WHEN the call runs
- THEN it returns `clm.toml`

#### Scenario: only the upgrade stamp changed
- GIVEN a revision that changes only `upgraded` in `clm.toml` and edits `CLAUDE.md`
- WHEN the call runs
- THEN it returns nothing

#### Serves
- capability/contribute-from-outside
- capability/upgrade-clm
