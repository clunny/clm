---
status: done
resolves: [the-gate-reads-changed-paths-relative-to-the-git-repository-s]
---

## Intent

The gate reads the paths a branch changes relative to the CLM root, as the approval fingerprint does (`fingerprint-covers-the-clm-root`), and counts a file outside the root as code. In a CLM repository that is a subdirectory of its git repository, a branch editing the root's specs or `ARCHITECTURE.md` is then a spec branch that needs review where it needs one, and a spec branch that also changes a file outside the root is `mixed` and blocked, so an approval, whose fingerprint covers only the root's files, never lands a file it didn't cover (`approval-policy`). New requirement `gate/paths-under-the-root`.

## Problem

`clm-gate`'s `changed_files` runs `git diff --name-status --no-renames -z <range>` in the CLM root, and `git diff` prints paths relative to the git repository's top whatever directory it runs in. With the root at `tool/`, a branch editing `tool/specs/a/SPEC.md` lists `tool/specs/a/SPEC.md`, which `is_spec_path` doesn't count, so the gate classes a spec branch as `code` and never asks for review. The gate's other reads mix the two forms: `git show <rev>:<path>` reads from the repository's top, while pathspecs (`status_only`'s `git diff <range> -- <file>`, `ls-tree -- changes`, the code-fix check's `:(glob)<path>`) are read relative to the directory git runs in. No requirement says which form is right; the fingerprint's does, since `20261119-the-fingerprint-covers-only`.

## Approaches

- **Taken: turn each path into one relative to the root.** A path under the root loses the root's prefix (`git rev-parse --show-prefix`); a path outside it climbs out with `../`, which git itself reads relative to the directory it runs in, both as a pathspec and as `<rev>:./<path>`. Every rule then reads root-relative paths, as the fingerprint and `land/derived-specs` already do, and an outside file can't collide with a root file of the same name. At a repository's top the prefix is empty and nothing changes.
- **Not taken: `git diff --relative`.** It prints root-relative paths but drops every file outside the root, so a spec branch carrying one would land under an approval that never covered it.
- **Not taken: refuse a CLM root that isn't the repository's top.** `root-from-cwd` lets a repository keep CLM in a subdirectory, and the fingerprint was just defined for that layout.
- **Not taken: match spec paths through the prefix and keep top-relative paths.** Every rule that names a path (`clm.toml`, `docs/allowed_signers`, `changes/<id>/`, a component's `paths`, the code-fix trailer's `backlog/<slug>`) would carry the prefix, and the pathspec reads would still need the other form.

## Outcome

- `gate/paths-under-the-root` (added): every changed path is read relative to the CLM root; a file outside it is named with `../` and counts as code, so a spec branch carrying one is `mixed`; `files-outside-spec-paths` returns it in that form, and `foreign_files` returns it with an empty owner, so `land/build-holds-own-files` refuses a build branch that changes one (`path-ownership`), though no component owns it and `check/unowned-files` never sees it (architecture audit). Six scenarios, all with the root at `tool/`.
- `gate/approval-setting` (modified): `clm.toml` is the root's file, read with `git show <rev>:./clm.toml` in the root. At a repository's top nothing changes; no sentence holding `SHALL` is lost.
- `gate/rules-version` (modified): `RULES` is 5. Version 5 reads paths as `paths-under-the-root` states; versions 1 to 4 read them as `git diff` prints them, as the gate did, so a land judged in a subdirectory root before this change is judged again as it was (`audit-by-the-rules-in-force`). In such a repository the kind, the cuts and the violations under versions 1 to 4 are those that reading gives; at a repository's top every version gives the same kind, as before. New scenario *a subdirectory root under each version*.
- `gate`'s Design gains **Paths under the root** and names version 5.
- `docs/decisions/fingerprint-covers-the-clm-root.md` names the requirement in place of the backlog item.

## Constraints

- `RULES` goes from 4 (pending from `20261119-the-gate-s-placement`) to 5, not folded into 4, since a build of that change alone may land before this one and judge lands under 4 with the old path reading.
- The path reading is one function given the version, not a second gate (`rules-version`'s Design).
- No caller of `clm-gate` changes: `land`, `ledger` and `crates/clm` pass the CLM root as `repo` already, and the Worker lists only repositories whose CLM root is the top, where nothing changes.

## Edge cases and workarounds

- A path outside the root is the shortest path from the root to it: with the root at `a/b/`, `a/x` is `../x` and `x` is `../../x`.
- `extract_paths` takes paths relative to the repository's top and refuses `..`, so the gate's graph loads pass the root's prefix and load from its directory in the extraction (builder check); `clm-git` doesn't change.
- `rename/replay` reads the whole repository's tree (`ls-tree --full-tree`), so in a subdirectory root it never finds the branch's added change and a rename branch is `mixed`, failing closed. Filed as `backlog/rename-replay-reads-the-repository-s-top-not-the-clm-root` on `rename` (builder check).
- A branch at the repository's top that edits a spec-path name outside the root, such as the top's own `CLAUDE.md`, rates lower under version 5 (`code`) than under 4 (`spec`); the audit reports only understatement, so no version is needed for it.
- `gate/up-to-date`'s check of `specs/` changed on both sides names the root's `specs/` files.

## What the requirements no longer state

Nothing: no sentence holding `SHALL` is removed. `approval-setting`'s "the path at the top of the repository" becomes "the file at the CLM root", the same file wherever the root is the top.

## Open questions

- **`#### Enforced by`** (architecture audit). `gate/paths-under-the-root` serves `approval-policy` and `humans-review-intent`, and neither entry's `#### Enforced by` block names it. Adding it is an `ARCHITECTURE.md` edit, which would make this branch need review for a list entry, so it is left for a human to add when next editing those entries; the rule is checked and gates whether or not the entry names it.
