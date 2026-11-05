# Spec format (v0, provisional)

How a CLM project's specs are written: components and their requirements with scenarios, the changes and deltas that edit them, decisions, the backlog and the two human layers.

This is the contract of the `model` component. It borrows OpenSpec's requirement/scenario and delta conventions, adds the Spec Growth Engine's component metadata, and replaces sequential IDs with slugs so that concurrent authors don't collide.

## Component spec: `specs/<component-path>/SPEC.md`

The component ID is the directory path under `specs/`, dot-separated (`specs/check/drift/SPEC.md` → `check.drift`). The root component is `specs/SPEC.md` with ID `clm`. The parent is implied by the path.

```markdown
---
title: Check
status: unbuilt          # unbuilt | active | deprecated
owners: ["@clunny"]      # who answers for the component (GitHub handles or names)
depends_on: [model]      # dependency edges (must form a DAG)
paths: ["crates/clm-check/**"]   # code this component owns (globs, disjoint across components)
serves: [capability/catch-drift]  # the human-layer entries its rules serve, unless a rule names its own (see The human layers)
successor: ""            # placeholder components: the slice that will flesh this out
optional: false          # optional; true makes this component and its descendants optional: no required component may depend on them (`optional-nodes`)
size_budget: { spec_tokens: 20000 }   # optional; this component's size budget in tokens, in place of clm.toml's `component_size` (`node-size-budgets`)
outside: ["src/web/**"]   # root only, optional; code no component owns yet, while a codebase is adopted a part at a time (`incremental-adoption`)
---

## Contract
What other components may rely on. Visible to dependants.

## Design
How it works inside. Visible to the implementer and parent only.

## Requirements

### Requirement: no-undeclared-deps
The check SHALL fail when code under a component's `paths` imports code owned by a component not listed in `depends_on`.

#### Scenario: undeclared import
- GIVEN component `a` with `depends_on: []`
- WHEN a file in `a.paths` imports a file in `b.paths`
- THEN `clm check` exits non-zero and names the file, the import and both components
```

### Rules

