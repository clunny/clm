---
component: gate
---

## ADDED Requirements

### Requirement: upgrade-stamp-is-spec
When the diff lists `clm.toml` and its parsed TOML at the merge base and at `HEAD` differ only in the top-level `upgraded` key, a missing file or key counting as unset, the file SHALL count as a spec change that doesn't need review, and `files-outside-spec-paths` SHALL leave it out, so a branch that edits only the files a project owns in words and moves `upgraded` past the migrations it took lands as a `spec` branch. `clm-gate` SHALL decide it by a function of the two sides' bytes that runs no process and builds for `wasm32-unknown-unknown`, as `approval_changed` does, so `web.worker` gives the same answer. Any other change to `clm.toml` SHALL be judged as before.

#### Scenario: a note migration taken
- GIVEN a branch that edits `CLAUDE.md` and changes only `upgraded` in `clm.toml`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

#### Scenario: a mechanical migration taken
- GIVEN a branch that adds a line to `.gitignore` and changes `upgraded` and `[land] check` in `clm.toml`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Serves
- capability/upgrade-clm
