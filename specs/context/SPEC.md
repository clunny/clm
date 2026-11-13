---
title: Context
status: active
owners: ["@clunny"]
depends_on: [model]
paths: ["crates/clm-context/**"]
successor: ""
serves: [capability/build-from-spec, architecture/ancestor-scoped-context]
---

## Contract
`clm context <component>` deterministically prints the bundle an agent needs to work on a component: `ARCHITECTURE.md`, CLM's carried constraints in force in a project that isn't CLM's own (`carried-constraints`), the capabilities the component serves, the contract and design of every component among the ancestors from the root down to the component, the component's frontmatter, requirements and list of assets, and the contract of each component it depends on directly, through `depends_on` or `test_depends_on`. `clm context <component> --hash` prints the bundle's content hash, so a caller can tell whether anything an agent working on the component would read has changed (`hash-based-staleness`); `clm rebase` uses it to report a spec that moved under a build (`sync/spec-changed`).

The bundle is also available without a checkout (`bundle_from`), from the graph and the text of `ARCHITECTURE.md`, with the same result: the graph holds `CAPABILITIES.md`'s entries and the carried constraints in force. The crate builds for `wasm32-unknown-unknown`, so the `web` Worker can use it through `orchestrator`.

## Design
Crate `clm-context` with `bundle(graph, root, node_id) -> String`, built on the `model` graph. It reads `ARCHITECTURE.md` from `root` and calls `bundle_from(graph, architecture: Option<&str>, node_id)`, which does the rest and touches no filesystem. The bundle is Markdown in this order:

1. `# ARCHITECTURE.md`, then the file's text (omitted if the file doesn't exist).
2. `# CLM's constraints`, the carried entries in force, outside the source only (`bundle-carries-constraints`).
3. `# Capabilities served`, then the block of each capability entry the target's requirements serve, verbatim: each requirement's effective serves (its `#### Serves` block, else the component's `serves`), each entry once, in `CAPABILITIES.md`'s order (`served_capabilities`). A component `serves` entry that every requirement overrides, or that a component without requirements names, adds nothing. Architecture IDs among them add nothing, since the whole of `ARCHITECTURE.md` is already there, and no other capability is included. The target that builds the capability tests gets every capability's block instead, in `CAPABILITIES.md`'s order: the graph's capability component (`model/reserved-ids`), which has no requirements of its own to serve them, or, in a graph without one, its root, the component `clm pending` places `CAPABILITIES.md`'s entries on, so a repository without the capability component gives the component that writes those tests the same bundle. The section is omitted when the target serves none.
4. `# Ancestors`, then for each component from the root `clm` down to the target, the target last: `## <id>: <title>`, `### Contract`, `### Design`. The target also gets `### Frontmatter`, its YAML in a fenced `yaml` code block, before its contract, and `### Requirements`, its requirement blocks, after its design, then `### Assets`, one line `- specs/<path>/assets/<asset> <hash>` per asset sorted by path, when it has any.
5. `# Dependencies`, then for each component named in the target's `depends_on` or `test_depends_on` that isn't among its ancestors, sorted by ID and each once: `## <id>: <title>` and `### Contract`. The section is omitted when there is none.

Every line of every part loses its trailing whitespace, and the parts are joined by a blank line. `bundle_nodes` lists the components whose specs make up the bundle: the ancestors, then the dependencies.

## Requirements

### Requirement: bundle-contents
`clm context <component>` SHALL print `ARCHITECTURE.md`, the block of each capability `<component>`'s requirements serve (each requirement's `#### Serves` block, else `<component>`'s `serves`; each once, in `CAPABILITIES.md`'s order), or every capability's block when `<component>` builds the capability tests (the graph's capability component, or its root when the graph has none), the contract and design of each component among the ancestors from the root to `<component>`, `<component>`'s frontmatter and requirements, the path and hash of each of `<component>`'s assets, and the contract of each component in `<component>`'s `depends_on` or its `test_depends_on` (`model/test-ownership-keys`), each once, since its tests lean on the latter's. It SHALL NOT print any other component's design, requirements or assets, any asset's content, any capability none of `<component>`'s requirements serves (unless `<component>` builds the capability tests), or anything from `changes/`, `backlog/` or `docs/`.

