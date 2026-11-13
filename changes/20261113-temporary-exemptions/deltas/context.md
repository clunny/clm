---
component: context
base: {}
---

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
