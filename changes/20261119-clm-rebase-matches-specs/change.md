---
status: applied
resolves: [clm-rebase-matches-specs-changes-and-backlog-at-the-repository-s-top-in-a-subdirectory-root]
---

## Intent

Every path `clm rebase` matches, writes or prints is the CLM root's, and a path outside the root is printed as `../<path>`, as the gate names one (`gate/paths-under-the-root`, `fingerprint-covers-the-clm-root`), so in a CLM repository that is a subdirectory of its git repository step 1 refuses an uncommitted edit to the root's `specs/`, `changes/` or `backlog/`, step 2 keeps `origin/main`'s side of a `specs/` conflict, and step 5 reports a spec that moved (`backlog/clm-rebase-matches-specs-changes-and-backlog-at-the-repository-s-top-in-a-subdirectory-root`). No change or code fix since 2026-11-12 fixes it: `20261119-clm-rebase-regenerates-a` filed it and teaches only the rename path the root, and no `Resolves:` commit names it.

## Problem

Read from `crates/clm-sync/src/lib.rs` on 2026-11-19, not run. With the CLM root at `tool/`:

- **Step 1** reads `git status --porcelain` and `git ls-files --full-name`, whose paths are from the repository's top (`tool/specs/a/SPEC.md`), and tests them for the prefixes `specs/`, `changes/` and `backlog/`. An uncommitted edit under `tool/specs/` on a spec branch is set aside by the autostash instead of refused, and an edit to a top-level `specs/` outside the root would be refused as if it were the root's. `uncommitted` lines print top-relative paths.
- **Step 2** writes `specs/** merge=clm-keep` to `core.attributesFile`, where git matches a pattern holding a slash from the top, so `tool/specs/` never takes the driver. Its fallback tests the conflicted paths `git diff --name-only` prints (top-relative) for `specs/`, so a `specs/` conflict stops the rebase, and the pathspecs it would hand back (`ls-files --stage`, `checkout --ours`, `add`, `rm`) are read from `tool/`, so `tool/tool/specs/...`. `conflict` lines print top-relative paths.
- **Step 4**'s tree is `clm-regenerate`'s, which already extracts and loads under the root's prefix, but `extract-through-layer` says it reads "the same top-relative paths it reads today", which the code no longer does, and no rule states which tree the regenerate step reads.
- **Step 5** (not named by the item, found reading the same code) extracts `specs`, `ARCHITECTURE.md` and `CAPABILITIES.md` from the top, where a subdirectory root has none, so the bundle is absent at both revisions and `spec changed` never fires; its `changed` lines would print top-relative paths.

`sync`'s requirements name `specs/`, `changes/` and `backlog/` without saying whose, and none says how a path outside the root is printed, so the fix needs a written rule (`code-fix-needs-a-written-rule`).

## Approaches

- **Taken: one rule for the whole command, as the gate has (`gate/paths-under-the-root`).** A new requirement `sync/paths-under-the-root` says every path `clm rebase` reads is the root's, the trees its rules name are the root's, and every `uncommitted`, `conflict` and `changed` line names a path relative to the root, `../` for one outside. The Design gains a **Paths** paragraph saying where git prints from the top and how each step takes such a path to the root before matching it, the attributes rule `<prefix>specs/**`, and step 5's extraction under the prefix. `extract-through-layer` is reworded from "top-relative" to the root's paths under the prefix, which is what `clm-regenerate` already does. At a repository's top nothing changes.
- **Not taken: a sentence in each of `rebase`, `reapply` and `spec-changed`.** Three copies of one rule, each needing its own scenarios; the gate states it once, and so does this.
- **Not taken: run every step at the repository's top and map paths at the edges.** `clm check`, `clm-regenerate`, `rename/replay` and the gate all work in the root; a second frame of reference would have to be kept in step with each.
- **Not taken: refuse `clm rebase` in a subdirectory root.** `root-from-cwd` lets a project keep CLM in a subdirectory, and the gate, the fingerprint and the rename path now support one.
- **Not taken: move the gate's `name_from_top` into `clm-git` for every crate.** It would edit `gate`'s and `vcs`'s requirements for a function of ten lines; `clm-sync` holds one copy, shared by its own steps and the rename path (`reuse-first` within the crate). Should a third crate need it, moving it to `vcs` is the next step.

## Outcome

- `sync/paths-under-the-root` (added), with six scenarios in a root at `tool/`: an uncommitted edit under the root's `backlog/` refused as `uncommitted backlog/q.md`; an edit to a top-level `specs/notes.md` outside the root carried through; an edit outside the root that `main` changed refused as `uncommitted ../README.md`; a `specs/` conflict kept on `origin/main`'s side and regenerated; a conflict outside the root printed as `conflict ../README.md`; and a spec moved under `impl/a` reported as `changed specs/a/SPEC.md`.
- `sync/extract-through-layer` (modified): each path the root's, given to `extract_paths` from the top as `<prefix><path>`, the graph loaded from the extraction's `<prefix>`, and the regenerate step's second extraction moved into the first under `<prefix>`.
- `sync`'s Design: a **Paths** paragraph after the steps of `clm rebase`.

## Constraints

- `git status --porcelain`, `git diff --name-only` and `git ls-files --full-name` print paths from the repository's top whatever directory they run in; `git ls-tree` and `git ls-files` without `--full-name` print them relative to it, and pathspecs are read relative to it.
- `extract_paths` takes paths from the top of the tree and refuses `..`, so a path outside the root is never extracted by name.
- `core.attributesFile` patterns holding a slash match from the top.

## Edge cases and workarounds

- A path outside the root is never under one of the root's trees, so it stops step 1 only as a staged change, on `impl/*`, or as a file `origin/main` changed; it is never handed back to git as a pathspec, since step 2 resolves only the root's `specs/` conflicts.
- Step 1 compares the status entries with the set of files `origin/main` changed after both are named relative to the root, so the comparison reads one form.
- The rename path (`regenerate-renames`, pending at the version `20261119-clm-rebase-regenerates-a` gives it) names a path outside the root the same way; the build shares one function between them, whichever requirement is built first.

## What the requirements no longer state

Nothing: no sentence holding `SHALL` is removed. `extract-through-layer` loses the words "with the same top-relative paths it reads today", which the code no longer does.

## Open questions

None.
