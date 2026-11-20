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
`clm context <component>` deterministically prints the bundle an agent needs to work on a component: `ARCHITECTURE.md`, CLM's carried constraints in force in a project that isn't CLM's own (`carried-constraints`), the capabilities the component serves, the contract and design of every component among the ancestors from the root down to the component, the component's frontmatter, requirements and list of assets, and the contract of each component it depends on directly, through `depends_on`. `clm context <component> --hash` prints the bundle's content hash, so a caller can tell whether anything an agent working on the component would read has changed (`hash-based-staleness`). It exits 0 with the bundle or its hash, 1 with the graph's violations on standard output and no bundle, and 2 naming the component on standard error when there is no such component (`bundle-errors`); `--help` prints the usage and exits 0, and arguments it can't parse, or a stream it can't write, exit 2 (`commands-in-crate`).

Crate `clm-context` offers the same to a caller that holds a graph (`bundle-components`): `bundle(graph, root, id)` is the bundle `clm context <id>` prints for the checkout at `root`, or nothing when the graph holds no such component, and doesn't judge the graph: for one with violations, where the command prints no bundle, it still gives the text; `hash(bundle)` is what `--hash` prints for it; `bundle_nodes(graph, id)` is the IDs of the components whose specs the bundle holds; and `served_capabilities(graph, component)` is the capability entries whose blocks it holds. `clm rebase` reads all four to report a spec that moved under a build and which files moved (`sync/spec-changed`). `bundle_from(graph, architecture, id)` gives the same bundle without a checkout, from the graph and the text of `ARCHITECTURE.md`, or none: the graph holds `CAPABILITIES.md`'s entries and the carried constraints in force (`bundle-from-text`). The crate builds for `wasm32-unknown-unknown` (`context-builds-for-wasm`), where it offers everything but the command's call.

`clm-context` offers `clm context` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup, of which this command reads nothing, and a stream each for standard output and standard error, and returns the exit status. `Args` is the command's arguments, which `crates/clm`'s parser reads. `crates/clm` routes the command to the call and adds nothing to what it writes.

## Design
Crate `clm-context` with `bundle(graph, root, node_id) -> Option<String>`, built on the `model` graph. It reads `ARCHITECTURE.md` from `root` and calls `bundle_from(graph, architecture: Option<&str>, node_id) -> Option<String>`, which does the rest and touches no filesystem. It gathers the bundle's parts in order (`bundle-deterministic`), each heading and each copied text a part of its own, trims each and joins them once with a blank line. Architecture IDs among a rule's serves add nothing to the served capabilities, since the whole of `ARCHITECTURE.md` is already there.

The command's call and its `Args` are compiled for native targets only, since the argument parser isn't built for wasm.

## Requirements

### Requirement: bundle-contents
`clm context <component>` SHALL print `ARCHITECTURE.md`, the block of each capability `<component>`'s requirements serve (each requirement's `#### Serves` block, else `<component>`'s `serves`; each once, in `CAPABILITIES.md`'s order), or every capability's block when `<component>` builds the capability tests (`model/capability-tests-component`), the contract and design of each component among the ancestors from the root to `<component>`, `<component>`'s frontmatter and requirements, the path and hash of each of `<component>`'s assets, and the contract of each component in `<component>`'s `depends_on` that isn't among its ancestors. The ancestors' section SHALL be headed `# Ancestors`. When the checkout has no `ARCHITECTURE.md`, the bundle SHALL hold no `# ARCHITECTURE.md` section; when `<component>` serves no capability, no `# Capabilities served` section; and when no dependency is left to print, no `# Dependencies` section. It SHALL NOT print any other component's design, requirements or assets, any asset's content, any capability none of `<component>`'s requirements serves (unless `<component>` builds the capability tests), or anything from `changes/`, `backlog/` or `docs/`.

#### Scenario: dependency stays private
- GIVEN component `a` depending on `b`, where `b` has design text `B-DESIGN`, requirement `b/y` and asset `bm.html`
- WHEN `clm context a` runs
- THEN the output contains `b`'s contract and `a`'s requirements, and contains none of `B-DESIGN`, `b/y` or `bm.html`

#### Scenario: nested component sees its ancestors
- GIVEN component `a.b` under `a` under root `clm`
- WHEN `clm context a.b` runs
- THEN the output contains the contracts and designs of `clm`, `a` and `a.b`, in that order

#### Scenario: a nested component
- GIVEN components `a` and `a.b`
- WHEN `clm context a.b` runs
- THEN its output holds the line `# Ancestors`, followed by `## clm: <title>`, then `a`'s and then `a.b`'s sections

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

#### Scenario: no architecture file
- GIVEN a checkout with component `a` and no `ARCHITECTURE.md`
- WHEN `clm context a` runs
- THEN the output holds no line `# ARCHITECTURE.md`

#### Needs
- model/capability-tests-component

### Requirement: bundle-deterministic
`clm context` SHALL print byte-identical output for the same repo content, listing the ancestors from root to target and dependencies sorted by ID. The output SHALL be these parts in this order, each heading a line of its own: `# ARCHITECTURE.md` and the file's text, when the file exists; `# CLM's constraints` and its section, when the graph isn't the source of the carried constraints (`bundle-carries-constraints`); `# Capabilities served` and each served block, when any is served; `# Ancestors`, then for each component from the root down to `<component>`, `## <id>: <title>`, `### Contract` and its Contract, `### Design` and its Design, with, for `<component>` alone, `### Frontmatter` and its frontmatter in a fenced `yaml` code block before `### Contract`, and after its Design `### Requirements` and each requirement block and, when it has assets, `### Assets` and one line `- <the directory of the component's SPEC.md>/assets/<asset path> <hash>` per asset, in the assets' path order; and `# Dependencies`, when a dependency is left that isn't an ancestor, then `## <id>: <title>`, `### Contract` and its Contract for each. Each part SHALL be printed without the whitespace at the end of each of its lines and at its own end, so no line of the output ends in whitespace; each part but the last SHALL be followed by one blank line, so two parts next to each other have exactly one between them unless the second starts with blank lines of its own, and a part with no text, such as an empty Contract, still takes its place and its blank line; and the last part SHALL be followed by a line break.

