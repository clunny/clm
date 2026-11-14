---
component: context
base: { contract: "52f4d7cea3e3a3f2", design: "4cf892d3ce5795f8" }
---

## MODIFIED Contract
`clm context <component>` deterministically prints the bundle an agent needs to work on a component: `ARCHITECTURE.md`, CLM's carried constraints in force in a project that isn't CLM's own (`carried-constraints`), the capabilities the component serves, the contract and design of every component among the ancestors from the root down to the component, the component's frontmatter, requirements and list of assets, and the contract of each component it depends on directly, through `depends_on`. `clm context <component> --hash` prints the bundle's content hash, so a caller can tell whether anything an agent working on the component would read has changed (`hash-based-staleness`); `clm rebase` uses it to report a spec that moved under a build (`sync/spec-changed`).

The bundle is also available without a checkout (`bundle_from`), from the graph and the text of `ARCHITECTURE.md`, with the same result: the graph holds `CAPABILITIES.md`'s entries and the carried constraints in force. The crate builds for `wasm32-unknown-unknown`, so the `web` Worker can use it through `orchestrator`.

## MODIFIED Design
Crate `clm-context` with `bundle(graph, root, node_id) -> String`, built on the `model` graph. It reads `ARCHITECTURE.md` from `root` and calls `bundle_from(graph, architecture: Option<&str>, node_id)`, which does the rest and touches no filesystem. The bundle is Markdown in this order:

1. `# ARCHITECTURE.md`, then the file's text (omitted if the file doesn't exist).
2. `# CLM's constraints`, the carried entries in force, outside the source only (`bundle-carries-constraints`).
3. `# Capabilities served`, then the block of each capability entry the target's requirements serve, verbatim: each requirement's effective serves (its `#### Serves` block, else the component's `serves`), each entry once, in `CAPABILITIES.md`'s order (`served_capabilities`). A component `serves` entry that every requirement overrides, or that a component without requirements names, adds nothing. Architecture IDs among them add nothing, since the whole of `ARCHITECTURE.md` is already there, and no other capability is included. The target that builds the capability tests gets every capability's block instead, in `CAPABILITIES.md`'s order: the graph's capability component (`model/reserved-ids`), which has no requirements of its own to serve them, or, in a graph without one, its root, the component `clm pending` places `CAPABILITIES.md`'s entries on, so a repository without the capability component gives the component that writes those tests the same bundle. The section is omitted when the target serves none.
4. `# Ancestors`, then for each component from the root `clm` down to the target, the target last: `## <id>: <title>`, `### Contract`, `### Design`. The target also gets `### Frontmatter`, its YAML in a fenced `yaml` code block, before its contract, and `### Requirements`, its requirement blocks, after its design, then `### Assets`, one line `- specs/<path>/assets/<asset> <hash>` per asset sorted by path, when it has any.
5. `# Dependencies`, then for each component named in the target's `depends_on` that isn't among its ancestors, sorted by ID and each once: `## <id>: <title>` and `### Contract`. The section is omitted when there is none.

Every line of every part loses its trailing whitespace, and the parts are joined by a blank line. `bundle_nodes` lists the components whose specs make up the bundle: the ancestors, then the dependencies.

## MODIFIED Requirements

### Requirement: bundle-contents <!-- base: 88b3e0e8b89462c8 -->
`clm context <component>` SHALL print `ARCHITECTURE.md`, the block of each capability `<component>`'s requirements serve (each requirement's `#### Serves` block, else `<component>`'s `serves`; each once, in `CAPABILITIES.md`'s order), or every capability's block when `<component>` builds the capability tests (the graph's capability component, or its root when the graph has none), the contract and design of each component among the ancestors from the root to `<component>`, `<component>`'s frontmatter and requirements, the path and hash of each of `<component>`'s assets, and the contract of each component in `<component>`'s `depends_on`. It SHALL NOT print any other component's design, requirements or assets, any asset's content, any capability none of `<component>`'s requirements serves (unless `<component>` builds the capability tests), or anything from `changes/`, `backlog/` or `docs/`.

#### Scenario: dependency stays private
- GIVEN component `a` depending on `b`, where `b` has design text `B-DESIGN`, requirement `b/y` and asset `bm.html`
- WHEN `clm context a` runs
- THEN the output contains `b`'s contract and `a`'s requirements, and contains none of `B-DESIGN`, `b/y` or `bm.html`

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
- model/capability-tests-component
