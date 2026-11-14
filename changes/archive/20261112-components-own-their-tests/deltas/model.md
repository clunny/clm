---
component: model
base: { contract: "9c947a8831e95b4f" }
---

## MODIFIED Contract
Parses `specs/` and `changes/` (see docs/spec-format.md) into a validated spec graph of components, requirements, assets, ownership tree and dependency DAG, with a content hash per element. A component's assets are the files under `specs/<component path>/assets/` (`node-assets`). A requirement exposes the requirements of other components it needs built first, from its `#### Needs` block (`requirement-needs`). The graph exposes the component frontmatter keys that live changes declare in `frontmatter_keys`, and the check a component's frontmatter must pass given them, so `apply` validates a delta's frontmatter by the same rule (`declared-frontmatter-keys`). The model exposes its component parser and its check of the rules across components, both of which loading runs, so `apply` checks the graph a change would leave by the same rules (`graph-check`). A component may name its test files with `tests`, globs it owns wherever they sit, and the components those files may import beyond its `depends_on` with `test_depends_on` (`test-ownership-keys`). The graph exposes the root's `outside` globs, the code no component owns yet, absent when the root doesn't set the key; `check`'s path ownership reads them, and runs only when the key is set (`incremental-adoption`). The graph also holds the two human layers, the entries of `ARCHITECTURE.md` and `CAPABILITIES.md` (`human-layers`), as requirements with IDs `architecture/<slug>` and `capability/<slug>`, which `Graph::requirement` finds and `Graph::requirements` lists after every component's, and the heading each layer's file writes its entries under, which `rename` rewrites; `capability` and `architecture` are reserved component IDs (`reserved-ids`). A component's `serves`, and a requirement's `#### Serves` block, name the entries a rule serves; `Graph::effective_serves` gives a rule's (`serves`). The model loads from a directory on disk or from an in-memory set of files, with the same result, and builds for `wasm32-unknown-unknown` so the `web` Worker can use it.

## ADDED Requirements

### Requirement: test-ownership-keys
A component's frontmatter MAY set `tests`, a list of globs written as `paths` are: the component's test files, which it owns wherever they sit, inside another component's `paths` included (`components-own-their-tests`); and `test_depends_on`, a list of component IDs: the components its test files may import beyond its `depends_on`, such as the one that holds the product's entry point or a shared test harness. The model SHALL expose both, in the order written, and each as empty when its key is absent. Loading SHALL fail, naming the file and the key, for a `tests` that isn't a list of non-empty strings or a `test_depends_on` that isn't a list of strings, a YAML null included, and the model's frontmatter check (`optional-nodes`), which `apply` runs on a delta, SHALL reject the same shapes. The model SHALL report, as a violation that keeps the component loaded (as `dependency-dag` does), a `test_depends_on` entry that names no component or the component itself. `test_depends_on` SHALL NOT count toward `dependency-dag`'s cycles: a component's tests may import a component that depends on it, since nothing imports a test.

#### Scenario: keys parsed
- GIVEN `specs/a/SPEC.md` with `tests: ["tests/a/**"]` and `test_depends_on: [b]`, and component `b`
- WHEN the model loads `specs/`
- THEN `a`'s tests are `tests/a/**`, its test dependencies are `b`, and loading reports nothing

#### Scenario: keys absent
- GIVEN `specs/a/SPEC.md` with neither key
- WHEN the model loads `specs/`
- THEN `a`'s tests and test dependencies are empty

#### Scenario: bad shape
- GIVEN `specs/a/SPEC.md` with `tests: "tests/**"`, `tests: [""]` or `test_depends_on:` with no value
- WHEN the model loads `specs/`
- THEN loading fails naming `specs/a/SPEC.md` and the key

#### Scenario: unknown or own component
- GIVEN `specs/a/SPEC.md` with `test_depends_on: [ghost]` or `test_depends_on: [a]`
- WHEN the model loads `specs/`
- THEN loading reports `specs/a/SPEC.md`, `test_depends_on` and the ID

#### Scenario: tests import a dependant
- GIVEN component `app` with `depends_on: [a]`, and `a` with `test_depends_on: [app]`
- WHEN the model loads `specs/`
- THEN loading reports no cycle

#### Serves
- architecture/path-ownership
- capability/work-concurrently