#### Scenario: repeated runs
- GIVEN component `a` depending on `c` and `b`
- WHEN `clm context a` runs twice
- THEN both outputs are identical and `b`'s contract comes before `c`'s

#### Scenario: the layout
- GIVEN component `a` whose Contract's lines end in spaces and whose assets are `mockups/m.html` and `b.png`
- WHEN `clm context a` runs
- THEN no line of the output ends in a space, the output ends with a line break, and it holds `### Assets`, one blank line, `- specs/a/assets/b.png <hash>` and then `- specs/a/assets/mockups/m.html <hash>`

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
When `<component>` doesn't exist, `clm context` SHALL write the line ``clm context: no component `<component>` `` to standard error and nothing to standard output, and exit 2. When the spec graph has violations, it SHALL write each to standard output as the line `model/violation-line` gives for it, write nothing to standard error and no bundle, and exit 1, whether or not `<component>` exists.

#### Scenario: unknown node
- GIVEN no component `zz`
- WHEN `clm context zz` runs
- THEN it exits 2, standard error is the line ``clm context: no component `zz` `` and standard output is empty

#### Scenario: invalid graph
- GIVEN component `a`, and component `b` whose `depends_on` names a component that doesn't exist
- WHEN `clm context a` runs
- THEN it exits 1, standard output holds one line per violation, the `model/dependency-dag` one among them, and nothing of `a`'s contract, and standard error is empty

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

### Requirement: bundle-carries-constraints
When the graph isn't the source of the carried constraints (`model/carried-constraints`), `clm context <component>` SHALL print, after `ARCHITECTURE.md`'s text (or first, when the file is absent) and before `# Capabilities served`, a section `# CLM's constraints`: one line saying these are the constraints CLM carries to every project, from the `clm` that runs, and that `clm guide constraints` explains their tiers; then, for each carried entry in force, in the carried order, `### Constraint: <slug> (<tier>)`, its statement and, for a `replaceable` entry, a line `Guarantee: <text>`. A carried entry the project's `ARCHITECTURE.md` replaces SHALL NOT appear. In the source the section SHALL NOT be printed, since its `ARCHITECTURE.md` already holds the entries. The section's first line SHALL also say that names in backticks it doesn't list are CLM's own decisions and constraints, kept in CLM's repository.

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

### Requirement: commands-in-crate
`clm-context` SHALL offer `clm context` as a call of its own that takes the command's arguments as the command line gives them, the repository root, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2, for `--help` it SHALL write the usage to the output stream and return 0, and when it can't write to a stream it SHALL return 2.

#### Scenario: a command called in a test
- GIVEN a fixture repository with component `a` and no component `zz`
- WHEN the call runs with the arguments `a`, then `a --hash`, then `zz`, each with two buffers
- THEN the first returns 0 with `a`'s bundle in the output buffer and an empty error buffer, the second returns 0 with the bundle's hash and a line end in the output buffer, and the third returns 2 with an empty output buffer and ``clm context: no component `zz` `` in the error buffer

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN it is passed an option `clm context` doesn't have
- THEN it returns 2 with the usage error, naming the option, in the error buffer and an empty output buffer, and the caller's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: bundle-components
`clm-context` SHALL offer, to a caller that holds a graph, what `clm context` prints and what the bundle is made of. `bundle(graph, root, id)` SHALL return the text `clm context <id>` prints for the checkout at `root` when its graph has no violation, the same text for the graph it is given whether or not that graph has one, and nothing when the graph holds no component `id`. `hash(bundle)` SHALL return the hash `clm context <id> --hash` prints for that bundle, without the line's end. `bundle_nodes(graph, id)` SHALL return the IDs of the components whose specs the bundle of `id` holds: its ancestors from the root down to `id` itself, then the components its `depends_on` names that aren't among them, in ascending order of their IDs as strings and each once; and an empty list when the graph holds no component `id`. `served_capabilities(graph, component)` SHALL return the capability entries whose blocks that component's bundle holds under `# Capabilities served` (`bundle-contents`), in the bundle's order, and none when the bundle has no such section.

#### Scenario: ancestors then dependencies
- GIVEN components `p`, `a` and `z`, and `p.b` under `p` with `depends_on: [p, z, a, z]`
- WHEN `bundle_nodes` runs for `p.b`, and for `missing`, which names no component
- THEN the first returns the root's ID, `p`, `p.b`, `a` and `z`, in that order, and the second returns an empty list

#### Scenario: the calls give what the command prints
- GIVEN a fixture repository with component `a`
- WHEN `bundle` runs for `a`, `hash` runs on what it returns, and `clm context a` and `clm context a --hash` run
- THEN `clm context a` prints exactly the text `bundle` returned, and `clm context a --hash` prints what `hash` returned followed by a line end

#### Scenario: no such component
- GIVEN a graph with no component `zz`
- WHEN `bundle` runs for `zz`
- THEN it returns nothing

#### Scenario: the served entries
- GIVEN `CAPABILITIES.md` with entries `a`, `b` and `c`, and component `n` whose requirements serve `capability/b` and `capability/a`
- WHEN `served_capabilities` runs for `n`
- THEN it returns the entries `capability/a` and `capability/b`, in that order

#### Serves
- capability/build-from-spec
- architecture/hash-based-staleness
