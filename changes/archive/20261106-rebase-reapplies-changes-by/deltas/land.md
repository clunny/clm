---
component: land
---

## MODIFIED Requirements

### Requirement: derived-specs <!-- base: 5bb5788623229713 -->
When the head adds a change, as `gate.fingerprint/fingerprint` defines it (`clm-gate`'s `adds_change` over the merge base's and the head's listings), or applies one, a change whose `changes/<id>/change.md` has status `proposed` or `approved` at the base and, in the merge commit, has status `applied`, or is gone while `changes/archive/<id>/change.md` has status `done`, `clm land` SHALL, after the merge commit and before any check, regenerate `specs/` with `sync.regenerate`'s `regenerated`, without its stale-delta merge, onto the base (`origin/main`'s, or the local `main`'s when landing offline), from the CLM root in the tree holding the merge commit, for the added changes whose status is `applied`, or `done` for a new archived one, and the applied ones, in the order `regenerated` chooses (`sync.regenerate/regenerated-tree`), a `change.md` that doesn't parse regenerating nothing, and compare the result with the merge commit's `specs/` byte for byte over the union of both sides' paths in byte order, a path on one side only being a difference. When they differ, or `regenerated` stops, it SHALL push nothing and exit 1 with rule `land/derived-specs`, naming the first differing path or the lines `regenerated` returned, and `clm rebase`. A head that adds and applies no change SHALL land without the comparison, since its `specs/` edits are its own. No file under `crates/clm-land/src` SHALL call `clm_apply::reapply` or `clm_model::load`: the regenerate is `sync.regenerate`'s alone. `clm-land` SHALL call it from crate `clm-regenerate`, never through `clm-sync`, which reaches `clm-health` (`clm/health-isolated`).

#### Scenario: regenerated branch lands
- GIVEN `spec/x` adding applied change `x`, whose delta adds requirement `a/r`, with `specs/` as `clm rebase` regenerates it, and an `origin/main` that has since added requirement `b/s`
- WHEN `clm land spec/x` runs
- THEN it lands, and `main`'s `specs/a/SPEC.md` holds `a/r` and `specs/b/SPEC.md` holds `b/s`

#### Scenario: change built on another lands whatever its ID
- GIVEN `spec/x` adding applied changes `x` and `w`, `w` sorting first, where `x`'s delta rewrites the first line of `a`'s Design with `origin/main`'s hash as its base and `w`'s rewrites its second line with the Design's hash after `x` applies as its base, with `specs/` as `clm rebase` regenerates it
- WHEN `clm land spec/x` runs
- THEN it prints no `land/derived-specs` line and lands, and `main`'s `specs/a/SPEC.md` holds both edits

#### Scenario: applied change's deltas lost
- GIVEN an `origin/main` holding change `y` with `status: proposed` and a delta adding requirement `a/x`, and `spec/z` that adds change `z`, which has no delta, and sets `y` to `applied`, with `specs/` as `origin/main`'s, so `a/x` is missing
- WHEN `clm land spec/z` runs
- THEN it exits 1 printing `land/derived-specs`, naming `specs/a/SPEC.md` and `clm rebase`, and `main` on `origin` is unchanged

#### Scenario: hand edit under specs refused
- GIVEN the same branch with one more commit changing a line of `specs/a/SPEC.md` that no delta of `x` holds, and an `approved` record for its fingerprint
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/derived-specs`, naming `specs/a/SPEC.md` and `clm rebase`, the check command never ran, and `main` on `origin` is unchanged

#### Scenario: branch that adds no change
- GIVEN `impl/a`, which adds no change and sets `status: active` in `specs/a/SPEC.md`
- WHEN `clm land impl/a` runs
- THEN it lands with that edit

#### Scenario: no copy of the regenerate
- GIVEN the workspace
- WHEN a test in `clm-land` reads every file under `crates/clm-land/src`
- THEN no file holds `clm_apply::reapply` or `clm_model::load`

#### Serves
- capability/catch-drift
- architecture/regenerate-dont-merge
- architecture/humans-review-intent

#### Needs
- clm/renamed-commands
- gate.fingerprint/fingerprint
- sync.regenerate/regenerated-tree
- land.approval/approve
