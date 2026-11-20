---
status: applied
resolves: [rename-replay-reads-the-repository-s-top-not-the-clm-root]
---

## Intent

`rename/replay` reads the CLM root's trees, with paths relative to it, as the approval fingerprint does (`fingerprint-covers-the-clm-root`), so a rename branch in a CLM repository that is a subdirectory of its git repository replays and the gate classes it `spec`. It is false when the branch adds, removes or changes anything outside the root, since the gate classes a replayed branch `spec` with every file it changes and an approval covers only the root's files (`approval-policy`). No change or code fix since 2026-11-12 touches it: `20261119-the-gate-reads-changed` filed it, and `20261119-clm-rename-breaks-five` changes other parts of `replay`.

## Problem

`crates/clm-rename/src/replay.rs` lists both revisions with `git ls-tree -r --full-tree` and extracts the whole repository. With the CLM root at `tool/`, `added_changes` sees `tool/changes/<id>/change.md`, never `changes/<id>/change.md`, so `replays` is always false and a rename branch there is `mixed` and can't land (`backlog/rename-replay-reads-the-repository-s-top-not-the-clm-root`). It fails closed, but `root-from-cwd` lets a project keep CLM in a subdirectory, and the gate itself now reads the root's paths (`gate/paths-under-the-root`). No requirement says which tree `replays` reads, so the fix needs one.

## Approaches

- **Taken: one full listing per revision, split by the root's prefix.** The entries under `git rev-parse --show-prefix`, with the prefix taken off, are the root's listing, exactly what `clm fingerprint` lists; the entries outside it must be equal between the revisions or `replays` is false before any extraction. `base`'s tree is still extracted whole with `extract_tree`, and `rename` runs in its `<prefix>` directory. At a repository's top the prefix is empty and nothing changes. One process per revision, as today, and no new `clm-git` call.
- **Not taken: list in the root without `--full-tree`, as `clm-fingerprint` does, plus a `git diff --quiet` for the outside.** Same answer for one more process, and two readings of the tree to keep in step.
- **Not taken: ignore the outside of the root.** The gate drains every code file of a replayed branch into its spec files, so a file outside the root would land as `spec` under an approval whose fingerprint never covered it (`approval-policy`).
- **Not taken: `extract_paths` of the prefix alone.** It refuses an empty path, so the top would need its own branch; the whole extraction costs little and is what `replays` does now.
- **Not taken: accept the failure.** A rename branch can't land in a subdirectory root at all, which `root-from-cwd` says works.

## Outcome

- `rename/replay` (modified): `repo` is the CLM root; each revision's tree is read as the root's, with paths relative to it; it is true only when `head`'s entries outside the root are `base`'s, and false without extracting a tree when they aren't. Two new scenarios, *a subdirectory root* and *a file outside a subdirectory root*. No sentence holding `SHALL` is lost.
- `rename`'s Contract names `repo` as the CLM root, and its Design's **Replay** says how the listing is split and where `rename` runs.
- `backlog/clm-rebase-regenerates-a-rename-branch-from-the-repository-s-top` (added, `kind: bug` on `sync`): `sync/regenerate-renames` reads `HEAD:changes/<id>/change.md` from the repository's top and runs `clm rename` at the extraction's top, so it can't regenerate a rename branch in a subdirectory root either. It fails closed and needs its own requirement text, so it is filed, not designed here.

## Constraints

- No caller changes: `gate` (`classify`, `files-outside-spec-paths`) and `sync` already pass the CLM root as `repo`.
- `gate/rules-version` needs no new version. Under versions 1 to 4 the gate reads paths from the repository's top, so in a subdirectory root it finds no added `changes/<id>/change.md` and never asks `replays`; under version 5 a rename branch there was `mixed` and blocked, so no landed record holds one, and a landed `spec` or `code` branch never has `replays` asked of it (a `spec` branch has no code file, a `code` branch adds no change). No recorded judgment changes. At a repository's top nothing changes under any version.

## Edge cases and workarounds

- A new file under the root's `backlog/` is still left out of the comparison; a new file under a `backlog/` outside the root is an entry outside the root and makes `replays` false.
- A submodule or symbolic link outside the root is compared as any entry outside it, by path, mode and object ID.
- `rename/replay` is already `behind` from `20261119-clm-rename-breaks-five` (modes, symbolic links, submodules, a refusal on the base), so one build of `rename` catches up both changes' text of it, and its tests cite the version this change gives it (builder check).
- `gate/files-outside-spec-paths` asks `replays` whatever `RULES` says, so in a subdirectory root it lists nothing for a clean rename branch once this is built; that is the intended effect, and that function's answer is not audited by version (builder check).

## What the requirements no longer state

Nothing: no sentence holding `SHALL` is removed.

## Open questions

None.
