---
component: verify
---

## ADDED Requirements

### Requirement: mutated-through-constructor
`clm-verify`, its code and its tests, SHALL build every `mutated` ledger body through `clm_ledger::Body::mutated` (`ledger/mutated-constructor`), and SHALL name the variant `Body::Mutated` only in patterns that end in `..`, so a field `ledger` adds to the variant breaks none of `verify`'s files.

#### Scenario: no literal of the variant
- GIVEN the files under `crates/clm-verify/`
- WHEN they are read
- THEN for every `Body::Mutated {` in them, the text up to its matching `}` ends in `..` before that `}`, and `src/mutate.rs` calls `Body::mutated(`; the test that reads them builds the text it searches for so that it never appears whole in its own file

#### Scenario: the record is unchanged
- GIVEN a passing check and `a/x` due, with a fake `mutate` that writes `a/x killed`
- WHEN `clm verify --due` runs
- THEN the ledger gains a `mutated` record with `a/x` under `killed`, as before the move

#### Serves
- architecture/proven-by-tests

#### Needs
- ledger/mutated-constructor
