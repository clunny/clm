---
status: applied
resolves: [clm-rebase-regenerates-a-rename-branch-from-the-repository-s-top]
---

## Intent

`clm rebase` regenerates a rename branch from the CLM root's trees, with paths relative to the root, as `rename/replay` reads them (`fingerprint-covers-the-clm-root`), so a rename branch in a CLM repository that is a subdirectory of its git repository is rebuilt on a moved `main` instead of failing (`sync/regenerate-renames`, `regenerate-dont-merge`). No change or code fix since 2026-11-12 touches it: `20261119-rename-replay-reads-the` filed it and makes `replays` read the root's trees, and no other change has a delta for `sync` that names the rename path.

## Problem

`crates/clm-sync/src/renames.rs` reads the change's `change.md` as `HEAD:changes/<id>/change.md`, which git resolves from the repository's top; lists each revision with `git ls-tree --full-tree` (`rev_ids`); runs `clm rename` at the top of the tree `extract_tree` extracts; and writes and stages the result by those top-relative paths. `added_changes` in `lib.rs`, which finds the change, lists `changes/` in the root but reads each `change.md` with `git show HEAD:<file>`, again from the top. With the CLM root at `tool/`, `clm rebase` on a branch that adds a change fails when git can't find that path, and, were the change found, would run `clm rename` where there is no `specs/`, compare the root's listing against the whole repository's, and write files to `tool/tool/...`. It fails closed, but `root-from-cwd` lets a project keep CLM in a subdirectory, and `rename/replay`, which `regenerate-renames` asks first, is being taught the root (`20261119-rename-replay-reads-the`). No requirement says which tree `regenerate-renames` reads, so the fix needs one (`code-fix-needs-a-written-rule`).

## Approaches

- **Taken: read the root's trees, as `rename/replay` does, and name a path outside the root as the gate does.** Each revision is listed once with `--full-tree` and split by `git rev-parse --show-prefix`: the entries under the prefix, with it taken off, are the root's listing; the rest are outside it. Blobs are read as `<rev>:./<path>` from the root. The rename runs in `<prefix>` of the extracted tree, the replay's and the regenerated one's alike. Files are written and staged by root-relative paths in the root's working tree. Outside the root, `replays` already requires `HEAD` to equal the merge base, so the regenerated commit holds `origin/main`'s entries there and the working tree is brought to them; a path outside the root that differs (which is what makes `replays` false there) is printed as `../<path>`, the form `gate/paths-under-the-root` gives. At a repository's top the prefix is empty, nothing lies outside, and nothing changes. `added_changes` reads each `change.md` as `HEAD:./<file>`, since the rename path finds its change through it; steps 2 and 4 share that listing, but no rule of theirs says which tree they read, so what they then do in a subdirectory root is the filed item's, not this change's.
- **Not taken: run the whole regenerate step at the repository's top and map paths at the edges.** `replays`, `clm rename` and the gate all work in the root, so a second frame of reference would have to be kept in step with all three.
- **Not taken: refuse a rename branch in a subdirectory root.** `root-from-cwd` says a subdirectory root works, and the gate and `replays` now support one.
- **Not taken: make every path of `clm rebase` the root's in this change.** Step 1's prefix tests, step 2's merge-driver pattern and step 4's reads (`extract-through-layer` says "the same top-relative paths") also read the top (`backlog/clm-rebase-matches-specs-changes-and-backlog-at-the-repository-s-top-in-a-subdirectory-root`); they are other requirements' text and a separate fix, and the rename path doesn't reach step 2.

## Outcome

- `sync/regenerate-renames` (modified): a second paragraph says it reads each revision as the CLM root's tree, runs `clm rename` in the root's directory of each extracted tree, prints a path outside the root whose entry differs from the merge base's as `../<path>`, and makes the commit hold `origin/main`'s entries outside the root, writing the working tree there only where the commit differs from `HEAD`. Two new scenarios, *a rename in a subdirectory root* and *a file outside a subdirectory root*. Its `#### Needs` stay `rename/replay` and `rename/finish`: the scenarios rely on `replays` reading the root's trees, which `20261119-rename-replay-reads-the` gives `rename/replay`.
- `backlog/clm-rebase-matches-specs-changes-and-backlog-at-the-repository-s-top-in-a-subdirectory-root` (added, `kind: bug` on `sync`): step 1's refusals, step 2's merge-driver attributes and the rules of step 4's reads (`extract-through-layer`, `reapply`) still match or state paths from the repository's top.

## Constraints

- `rename/replay` is pending at the version `20261119-rename-replay-reads-the` gives it, so this requirement's build waits on `rename`'s (`#### Needs`).
- `clm-regenerate` already reads `specs/` and the layers under the root's prefix, so the regenerate step after a rename (`conclude`) needs nothing new; step 8's temporary worktree already finds the root by the prefix.

## Edge cases and workarounds

- An uncommitted edit to a file outside the root that `origin/main` changed since the merge base is refused by step 1 before the regenerate step runs (`uncommitted <path>`, as step 1 prints it today), so the commit step's own uncommitted check needs nothing new outside the root; its comparison with `git diff --name-only HEAD`, whose paths are the repository's, is read as the root's by taking the prefix off (builder check).
- Outside the root, a differing entry is any whose path, mode or object ID differs from the merge base's, a symbolic link, an executable bit or a submodule included, so a branch `replays` refuses for one of those is named by it, not by its `change.md` (builder check).
- `clm-sync` may take the prefix-split listing from `clm-rename` if `rename/replay`'s build makes it public, or keep its own in `rev_ids`; reuse is preferred (`reuse-first`), and the build decides by what that build leaves.
- The working tree outside the root is written only where the regenerated commit differs from the branch's `HEAD`, so an unrelated edit step 1 lets through stays (`sync-carries-unrelated-edits`) (builder check).
- A new file under the root's `backlog/` is kept as before; a new file under a `backlog/` outside the root is outside the root, makes `replays` false, and is printed as `../<path>`.

## What the requirements no longer state

Nothing: no sentence holding `SHALL` is removed.

## Open questions

None.
