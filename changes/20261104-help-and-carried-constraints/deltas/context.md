---
component: context
base: { contract: "4f0348a1c2deb924" }
---

## MODIFIED Contract
`clm context <component>` deterministically prints the bundle an agent needs to work on a component: `ARCHITECTURE.md`, CLM's carried constraints in force in a project that isn't CLM's own (`carried-constraints`), the capabilities the component serves, the contract and design of every component among the ancestors from the root down to the component, the component's frontmatter, requirements and list of assets, and the contract of each component it depends on directly. `clm context <component> --hash` prints the bundle's content hash, so a caller can tell whether anything an agent working on the component would read has changed (`hash-based-staleness`); `clm rebase` uses it to report a spec that moved under a build (`sync/spec-changed`).

The bundle is also available without a checkout (`bundle_from`), from the graph and the text of `ARCHITECTURE.md`, with the same result: the graph holds `CAPABILITIES.md`'s entries and the carried constraints in force. The crate builds for `wasm32-unknown-unknown`, so the `web` Worker can use it through `orchestrator`.

## ADDED Requirements

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
