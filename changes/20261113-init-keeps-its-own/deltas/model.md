---
component: model
base: { contract: "004eda88b150f756" }
---

## MODIFIED Contract
Parses `specs/` and `changes/` (see docs/spec-format.md) into a validated spec graph of components, requirements, assets, ownership tree and dependency DAG, with a content hash per element. A component's assets are the files under `specs/<component path>/assets/` (`node-assets`). A requirement exposes the requirements of other components it needs built first, from its `#### Needs` block (`requirement-needs`). The graph exposes the component frontmatter keys that live changes declare in `frontmatter_keys`, and the check a component's frontmatter must pass given them, so `apply` validates a delta's frontmatter by the same rule (`declared-frontmatter-keys`). The model exposes its component parser and its check of the rules across components, both of which loading runs, so `apply` checks the graph a change would leave by the same rules (`graph-check`). A component may name its test files with `tests`, globs it owns wherever they sit (`test-ownership-keys`), and the rules its files are temporarily exempt from with `exemptions`, each naming the backlog item that ends it (`exemptions`). The model exposes its check of an `exemptions` value, which loading runs, so `init` judges the entries of a proposed component map by the same rule (`exemptions`). The graph exposes the root's `outside` globs, the code no component owns yet, absent when the root doesn't set the key; `check`'s path ownership reads them, and runs only when the key is set (`incremental-adoption`). The graph also holds the two human layers, the entries of `ARCHITECTURE.md` and `CAPABILITIES.md` (`human-layers`), as requirements with IDs `architecture/<slug>` and `capability/<slug>`, which `Graph::requirement` finds and `Graph::requirements` lists after every component's, and the heading each layer's file writes its entries under, which `rename` rewrites; `capability` and `architecture` are reserved component IDs (`reserved-ids`). A component's `serves`, and a requirement's `#### Serves` block, name the entries a rule serves; `Graph::effective_serves` gives a rule's (`serves`). The model loads from a directory on disk or from an in-memory set of files, with the same result, and builds for `wasm32-unknown-unknown` so the `web` Worker can use it.

## MODIFIED Requirements

### Requirement: exemptions <!-- base: 20ac503eb1bad92f -->
A component's frontmatter MAY set `exemptions`, a list of entries that each exempt some of the component's own files from one rule `clm check` judges code by, for a time (`temporary-exemptions`). An entry SHALL hold `rule`, one of `undeclared-imports`, `tests-cite-their-owner` and `skipped-tests`; `files`, a non-empty list of non-empty globs written as `paths` are; `until`, `backlog/<slug>` naming the backlog item whose work ends the exemption; and MAY hold `why`, one line saying why the rule can't be met yet. The model SHALL expose each component's entries in the order written, none when the key is absent. The model SHALL expose its check of an `exemptions` value, `clm_model::check_exemptions`: given the value as parsed YAML (a `&serde_yaml::Value`), it returns `Result<Vec<Exemption>, Vec<String>>`, the entries in the order written, or one message per problem, every problem of every entry and not only the first, each in the words loading reports: a message naming `exemptions` for a value that isn't a list, a null included, and a message naming `exemptions` and the entry's position for an entry that isn't a map, lacks `rule`, `files` or `until`, holds another key, names another rule, holds a `files` or `until` of the wrong shape, or holds a `why` that isn't a string. It judges one key's value and not a component's frontmatter, so it is not a frontmatter check that takes no component ID, which `optional-nodes` forbids. Loading and the model's frontmatter check (`optional-nodes`) SHALL judge a component's `exemptions` by that one check and no other rule, failing under `model/exemptions` with the file and each message the check gives, so a caller that holds entries outside a `SPEC.md`, as `clm survey --components` does (`init/survey-components`), refuses exactly what loading refuses. A glob that matches files other components own exempts nothing there, since an exemption covers only its own component's files. Whether `until` names an item that exists SHALL NOT be judged when loading, so the code fix that resolves the item doesn't fail on the exemption a spec change removes after it (`gate/exemption-names-its-item`, `check.health/exemptions-signal`).

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

#### Scenario: the exposed check agrees with loading
- GIVEN an `exemptions` value of two entries whose second has `until: backlog/a/b`
- WHEN the model's exemptions check runs on the value, and the model loads `specs/a/SPEC.md` holding it
- THEN the check returns one message naming `exemptions`, entry 2 and `until`, and loading fails under `model/exemptions` naming `specs/a/SPEC.md` with that same message

#### Scenario: a value that isn't a list
- GIVEN the `exemptions` value `3`, and again a null value
- WHEN the model's exemptions check runs on the value, and the model loads `specs/a/SPEC.md` holding it
- THEN for each the check returns one message naming `exemptions`, and loading fails under `model/exemptions` with that same message and reports nothing under `model/valid-frontmatter`

#### Scenario: a why that isn't a string
- GIVEN an `exemptions` value whose one entry is valid apart from `why: 3`
- WHEN the model's exemptions check runs on the value
- THEN it returns one message naming `exemptions`, entry 1 and `why`

#### Scenario: entries from the exposed check
- GIVEN the `exemptions` value `[{rule: undeclared-imports, files: ["tests/test_api.py"], until: backlog/x}]`
- WHEN the model's exemptions check runs on the value
- THEN it returns one exemption from `undeclared-imports` for `tests/test_api.py` until `backlog/x`

#### Serves
- architecture/exemptions-end
- capability/adopt-clm
