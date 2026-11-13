---
component: gate
base: {}
---

## MODIFIED Requirements

### Requirement: needs-review <!-- base: 3276bfca81f7e9e9 -->
A `spec` branch SHALL need review exactly when its diff adds, modifies or deletes `ARCHITECTURE.md`, `CAPABILITIES.md` or `docs/allowed_signers` (the keys a signed repository trusts, `signed-ledger-records`); or `approval-setting` counts `clm.toml` as changed; or a change one of whose files it adds or modifies, a live change or a new archived one, has a `change.md` at `HEAD` that can't be read as UTF-8, whose frontmatter doesn't parse, or that declares a non-empty `human_burdens` (`yes-no-review`), so editing a change's deltas or assets alone doesn't leave its burdens unreviewed; or a component's `SPEC.md` at `HEAD` holds an exemption (`model/exemptions`) whose `files` aren't all among the `files` of an exemption with the same `rule` and `until` that the component with the same ID holds at the merge base (every exemption of a component new on the branch counting), so relaxing a rule, or widening a relaxation, is always seen by a human, while removing or narrowing one, or changing only its `why`, never needs review (`temporary-exemptions`). A `spec` branch that `rename/replay` accepts SHALL instead need review exactly when its added change records a `term` rename, declares a non-empty `human_burdens`, or has a `change.md` that can't be read or parsed: its replayed edits to `ARCHITECTURE.md` and `CAPABILITIES.md` SHALL NOT count. Nothing else SHALL make a branch need review, and a `code` or `mixed` branch never does. `clm gate` SHALL print `spec review` for a `spec` branch that needs review and `spec` for one that doesn't.

#### Scenario: architecture
- GIVEN a branch that only edits `ARCHITECTURE.md`
- WHEN `clm gate --base main --approved` runs
- THEN it prints `spec review` and exits 0

#### Scenario: trusted keys
- GIVEN `main` with `docs/allowed_signers`, and a branch that only adds a key to it
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: capabilities added
- GIVEN `main` without `CAPABILITIES.md`, and a branch that only adds it
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: burdens
- GIVEN a branch that only adds `changes/x/change.md`, `proposed` with `human_burdens: [a paid model run on every land]`
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: deltas of a change with burdens
- GIVEN `main` with `changes/x/change.md`, `applied` with `human_burdens: [b]`, and `changes/x/deltas/a.md`, and a branch that only edits that delta
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: change that doesn't parse
- GIVEN a branch that only adds `changes/x/change.md` whose frontmatter isn't valid YAML
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: nothing else
- GIVEN a branch that changes the Contract of `specs/a/SPEC.md`, removes one of `a`'s requirements, adds `docs/decisions/d.md` and adds `changes/x/change.md`, `applied` with no `human_burdens`
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

#### Scenario: rename of IDs through the human layers
- GIVEN a branch holding only the committed result of `clm rename r`, whose `renames` hold one `requirement` rename that rewrote a line of `ARCHITECTURE.md`'s `#### Enforced by` and a marker in a test
- WHEN `clm gate --base main` runs
- THEN it prints `spec` and exits 0

#### Scenario: term rename
- GIVEN a branch holding only the committed result of `clm rename r`, whose `renames` hold one `term` rename
- WHEN `clm gate --base main` runs
- THEN it prints `spec review` and exits 1

#### Scenario: an added exemption
- GIVEN a `spec` branch whose only change adds an exemption to `specs/a/SPEC.md`'s frontmatter
- WHEN the gate runs
- THEN it prints `spec review`

#### Scenario: a narrowed exemption
- GIVEN a `spec` branch whose only change removes one of the globs an exemption in `specs/a/SPEC.md` lists
- WHEN the gate runs
- THEN it prints `spec`

#### Scenario: a removed exemption
- GIVEN a `spec` branch whose only change removes an exemption from `specs/a/SPEC.md`'s frontmatter
- WHEN the gate runs
- THEN it prints `spec`

#### Needs
- model/exemptions
- rename/replay
- model/parse-changes

## ADDED Requirements

### Requirement: exemption-names-its-item
The gate SHALL block a `spec` branch holding an exemption that `needs-review` counts as added or widened (`model/exemptions`) whose `until` names no backlog item that `backlog/<slug>.md` holds at `HEAD`, reporting `<file>: gate/exemption-names-its-item: exemption from <rule> until backlog/<slug> names no backlog item; add it` for each, so every exemption starts with the work that ends it in the queue (`temporary-exemptions`). An exemption whose item is deleted later SHALL NOT block a branch: `check.health` reports it.

#### Scenario: no item
- GIVEN a `spec` branch that adds to `specs/a/SPEC.md` an exemption from `skipped-tests` until `backlog/x`, with no `backlog/x.md`
- WHEN the gate runs
- THEN it exits 1 with the line `specs/a/SPEC.md: gate/exemption-names-its-item: exemption from skipped-tests until backlog/x names no backlog item; add it`

#### Scenario: item added with it
- GIVEN the same branch, also adding `backlog/x.md`
- WHEN the gate runs
- THEN it reports no `gate/exemption-names-its-item` violation

#### Serves
- architecture/exemptions-end

#### Needs
- model/exemptions