- A requirement's ID is `<component-id>/<slug>`, for example `check/no-undeclared-deps`. Slugs are kebab-case and unique within their component.
- Requirement text uses SHALL/MUST. Every requirement has at least one scenario.
- Frontmatter keys other than those above are rejected, unless a live change lists the key in its `change.md`'s `frontmatter_keys`: the change that adds a key's requirement declares it there and sets it in the same spec PR (`declared-frontmatter-keys`).
- **Needs** (`requirement-needs`). A requirement may hold one `#### Needs` block among its scenarios, each line `- <component>/<slug>` naming a requirement of another component that must be current before this one can be built. The model rejects a need naming no requirement, one on the same component, or a cycle. The queue treats a requirement whose needs aren't current like one a backlog item blocks (`requirement-blocks`): left out of its component's work until they are.
- **Path ownership** (`incremental-adoption`). Every tracked file outside `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` and `CLAUDE.md` is code, and, once the root sets `outside` (even to `[]`), `clm check` fails one that no component's `paths` match unless a glob of the root's `outside` does (`check/unowned-files`). `outside` names the code not yet adopted; a file a component's `paths` also match is the component's, and a glob that matches no file outside every component fails the check, so the list shrinks as later changes give that code components.
- **Children** (`nested-splits`). A component may hold children: `specs/<component>/<part>/SPEC.md` is component `<component>.<part>`, a full component with its own frontmatter, requirements and `paths` (still disjoint from every other component's). A child's builds receive its parent's Contract and Design with their ancestors (`ancestor-scoped-context`), and the views draw it inside its parent. A component is split by nesting, never into new siblings: see `docs/guide/shaping-the-graph.md` and `slim-components`.
- **Requirement status is derived, never written.** A requirement is *current* if a passing test cites it at its current version, *behind* if tests cite it only at older versions, and *unbuilt* if none cites it (`versioned-markers`). The last two are *pending*.
- Each element's **content hash** is computed from its normalized text: the requirement block, contract section, design section and frontmatter. These hashes are what deltas and test markers record (`hash-based-staleness`). A requirement's **version** is the first 8 hex digits of its hash, and leaves out its `#### Serves` block: the hash is that version followed by the last 8 hex digits of the whole block's hash, so a Serves-only edit makes deltas on the rule stale but no test `behind` (`serves-in-the-base`).

## The human layers: `ARCHITECTURE.md` and `CAPABILITIES.md`

Two files at the repository root hold what people decide: the architectural constraints, which the architect owns, and the capabilities, what a person can do with the product, which the product manager owns (`human-layers`). An edit to either needs review. Each starts with a short preamble, then one section holding the entries:

- `ARCHITECTURE.md`'s entries are the `### Constraint: <slug>` blocks inside its `## Constraints` section. Each uses SHALL or MUST and has at least one `#### Scenario:`.
- `CAPABILITIES.md`'s entries are the `### Capability: <slug>` blocks inside its `## Capabilities` section. Each states what someone can do, has exactly one `#### Why` block and at least one `#### Scenario:`.

```markdown
### Capability: approve-intent
A reviewer can read a change's intent and approve it.

#### Why
Only a human decides what CLM builds.

#### Scenario: approve
- GIVEN a spec branch whose change needs review
- WHEN the reviewer approves it
- THEN it lands
```

Each block runs to the next entry heading or to its section's end. Any other section, and the preamble, is prose the model doesn't read; an entry heading outside its section is an error, as are a duplicate slug, a `#### Needs` or `#### Serves` block in an entry, and an `#### Enforced by` block in a capability or naming no rule. A file that is absent, or has no such section, has no entries.

- **IDs and versions.** An entry's ID is `architecture/<slug>` or `capability/<slug>`; no top-level component may be named `architecture`, and a top-level component `capability` owns the capabilities (`reserved-ids`). Its version is the first 8 hex digits of its block's content hash, as for a rule. `clm hashes architecture` and `clm hashes capability` print them.
- **Proving a capability.** A capability is proven by a black-box test at the product's public boundary that cites it like a rule, `req:capability/approve-intent@<version>`: deterministic, using only what a user or client can touch, with what lies beyond the boundary (a model, the network) stubbed. The top-level component `capability`, whose requirements are `CAPABILITIES.md`'s entries and whose `SPEC.md` holds no requirements of its own, builds those tests; its Design names the project's boundary and harness. Until a project has that component, its root builds them. A capability may hold one `#### Untested` block, prose saying why no program can check it, which leaves it current and uncitable.
- **Proving an architectural constraint.** An architectural constraint is never cited by a test. It holds an `#### Enforced by` block of `- <component>/<slug>` lines naming the rules that enforce it (a whole-tree check for a property of every file, the one path and each blocked bypass for a choke point, the rules that state a local mechanism), an `#### Untested` block saying why no program can check it, or both for an entry that is part mechanism and part judgement. It is current when every rule it names is current, and never pending itself: its rules are built by their own components. `clm check` fails an entry with neither block (`check/unenforced`) and a test that cites one (`check/dangling-markers`) (`human-layer-proof`).
- **CLM's carried constraints** (`carried-constraints`). Some of CLM's own architectural constraints govern how any project is built with it. They reach every project from the `clm` it runs, never as a copy in its files: `clm context` adds those in force to every bundle and `clm guide constraints` prints them, each with its tier. A `method` constraint can't be replaced. A `replaceable` one states, under `Guarantee:`, what CLM relies on; a project may replace it with an entry of its own that keeps that guarantee. A `default` one is an opinion a project may drop. To replace or drop one, an entry of the project's `ARCHITECTURE.md` holds a `#### Replaces` block of `- <slug>` lines naming it, and is an ordinary constraint otherwise, with `#### Enforced by` or `#### Untested`. `clm check` fails a `#### Replaces` that names a `method` constraint or none at all, and an entry of the project's own that takes a carried constraint's slug. In CLM's own `ARCHITECTURE.md`, a carried entry holds a `#### Carries` block naming its tier, and a `replaceable` one a `#### Guarantee` block. A project set up by an older `clm init` may hold an `## Enforced by clm` list; it is prose the model doesn't read, and can be deleted.
- **What a rule serves** (`serves`). A component's frontmatter `serves` lists the entry IDs its rules serve. A requirement whose purpose differs from its component's holds one `#### Serves` block among its scenarios, shaped like `#### Needs` (`- <id>` lines), which replaces the component's list for that rule. Each ID must name an entry that exists. `serves` names entries without versions: rewording an entry leaves the rules that serve it current, and a `#### Serves` block is left out of its rule's version, though not its hash. Once either layer holds an entry, `clm check` fails a rule whose effective serves is empty (`check/untraced`); a repository whose layers hold none isn't checked.

## Component assets: `specs/<component-path>/assets/**`

Files a component's spec needs but can't hold as text, such as UI mockups or diagrams, live in its `assets/` directory (`node-assets`). An asset's ID is its path relative to `assets/`. Its hash is `content_hash` of its text when it is UTF-8, otherwise the first 8 bytes of the SHA-256 of its bytes. The spec refers to an asset as `` `assets/<path>` ``, and `clm check` fails a reference to an asset that doesn't exist. `assets` is never a component name, and the root component has no assets.

`clm context <component>` lists the component's assets with their hashes but not their contents, so a changed asset changes the component's bundle hash and `clm rebase` reports it to a build in progress.

## Test references

A test proves a requirement by containing the marker `req:<requirement-id>@<version>` in its name, attribute, tag or docstring, whichever its language makes natural. The version is the first 8 hex digits of the requirement's hash, as `clm hashes <component>` prints it: the marker says which text the test was written for (`versioned-markers`). Examples:

- Rust: `#[test] fn drift_undeclared_import() { // req:check/no-undeclared-deps@3fa2c9d1`
- Python: `def test_undeclared_import():  # req:check/no-undeclared-deps@3fa2c9d1`
- TypeScript: `it("rejects undeclared import [req:check/no-undeclared-deps@3fa2c9d1]", ...)`

The marker is always the literal text `req:<id>@<version>`, so `clm check` finds it without parsing any language. Markdown files are not scanned. Rules:

- A requirement is *current* when markers cite it and every one cites its current version, *behind* when a marker cites it at another version or without one, even beside one at the current version (`stale-markers-keep-behind`), and *unbuilt* when nothing cites it. A marker without a version still cites the requirement but never makes it current. The build of a `behind` requirement re-cites each stale marker, wherever its file sits.
- A change is complete, and is archived, when every requirement it added or modified is current and no marker cites one it removed (`archive-on-sync`).
- A marker citing a requirement that doesn't exist fails the check (`check/dangling-markers`), unless a change outside the archive removed the requirement: then the marker is *obsolete*, and the component's next build removes the code and tests it marks (`removed-requirements-build-out`). When the change moved the requirement to another component, the obsolete one is blocked until the requirement it moved to is current, since that component's build re-points the markers (`moved-markers-wait`).
- Every match counts, wherever it sits in a non-Markdown file: in a comment, a string or a fixture. A file that must hold a requirement ID after `req:` without citing it, such as a test that searches other files for citations or a fixture for a checker, builds the string so the literal never appears whole: `concat!("req", ":a/x")` in Rust, `"req" + ":a/x"` in Python or TypeScript. CLM has no escape or ignore comment for this, so no text can hide a real citation from the check; a bare match also fails `check/dangling-markers` when it names no requirement, and keeps one it names `behind`.
- `clm coverage` lists each requirement as `current` or `behind` (with its citing locations) or `unbuilt`, then each `obsolete` one. `clm pending [<component>]` lists the pending requirements with their versions and what blocks them.

"Passing" means the project's test suite passes in the checks `clm land` runs (`land-locally`). A skipped test in a file that holds a marker fails `clm check` (`check/skipped-tests`), so a skipped test never counts as proof.

## Changes: `changes/<YYYYMMDD-slug>/`

```
change.md          # intent, human_burdens, author, status
deltas/<component-id>.md   # one file per touched component
assets/<component-id>/**   # added or modified assets (see Component assets)
```

`change.md` frontmatter: `status: proposed|approved|applied|implementing|done|abandoned`. Optional: `human_burdens: [<line>, ...]` (what the change spends of the human's money or time, or can't undo, one short line each; a change that declares any needs review, `gate/needs-review`, and a spec branch that adds a `kind: human-task` backlog item must declare one, `gate/undeclared-burden`), `author`, `from: backlog/<slug>` (the item it was promoted from, deleted in the spec PR), `resolves: [<slug>, ...]` (backlog items it resolves, each deleted by the build that fixes it, or else when the change is archived, `task-deletes-fixed-items`, except a `kind: question` item, which `clm apply` deletes since the applied spec answers it, `apply-deletes-answered-questions`; an item the spec branch finds already fixed stays here too, and the branch deletes it, so `clm proposals` attributes its claim to the change) `priority: P1|P2|P3|P4|P5` (default `P5`; capped slots shared with the backlog, `priority-caps`) and `frontmatter_keys: [<key>, ...]` (component frontmatter keys the change adds, which the model accepts on any component while the change is live, so one spec PR can add a key's requirement and set the key, `declared-frontmatter-keys`). Changes are not ordered against each other: work is built component by component (`component-reconciliation`), `depends_on` orders nothing, a requirement's `#### Needs` and the backlog items that block it hold it back (`needs-alone-order`), and `priority` says what to build first. `after` and `prefer_after`, which changes recorded while changes were ordered against each other, are accepted and ignored. `gate`, the review tier changes recorded before review became yes or no (`yes-no-review`), is rejected on a live change and ignored on an archived one. The body is the intent (problem, outcome, constraints, open questions), as in the playbook's `intent.md`. It starts with `## Intent`, a few sentences on what the change does and why, which a reviewer reads first.

A delta file describes one component's changes using OpenSpec-style sections. Its frontmatter:

```yaml
component: check                 # component ID the delta applies to
create: false               # true: this delta creates the component
remove: false               # true: this delta removes the component (`node-removal`)
base: { frontmatter: "<hash>", contract: "<hash>", design: "<hash>" }   # only the elements it MODIFIES
```

Allowed sections (any other `##` section is an error):

| Section | Use | Needs base |
|---|---|---|
| `## Frontmatter` | `create: true` only: the whole frontmatter in a ```` ```yaml ```` fence | no |
| `## Contract`, `## Design` | `create: true` only: the section text | no |
| `## MODIFIED Frontmatter` | top-level keys to set, in a ```` ```yaml ```` fence; other keys untouched | `base.frontmatter` |
| `## MODIFIED Contract`, `## MODIFIED Design` | replacement section text | `base.contract` / `base.design` |
| `## ADDED Requirements` | full requirement blocks; slug must be new | no |
| `## MODIFIED Requirements` | full blocks, heading `### Requirement: <slug> <!-- base: <hash> -->`; a block without `#### Serves` keeps the rule's, and its base goes stale only when the rule's version moves (to change what it serves, give the block in full) | per requirement |
| `## REMOVED Requirements` | headings only, `### Requirement: <slug> <!-- base: <hash> -->` | per requirement |
| `## REMOVED Assets` | one line per asset, `` - `<path>` <!-- base: <hash> --> `` | per asset |

A delta with `remove: true` removes its component and holds only `REMOVED Requirements` and `REMOVED Assets`, which list every requirement and asset the component has. Its `base` records the component's `frontmatter`, `contract` and `design` hashes. A component with children is removed only with them, in the same change. A rename is a removal plus a `create: true` delta that adds the same requirements under the new ID (`node-removal`). The markers that cite the removed requirements are then `obsolete` on the component whose `paths` hold their files, or on the root when none does.

An added or modified asset is a file at `changes/<id>/assets/<component-id>/<path>`, which apply copies to the component's `assets/<path>`. A modified asset needs its current hash in the delta's `base.assets`, a map from path to hash (`base: { assets: { "mockups/m.html": "<hash>" } }`). A component that only changes assets still has a delta file, holding just its frontmatter.

Text in a frontmatter section outside its yaml fence is rationale, which apply ignores. Frontmatter values are single-line (flow style), because apply edits frontmatter line by line to preserve comments.

Get the hashes with `clm hashes <component-id>` **when you write the delta**. If a recorded base no longer matches `specs/`, the delta is **stale**: `clm check` fails it, and `clm apply` refuses it. The author (or Claude) then rebases it, much like a git rebase but per element.

`clm apply <change-id>` is the final step of a spec PR (per `apply-in-spec-pr`). It applies every delta or none, and sets the change's status to `applied`.

## Building: components, not changes

Nothing is planned (`node-reconciliation`). What is left to build is derived from the test markers: a component is *pending* while any of its requirements is behind, unbuilt or obsolete, and `clm pending [<component>]` lists them. A component is *ready* when it has pending work that neither a backlog item nor a pending requirement it `Needs` blocks; `depends_on` doesn't order builds (`needs-alone-order`). A capability also waits on each never-built (`unbuilt`) rule that serves it, since its test exercises them (`serving-rules-order-capabilities`). `clm next` lists the ready components, most urgent first; `clm claim <component>` claims one by pushing `impl/<component>` (`lease-claims`); `clm run <component>` claims and builds it in an unattended session. A build brings the component's code up to its whole spec, highest priority first, and cites each requirement it builds at its current version. It may stop at a green point when the list is long, leaving the rest pending.

Archived changes may hold a `plan.md` and `tasks/` from when changes were planned; nothing parses them.

## Archive: `changes/archive/<id>/`

A change whose status is `done` or `abandoned` is moved here with `clm archive <id>`, keeping its ID. `clm rebase` does it on any `impl/*` branch for a change that is complete: every requirement it added or modified current, and nothing citing one it removed (`archive-on-sync`). Archived changes are history. They are never edited (the gate blocks it), never validated against current specs, and never part of a context bundle. Their requirements' currency is tracked on the component, not the change.

## Decisions: `docs/decisions/<name>.md`

One file per decision. The file name is the decision's name.

```markdown
---
title: "No stacked PRs"
date: 2026-10-21
status: accepted          # accepted | superseded
superseded_by: other-name # only when superseded; must exist
---
The decision and why.
```

A decision is never rewritten. To reverse one, write a new decision and mark the old one `superseded` (a spec PR).

## Backlog: `backlog/<slug>.md`

Ideas, deferred work, open questions, steps only a human can take, bugs and workarounds. One file per item; the slug is its name.

```markdown
---
title: "Markers in skipped tests shouldn't count"
kind: deferred            # idea | deferred | question | bug | human-task | workaround
components: [check]            # components it concerns (must exist)
origin: 20261021-coverage # optional: the change during which it came up
priority: P3              # optional: P1..P5, default P5 (capped slots, see below)
blocks: [check/skipped-tests]   # optional: requirements that can't be built until this is resolved
---
Enough context to act on later.
```

- Any PR, including an AI-only code PR, may **add** backlog items (`fold-small-defer-large`), and may add requirements to an existing item's `blocks`. A code PR may also delete the items listed in `resolves` by a change it archives, or by a change outside the archive, as the build that fixes an item does (`task-deletes-fixed-items`), and a `kind: bug` or `kind: deferred` item naming a component whose `paths` its code changes, named by a commit trailer `Resolves: backlog/<slug>` (`code-fix-resolves-bug`, `deferred-work-resolves-in-code`). Any other edit or deletion is a spec change.
- An item that says when to delete it names a step its closer can take. A build may delete it only when a change's `resolves` lists it, or when it is `kind: deferred` and the build does its work (`deferred-work-resolves-in-code`); otherwise an item a build should close goes into the `resolves` of the change whose work it waits on, in the spec PR that adds the item or that change. An item no change lists, such as one about a change already archived, is deleted by a spec PR, and its body says so instead of asking a code PR to.
- **Blocks** (`requirement-blocks`). `blocks` names the requirements that can't be built until the item is resolved, usually a `question` or `human-task` item a build raised. A blocked requirement stays pending but is left out of its component's work, and a component whose pending requirements are all blocked holds up nothing above it. Deleting the item releases them.
- An item is promoted by creating a change with `from: backlog/<slug>` in its `change.md`, which deletes the item in the same spec PR.
- `clm backlog` lists the items, highest priority first.
- **Priority** (`priority-caps`). Items and changes may set `priority: P1` to `P5`; without one they are `P5`. Items and live changes share fixed slots: one `P1`, two `P2`, three `P3`, four `P4`, any number of `P5`. An item named in a live change's `resolves` isn't counted, since the change carries it. `clm check` reports an over-full priority as `check/priority-caps`. Priority orders `clm backlog` and `clm next`: a pending requirement takes the priority of the live change that added, modified or removed it, a component the highest of its unblocked pending requirements, and a component that urgent work waits on takes that work's priority (`priority-orders-nodes`). It never holds work back.