#### Scenario: dependency stays private
- GIVEN component `a` depending on `b`, where `b` has design text `B-DESIGN`, requirement `b/y` and asset `bm.html`
- WHEN `clm context a` runs
- THEN the output contains `b`'s contract and `a`'s requirements, and contains none of `B-DESIGN`, `b/y` or `bm.html`

#### Scenario: test dependency's contract
- GIVEN component `feat` with `test_depends_on: [app]` and no `depends_on`
- WHEN `clm context feat` runs
- THEN it prints `app`'s contract and not its design

#### Scenario: nested component sees its ancestors
- GIVEN component `a.b` under `a` under root `clm`
- WHEN `clm context a.b` runs
- THEN the output contains the contracts and designs of `clm`, `a` and `a.b`, in that order

#### Scenario: assets listed
- GIVEN component `a` with asset `mockups/m.html`
- WHEN `clm context a` runs, and then again after `m.html` changes
- THEN the output lists `specs/a/assets/mockups/m.html` with its hash but not its content, and `clm context a --hash` differs between the two runs

#### Scenario: served capabilities
- GIVEN `CAPABILITIES.md` with entries `a`, `b` and `c`, and component `n` with `serves: [capability/a, architecture/x]` and a requirement whose `#### Serves` block names `capability/b`
- WHEN `clm context n` runs
- THEN the output holds `a`'s and `b`'s blocks, in that order, after `ARCHITECTURE.md` and before the ancestors, and nothing of `c`

#### Scenario: none served
- GIVEN component `n` that serves no capability
- WHEN `clm context n` runs
- THEN the output has no `# Capabilities served` section

#### Scenario: capability node
- GIVEN `CAPABILITIES.md` with entries `a` and `b`, and the capability component `capability` with no `serves`
- WHEN `clm context capability` runs
- THEN the output holds `a`'s and `b`'s blocks, in that order, under `# Capabilities served`

#### Scenario: root without a capability node
- GIVEN `CAPABILITIES.md` with entries `a` and `b`, no `specs/capability/SPEC.md`, and root component `r` with `serves: [capability/a]`
- WHEN `clm context r` runs
- THEN the output holds `a`'s and `b`'s blocks, in that order, under `# Capabilities served`

#### Scenario: overridden node default
- GIVEN component `n` with `serves: [capability/a]` whose only requirement's `#### Serves` block names `capability/b`
- WHEN `clm context n` runs
- THEN the output holds `b`'s block and nothing of `a`

#### Needs
- model/test-ownership-keys
- model/capability-tests-component

### Requirement: bundle-deterministic
`clm context` SHALL print byte-identical output for the same repo content, listing the ancestors from root to target and dependencies sorted by ID.

#### Scenario: repeated runs
- GIVEN component `a` depending on `c` and `b`
- WHEN `clm context a` runs twice
- THEN both outputs are identical and `b`'s contract comes before `c`'s

### Requirement: bundle-hash
`clm context <component> --hash` SHALL print the content hash of the bundle `clm context <component>` prints. The hash SHALL change when any text in the bundle changes, and SHALL NOT change when text outside it changes.

#### Scenario: dependency design edit
- GIVEN component `a` depending on `b`
- WHEN `b`'s design changes
- THEN `clm context a --hash` prints the same hash as before

#### Scenario: dependency contract edit
- GIVEN component `a` depending on `b`
- WHEN `b`'s contract changes
- THEN `clm context a --hash` prints a different hash

#### Scenario: capability edit
- GIVEN component `n` serving capability `a` and not `c`
- WHEN `a`'s text changes, and then `c`'s
- THEN `clm context n --hash` changes the first time and not the second

