# Shaping the graph

A CLM project is a graph of components, each with one `SPEC.md`. This page is about the shape of that graph: how big a component should be, when to split one, how a split is done, and what a shared layer is. It applies to any project built with CLM, CLM itself included.

## What a component is for

A component is the unit a build session works on. When `clm run` launches a build, the session receives `ARCHITECTURE.md`, the Contract and Design of every component from the root down to the one it is building (its *ancestors*), that component's requirements, and the Contracts of the components it depends on. It then reads and changes the code under the component's `paths`, and the tests that cite the component's requirements. Everything the session holds in its head at once is that bundle plus that code.

So a component has a natural size: small enough that one session can hold its spec and its code, and large enough that what belongs together stays together. CLM measures both (`clm size`): the spec in tokens of `SPEC.md`, the code in tokens of the tracked files a build reads, which are the files under its `paths` plus every test that cites it, wherever that test sits. Binary files and anything `.gitattributes` marks `linguist-generated` or `linguist-vendored` don't count. Everything else does, lockfiles included: a `Cargo.lock`, `package-lock.json` or `uv.lock` counts in full until `.gitattributes` marks it, as with the line `uv.lock linguist-generated` (a name with no `/` marks every file of that name, at any depth). A session never reads a lockfile, so mark each one the project tracks.

## Budgets and the oversized state

Every component has a budget for each count. The repository's `clm.toml` sets the defaults (`[component_size]`, 12,000 spec tokens and 40,000 code tokens unless you change them), and a component's frontmatter may set its own with `size_budget`. A component over a budget it hasn't set for itself, or over one it has, is **oversized**.

Nothing fails because of it. No check, gate or merge blocks an oversized component, and a build of one lands like any other. What happens instead is that the state can't be hidden: `clm size` says `oversized`, `clm status` shows it on the component's row and counts it in the summary, the spec view marks the component with `⚠`, and `clm health` lists it under `oversized`. Nothing files a backlog item for it: the state is the record. First make sure the count is real: a lockfile or generated file under the component's `paths` that `.gitattributes` doesn't mark counts in full, and marking it is a small code change that may clear the state with no split. A component still over its budget is dealt with in a spec change, one of two ways:

- **Split the component** along a seam (below).
- **Set `size_budget`** on the component, with the reason in its spec. That is the conscious exception: it's reviewed like any spec change, and it sits next to the component it's about. A budget the component later outgrows makes it oversized again.

## When to split

Split along architectural seams, never to satisfy the number. A seam is where two parts of a component:

- **change for different reasons**: the queue that decides what's ready and the prompt that tells a session how to build it change on different occasions;
- **have different dependencies**: a module that only talks to GitHub and one that only reads the spec graph;
- **are already separate**: a second package, a second crate, a second deliverable with its own manifest;
- **have their own contract**: something another component, or a person, relies on directly.

If the only argument for a cut is that the halves would be smaller, don't make it. A UI with five views behind one shell is one component; cutting it into five components that all share the shell gives you five specs that can't be built alone. Set the budget and write down why.

## How to split: nest

A component is split by **nesting**: the part you carve out becomes a child of the component it came from, at `specs/<component>/<part>/SPEC.md`, with the ID `<component>.<part>`. The half that stays keeps the component's ID. `web` stays the product and gains `web.worker`; `orchestrator` stays the queue and gains `orchestrator.run`.

Why a child rather than a new sibling:

- A child's builds read its parent's Contract and Design as part of their ancestors, so a component sees what its product promises without anyone listing it.
- Only the carved-out requirements move. Each is removed from the parent and added, text unchanged, to the child; the tests that cite it go `obsolete` on the parent and the child's requirement is `unbuilt`; the child's build re-points the markers, and the parent's obsolete lines wait for it (`moved-markers-wait`). Nothing else changes name.
- The spec view and the web graph draw the child inside its parent, so the picture stays a product with components rather than a longer list.

In the spec change, that is: a delta with `create: true` for the child (its frontmatter, Contract, Design and `ADDED Requirements`), a delta for the parent with `MODIFIED Frontmatter` (its `paths` shrink, its `depends_on` may gain the child), a `MODIFIED Contract` and `Design` that hand the carved-out part over, and `REMOVED Requirements` naming what moved. `paths` stay disjoint across every component, parent and child included. Which half keeps the ID: the one other components already name, or the one that is the product rather than a part of it.

### The cut must leave imports running one way

`depends_on` is a graph without cycles, and `clm check` judges every import edge between two components' files against it. So a split's cut must leave imports running one way: the child below the parent (the parent imports the child, never the reverse), or what both halves use in a shared layer of its own, a component below both (below). A cut that leaves imports running both ways can't pass `clm check`: without the edge in `depends_on` it reports undeclared imports, and with it a dependency cycle.

Inside one crate or package that is the usual trap. The crate root's `mod` lines and the parent's calls into the child are edges from the parent to the child; a carved-out module's `use crate::...` of the crate's shared modules are edges back. Files can stay in their crate only when the carved-out ones use none of the rest. Otherwise the carved-out part goes into a crate or package of its own, and what both halves share goes below both, as a shared layer, or into the child.

Check a cut before you write it. Write the component's halves as a component map, each with the `paths` the split gives it, listing every file the half will own (the example shortens the parent's), its `depends_on`, and `outside: ["**"]` for the rest of the code:

```yaml
components:
  - id: view
    paths: ["crates/clm-view/src/lib.rs", "crates/clm-view/src/spec.rs"]
    depends_on: [view.repo]
  - id: view.repo
    paths: ["crates/clm-view/src/repo.rs", "crates/clm-view/src/at.rs"]
    depends_on: []
outside: ["**"]
```

`clm survey --components <file>` exits 0 for a cut whose imports run one way, and otherwise reports each `survey/undeclared-edge` or `survey/cycle`. It reads the tree as it is, before any file moves, and that is enough: whether files can move into a crate of their own is exactly whether their imports already run one way.

The survey also reports an edge the change itself removes, such as a carved-out file importing a parent module that one of the change's requirements replaces with a value the parent passes in. Don't move the parent's module into the child to silence it. Such a `survey/undeclared-edge` line may stand when the importing file moves into a crate or package of the child's own (so `clm check` judges none of the child's files until the build makes them), a requirement the change adds to the child states the removal, naming the importing file, the file it imports and what replaces the import (what the parent must do for it, such as passing the value in, being a parent's requirement that needs the child's), and `change.md` lists the edge with the child's requirement under `## Edges the change removes`. A `survey/cycle` line never stands. Once the child is built, `clm check` reports any such edge the build left.

The `propose` skill knows this and will do it when a change splits a component, checking the cut first; `triage` checks a split the same way before it recommends one.

## Shared layers

When several components copy the same plumbing, a git wrapper, an HTTP client, a config reader, the copies are a maintenance cost of their own, and each component's size carries code that isn't about that component. Make the plumbing a component: one crate or package, one Contract, a test that keeps it the only copy where that's cheap to check, and `depends_on` edges from the components that use it. The copies go in those components' next builds. CLM's own `vcs` component (crate `clm-git`) is one of these: every command that shells out to `git` or `gh` goes through it.

A shared layer is a dependency for many components, so it should change rarely and its Contract should be plain. A build of it settles before its dependants are ready, which is the right order: a change to plumbing lands before the code that relies on it.

## Reading the result

`clm size` lists every component with both counts and marks the oversized ones. `clm view spec` draws the graph with children inside their parents and `⚠` on an oversized component. `clm status` counts them, and `clm health` lists them with both counts against their budgets.
