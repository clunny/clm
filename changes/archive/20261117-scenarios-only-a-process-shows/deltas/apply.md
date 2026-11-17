---
component: apply
---

## MODIFIED Requirements



### Requirement: hash-command <!-- base: 971912c5f97f8b14 -->
`clm hashes <node-id>` SHALL print the component's element hashes, and `clm hashes architecture` and `clm hashes capability` the layer's entry hashes, in file order, as the Contract describes. When component `capability` loads, `clm hashes capability` SHALL print its element hashes before the entries', so a delta for that component can record its bases. A component's hashes SHALL equal the hashes `validate` compares bases against; an entry's is the version tests cite, since no delta edits a human layer. That `clm hash` is no command is the root's to state and prove, since its one parser holds the command names (`clm/old-command-names-retired`). Every message CLM prints that points to the command, such as `apply/stale-base`'s for a missing base, SHALL name `clm hashes`.

#### Scenario: hash then modify
- GIVEN the hash `clm hashes a` prints for `requirement:r`
- WHEN a delta records it as `r`'s base and modifies `r`
- THEN validation reports no staleness

#### Scenario: hashes of a layer
- GIVEN a `CAPABILITIES.md` with entries `a` and `b`, and an `ARCHITECTURE.md` with entry `x`
- WHEN `clm hashes capability` and `clm hashes architecture` run
- THEN the first prints `capability:a <hash>` and `capability:b <hash>`, and the second `requirement:x <hash>`, each the entry's hash

#### Scenario: hashes of the capability node
- GIVEN a `CAPABILITIES.md` with entry `a`, and a component `capability` with frontmatter, a Contract and a Design
- WHEN `clm hashes capability` runs
- THEN it prints `frontmatter`, `contract` and `design` lines, then `capability:a <hash>`
- AND a delta for component `capability` that records the printed `design` hash as its base and modifies its Design reports no staleness

#### Serves
- capability/prove-by-tests
- architecture/hash-based-staleness
