---
component: context
base: { contract: "32d568442eb792b3", design: "1fafb7810002b1d2" }
---

## MODIFIED Contract
`clm context <component>` deterministically prints the bundle an agent needs to work on a component: `ARCHITECTURE.md`, CLM's carried constraints in force in a project that isn't CLM's own (`carried-constraints`), the capabilities the component serves, the contract and design of every component among the ancestors from the root down to the component, the component's frontmatter, requirements and list of assets, and the contract of each component it depends on directly, through `depends_on`. `clm context <component> --hash` prints the bundle's content hash, so a caller can tell whether anything an agent working on the component would read has changed (`hash-based-staleness`). It exits 0 with the bundle or its hash, 1 with the graph's violations on standard output and no bundle, and 2 naming the component on standard error when there is no such component (`bundle-errors`); `--help` prints the usage and exits 0, and arguments it can't parse, or a stream it can't write, exit 2 (`commands-in-crate`).

Crate `clm-context` offers the same to a caller that holds a graph (`bundle-components`): `bundle(graph, root, id)` is the bundle `clm context <id>` prints for the checkout at `root`, or nothing when the graph holds no such component, and doesn't judge the graph: for one with violations, where the command prints no bundle, it still gives the text; `hash(bundle)` is what `--hash` prints for it; `bundle_nodes(graph, id)` is the IDs of the components whose specs the bundle holds; and `served_capabilities(graph, component)` is the capability entries whose blocks it holds. `clm rebase` reads all four to report a spec that moved under a build and which files moved (`sync/spec-changed`).

`clm-context` offers `clm context` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup, of which this command reads nothing, and a stream each for standard output and standard error, and returns the exit status. `Args` is the command's arguments, which `crates/clm`'s parser reads. `crates/clm` routes the command to the call and adds nothing to what it writes.

## MODIFIED Design
Crate `clm-context` with `bundle(graph, root, node_id) -> Option<String>`, built on the `model` graph. It reads `ARCHITECTURE.md` from `root`, the one file it reads, and builds the rest from the graph, which holds `CAPABILITIES.md`'s entries and the carried constraints in force. It gathers the bundle's parts in order (`bundle-deterministic`), each heading and each copied text a part of its own, trims each and joins them once with a blank line. Architecture IDs among a rule's serves add nothing to the served capabilities, since the whole of `ARCHITECTURE.md` is already there.

The crate is built for the native targets alone: the web app links none of it, and nothing builds it for `wasm32-unknown-unknown`.

## REMOVED Requirements

### Requirement: bundle-from-text <!-- base: 418dfef82802fc9c -->
### Requirement: context-builds-for-wasm <!-- base: da5bbf1b949af62b -->
