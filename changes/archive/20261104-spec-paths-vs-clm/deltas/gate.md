---
component: gate
---

## MODIFIED Requirements

### Requirement: files-outside-spec-paths <!-- base: b74ab21302a51766 -->
`clm-gate` SHALL offer a library call returning the files a revision changes against a base, from the merge base as `classify` reads them, that aren't spec paths, sorted, and returning none when `rename/replay` accepts the revision. It SHALL leave out `clm.toml` when `approval-setting` counts it as a spec change, deciding that by `approval_changed` of the file's bytes at the merge base and at the revision, so a branch that changes whose approval counts, which needs review, can land as a `spec/*` branch; any other change to `clm.toml` it SHALL return. It SHALL read the revision only through git, never the working tree or the index, as `gate/evaluate-at` does.

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

### Requirement: approval-setting <!-- base: 1b3061c9e5b17441 -->
When the PR's diff lists `clm.toml` (the path at the top of the repository, matched as the gate matches `specs/` and `docs/`, and the file `ledger verify` audits lands by) and the change alters the value of its top-level `approval` key, that file SHALL count as a spec change that needs review (`needs-review`), so the policy deciding whose approval counts changes only with an approval (`approval-setting-is-spec`). It doesn't become a spec path: the list of spec paths stays as it is, and the file is reported as any other spec change is, with no rule of its own; only `files-outside-spec-paths` leaves it off the files it returns, so such a change can land on a `spec/*` branch. The gate SHALL compare the key's value as a parsed TOML value at the merge base and at `HEAD`, read with `git show <rev>:clm.toml`, a missing file or a missing key counting as unset: so editing a comment, the layout or another key changes nothing, setting `approval = "any"` where it was unset is a change, deleting a file that set it is a change, and a value `config` would reject, such as `approval = 1`, is compared like any other value. When the diff lists `clm.toml` and either side's text isn't valid UTF-8 or doesn't parse as TOML, the file SHALL count as a spec change that needs review, so the gate fails closed. Any other change to `clm.toml` SHALL NOT count as a spec change, and `clm.local.toml` is never compared.

#### Scenario: code branch weakens the policy
- GIVEN `main`'s `clm.toml` sets `approval = "other"`, and a branch changes it to `approval = "any"` and changes nothing else
- WHEN `clm gate --base main` runs
- THEN it prints `spec review`, reports `gate/approval` and exits 1

#### Scenario: policy set where it was unset
- GIVEN `main`'s `clm.toml` doesn't set `approval`, and a branch adds `approval = "other"` to it and changes nothing else
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: approved policy change
- GIVEN the branch of *code branch weakens the policy*
- WHEN `clm gate --base main --approved` runs
- THEN it prints `spec review` and exits 0

#### Scenario: policy change with code
- GIVEN a branch that changes `clm.toml`'s `approval` and `crates/x/src/lib.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `mixed` and exits 1

#### Scenario: other setting changed
- GIVEN `main`'s `clm.toml` sets `approval = "other"`, and a branch changes its `[land] check` and a comment, leaves `approval` as it was, and changes `crates/x/src/lib.rs`
- WHEN `clm gate --base main` runs
- THEN it prints `code` and exits 0

#### Scenario: unparseable clm.toml
- GIVEN a branch that changes `clm.toml` so that it isn't valid TOML, and changes nothing else
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Serves
- capability/approve-intent
- architecture/approval-policy
- architecture/humans-review-intent
