---
component: apply
---

## MODIFIED Requirements

### Requirement: declared-frontmatter-keys <!-- base: 70002c38e95a0844 -->
When validation checks the frontmatter a delta produces, from a `## MODIFIED Frontmatter` or a `create: true` delta's `## Frontmatter`, it SHALL run the model's frontmatter check that takes the component's ID, with the delta's component, the same check the model runs on a loaded component: `model/valid-frontmatter` with the keys that live changes declare in `frontmatter_keys`, the change being applied among them, since it is live (`declared-frontmatter-keys`), and `model/optional-nodes`, which refuses `optional: true` on the root `clm`. A key the model doesn't know and no live change declares, or any other failure of that check, SHALL be a violation, reported as today's frontmatter errors are, under `apply/frontmatter` for a `## MODIFIED Frontmatter` and `apply/create-node` for a created component, with a message naming the component and the key; nothing SHALL be written.

#### Scenario: key the same change declares
- GIVEN change `x` with `frontmatter_keys: [colour]` and a delta for `a` whose `## MODIFIED Frontmatter` sets `colour: blue`, with the correct base
- WHEN `clm apply x` runs
- THEN it exits 0, `a`'s frontmatter has `colour: blue`, and `clm check` reports nothing

#### Scenario: key another live change declares
- GIVEN change `y`, `applied`, with `frontmatter_keys: [colour]`, and change `x`, declaring nothing, whose delta creates component `b` with `colour: red` in its `## Frontmatter`
- WHEN `clm apply x` runs
- THEN it exits 0 and `specs/b/SPEC.md` has `colour: red`

#### Scenario: undeclared key
- GIVEN change `x` declaring nothing, and a delta for `a` whose `## MODIFIED Frontmatter` sets `colour: blue`
- WHEN `clm apply x` runs
- THEN it exits 1 naming `a` and `colour`, and nothing is written

#### Scenario: optional root
- GIVEN change `x` with a delta for `clm` whose `## MODIFIED Frontmatter` sets `optional: true`, with the correct base
- WHEN `clm apply x` runs
- THEN it exits 1 naming `clm` and `optional`, and nothing is written

#### Serves
- capability/propose-intent
- architecture/checks-relax-before-use

#### Needs
- model/optional-nodes
