---
component: land.approval
---

## MODIFIED Requirements

### Requirement: approval-from-base <!-- base: 1c9573a4f1f68376 -->
`clm land` SHALL count approvals under the `approval` policy that the top-level `clm.toml` committed at the base it lands on sets (the `origin/main` it fetched, or the local `main` with `host = "none"`), `any` when that file or its `approval` key is missing, unless the clone's `clm.local.toml` sets `approval`, whose value SHALL be the policy for that clone's lands; it SHALL never take the policy from the `clm.toml` of the root it runs in, of the head or of the merged tree. When the base's `clm.toml` doesn't parse, or sets a value other than `any` and `other`, it SHALL exit 2 naming `<base>:clm.toml` and `approval`, before running any check or pushing or appending anything.

#### Scenario: a branch that weakens the policy is judged by the base
- GIVEN `origin/main` whose `clm.toml` sets `approval = "other"`, a `spec/x` that needs review authored by `Christopher Lunny` whose head sets `approval = "any"`, an `approved` record by writer `christopher-lunny/1a2b3c4d`, and no `clm.local.toml`
- WHEN `clm land spec/x` runs from a checkout of `spec/x`
- THEN it exits 1 naming `approval = "other"` from `origin/main:clm.toml`, and `main` is unchanged

#### Scenario: a branch that tightens the policy is judged by the base
- GIVEN `origin/main` whose `clm.toml` sets no `approval`, the same `spec/x` whose head sets `approval = "other"`, and the same `approved` record
- WHEN `clm land spec/x` runs from a checkout of `spec/x`
- THEN it lands

#### Scenario: the clone's local file wins
- GIVEN `origin/main` whose `clm.toml` sets `approval = "other"`, the clone's `clm.local.toml` setting `approval = "any"`, and the self-approval of the first scenario
- WHEN `clm land spec/x` runs
- THEN it lands

#### Scenario: no remote
- GIVEN `host = "none"`, a local `main` whose `clm.toml` sets `approval = "other"`, and the `spec/x` and the self-approval of the first scenario
- WHEN `clm land spec/x` runs from a checkout of `spec/x`
- THEN it exits 1 naming `approval = "other"` from `main:clm.toml`, and `main` is unchanged

#### Scenario: unreadable policy at the base
- GIVEN `origin/main` whose `clm.toml` sets `approval = "everyone"`, and a `spec/x` whose `clm.toml` sets `approval = "any"`, with an `approved` record
- WHEN `clm land spec/x` runs from a checkout of `spec/x`
- THEN it exits 2 naming `origin/main:clm.toml` and `approval`, runs no check, and `main` and the ledger are unchanged

#### Needs
- gate/files-outside-spec-paths

#### Serves
- capability/approve-intent
- architecture/land-policy-from-base
