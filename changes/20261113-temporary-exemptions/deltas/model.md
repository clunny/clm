---
component: model
base: { contract: "5e5d5a618882810c" }
---

## MODIFIED Contract
Parses `specs/` and `changes/` (see docs/spec-format.md) into a validated spec graph of components, requirements, assets, ownership tree and dependency DAG, with a content hash per element. A component's assets are the files under `specs/<component path>/assets/` (`node-assets`). A requirement exposes the requirements of other components it needs built first, from its `#### Needs` block (`requirement-needs`). The graph exposes the component frontmatter keys that live changes declare in `frontmatter_keys`, and the check a component's frontmatter must pass given them, so `apply` validates a delta's frontmatter by the same rule (`declared-frontmatter-keys`). The model exposes its component parser and its check of the rules across components, both of which loading runs, so `apply` checks the graph a change would leave by the same rules (`graph-check`). A component may name its test files with `tests`, globs it owns wherever they sit (`test-ownership-keys`), and the rules its files are temporarily exempt from with `exemptions`, each naming the backlog item that ends it (`exemptions`). The graph exposes the root's `outside` globs, the code no component owns yet, absent when the root doesn't set the key; `check`'s path ownership reads them, and runs only when the key is set (`incremental-adoption`). The graph also holds the two human layers, the entries of `ARCHITECTURE.md` and `CAPABILITIES.md` (`human-layers`), as requirements with IDs `architecture/<slug>` and `capability/<slug>`, which `Graph::requirement` finds and `Graph::requirements` lists after every component's, and the heading each layer's file writes its entries under, which `rename` rewrites; `capability` and `architecture` are reserved component IDs (`reserved-ids`). A component's `serves`, and a requirement's `#### Serves` block, name the entries a rule serves; `Graph::effective_serves` gives a rule's (`serves`). The model loads from a directory on disk or from an in-memory set of files, with the same result, and builds for `wasm32-unknown-unknown` so the `web` Worker can use it.

## MODIFIED Requirements

### Requirement: test-ownership-keys <!-- base: baebdbff58f9bc03 -->
A component's frontmatter MAY set `tests`, a list of globs written as `paths` are: the component's test files, which it owns wherever they sit, inside another component's `paths` included (`components-own-their-tests`). A component's test files import only what its code may (`depends_on`): a test that needs the whole product belongs to the component that holds the product's entry point (`temporary-exemptions`). The model SHALL expose `tests` in the order written, empty when the key is absent. Loading SHALL fail, naming the file and the key, for a `tests` that isn't a list of non-empty strings, a YAML null included, and the model's frontmatter check (`optional-nodes`), which `apply` runs on a delta, SHALL reject the same shape.

#### Scenario: keys parsed
- GIVEN `specs/a/SPEC.md` with `tests: ["tests/a/**"]`
- WHEN the model loads `specs/`
- THEN `a`'s tests are `tests/a/**`, and loading reports nothing

#### Scenario: keys absent
- GIVEN `specs/a/SPEC.md` with no `tests`
- WHEN the model loads `specs/`
- THEN `a`'s tests are empty

#### Scenario: bad shape
- GIVEN `specs/a/SPEC.md` with `tests: "tests/**"`, `tests: [""]` or `tests:` with no value
- WHEN the model loads `specs/`
- THEN loading fails naming `specs/a/SPEC.md` and the key

#### Serves
- architecture/path-ownership
- capability/work-concurrently

## ADDED Requirements

### Requirement: test-dependencies-retired
`test_depends_on` SHALL be a key the model doesn't know, refused as any unknown key is (`valid-frontmatter`), and the graph SHALL expose no test dependencies, since a component's tests import only what its code may (`temporary-exemptions`). It is removed only once no component's code reads it (`coupled-changes-sequence`); until then the model's `test_depends_on` field and its accessor carry the marker `clm:remove-by 20261113-temporary-exemptions` (`expand-contract`).

#### Scenario: test dependencies retired
- GIVEN `specs/a/SPEC.md` with `test_depends_on: [b]`
- WHEN the model loads `specs/`
- THEN loading fails naming `specs/a/SPEC.md` and `test_depends_on`

#### Serves
- architecture/path-ownership

#### Needs
- check/undeclared-imports
- check.affected/affected-components
- check.affected/why-chain
- context/bundle-contents
- init/survey-components

### Requirement: exemptions
A component's frontmatter MAY set `exemptions`, a list of entries that each exempt some of the component's own files from one rule `clm check` judges code by, for a time (`temporary-exemptions`). An entry SHALL hold `rule`, one of `undeclared-imports`, `tests-cite-their-owner` and `skipped-tests`; `files`, a non-empty list of non-empty globs written as `paths` are; `until`, `backlog/<slug>` naming the backlog item whose work ends the exemption; and MAY hold `why`, one line saying why the rule can't be met yet. The model SHALL expose each component's entries in the order written, none when the key is absent. Loading SHALL fail, naming the file, `exemptions` and the entry's position, for an entry that isn't a map, lacks `rule`, `files` or `until`, holds another key, names another rule, or holds a `files` or `until` of the wrong shape, and the model's frontmatter check (`optional-nodes`) SHALL reject the same. A glob that matches files other components own exempts nothing there, since an exemption covers only its own component's files. Whether `until` names an item that exists SHALL NOT be judged when loading, so the code fix that resolves the item doesn't fail on the exemption a spec change removes after it (`gate/exemption-names-its-item`, `check.health/exemptions-signal`).

#### Scenario: an exemption parsed
- GIVEN `specs/a/SPEC.md` with `exemptions: [{rule: undeclared-imports, files: ["tests/test_api.py"], until: backlog/x, why: "the suite builds the whole app"}]`
- WHEN the model loads `specs/`
- THEN `a` holds one exemption from `undeclared-imports` for `tests/test_api.py` until `backlog/x`, and loading reports nothing

#### Scenario: a rule that can't be exempted
- GIVEN `specs/a/SPEC.md` with an exemption whose `rule` is `paths-disjoint`
- WHEN the model loads `specs/`
- THEN loading fails naming `specs/a/SPEC.md`, `exemptions` and `paths-disjoint`

#### Scenario: an exemption with no end
- GIVEN `specs/a/SPEC.md` with an exemption that has no `until`
- WHEN the model loads `specs/`
- THEN loading fails naming `specs/a/SPEC.md`, `exemptions` and `until`

#### Serves
- architecture/exemptions-end
- capability/adopt-clm