#### Serves
- capability/build-from-spec
- architecture/hash-based-staleness
- architecture/ancestor-scoped-context

### Requirement: bundle-errors
`clm context` SHALL exit 2 naming the component when `<component>` doesn't exist, and SHALL print the violations and exit 1 when the spec graph has violations, printing no bundle.

#### Scenario: unknown node
- GIVEN no component `zz`
- WHEN `clm context zz` runs
- THEN it exits 2 and its error names `zz`

### Requirement: bundle-from-text
`bundle_from` SHALL return, from a graph and the text of `ARCHITECTURE.md`, the same bundle `clm context` prints for a checkout with that graph and that file.

#### Scenario: same as checkout
- GIVEN a checkout with `ARCHITECTURE.md` and a component `a`
- WHEN `bundle_from` runs on its graph and the file's text, and `clm context a` runs on the checkout
- THEN the two outputs are equal

#### Serves
- capability/follow-from-browser
- architecture/web-optional

### Requirement: context-builds-for-wasm
The `clm-context` crate SHALL build for `wasm32-unknown-unknown`.

#### Scenario: wasm build
- GIVEN the workspace with the `wasm32-unknown-unknown` target installed
- WHEN `cargo build -p clm-context --target wasm32-unknown-unknown` runs
- THEN it succeeds

#### Serves
- capability/follow-from-browser
- architecture/web-optional

### Requirement: says-component
The bundle's section of the components from the root down to the target SHALL be headed `# Ancestors`, and the bundle's own headings and text SHALL hold no old word, as `component-words` defines them; the specs, `ARCHITECTURE.md` and capabilities it copies are copied as they are. A test reads the string literals in `crates/clm-context/src/` and fails naming the file, the line and the word for each old word `component-words` doesn't keep.

#### Scenario: a nested component
- GIVEN components `a` and `a.b`
- WHEN `clm context a.b` runs
- THEN its output holds the line `# Ancestors`, followed by `## clm: <title>`, then `a`'s and then `a.b`'s sections

#### Serves
- architecture/plain-terms

### Requirement: bundle-carries-constraints
When the graph isn't the source of the carried constraints (`model/carried-constraints`), `clm context <component>` SHALL print, after `ARCHITECTURE.md`'s text (or first, when the file is absent) and before `# Capabilities served`, a section `# CLM's constraints`: one line saying these are the constraints CLM carries to every project, from the `clm` that runs, and that `clm guide constraints` explains their tiers; then, for each carried entry in force, in the carried order, `### Constraint: <slug> (<tier>)`, its statement and, for a `replaceable` entry, a line `Guarantee: <text>`. A carried entry the project's `ARCHITECTURE.md` replaces SHALL NOT appear. In the source the section SHALL NOT be printed, since its `ARCHITECTURE.md` already holds the entries. `bundle_from` SHALL print the same, from the graph. The section's first line SHALL also say that names in backticks it doesn't list are CLM's own decisions and constraints, kept in CLM's repository. A `clm` whose carried text differs gives a project a different bundle hash, which is meant: what a build reads has changed (`sync/spec-changed`). A test whose scenario asserts the compiled copy's entries holds a `clm:reads` marker naming `ARCHITECTURE.md` (`doc-readers-marked`).

#### Scenario: a project
- GIVEN a repository that isn't the source, whose `ARCHITECTURE.md` entry `x` replaces `expand-contract`, and component `a`
- WHEN `clm context a` runs
- THEN the output holds `# CLM's constraints` after `x`'s entry and before `# Ancestors`, holding `### Constraint: proven-by-tests (method)` and `### Constraint: yagni (default)` and no `expand-contract` heading

#### Scenario: the source
- GIVEN a repository whose `ARCHITECTURE.md` entry `a` holds `#### Carries` `- method`, and component `n`
- WHEN `clm context n` runs
- THEN the output holds no `# CLM's constraints`

#### Needs
- model/carried-constraints

#### Serves
- architecture/carried-constraints
- architecture/ancestor-scoped-context
