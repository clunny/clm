---
title: Fingerprint
status: active
owners: ["@clunny"]
depends_on: [vcs]
paths: ["crates/clm-fingerprint/**"]
serves: [capability/approve-intent, architecture/approval-policy, architecture/hash-based-staleness]
---

## Contract
`clm fingerprint --base <ref>` prints the fingerprint of the changes `HEAD` makes relative to `<ref>`, so an approval can be pinned to the diff it was given for (`approve-pins-the-review`). It depends on the two commits alone, never on git config or the host that computes it (`tree-diff-fingerprint`). Crate `clm-fingerprint` holds it: `fingerprint(repo, base)` is the command's answer; `fingerprint_of(base, head)` computes it from the two trees' entries (`Entry`), each a mode, type, full object ID and path as `git ls-tree -r` lists it and `parse_ls_tree` reads it, with no process, so the `web` Worker computes the same value from the Trees API's listings. `adds_change(base, head)` says whether `head` adds a change, when the fingerprint skips `specs/`, and `added_changes(base, head)` names the `change.md` paths it counts. `clm-gate` re-exports all six (`gate/fingerprint-reexported`).

## Design
`ls-tree -z` prints full object IDs and unquoted paths whatever `core.abbrev` or `core.quotePath` say, and no `diff.*` setting reaches it, which is why the listings and not a diff are hashed (`fingerprint-ignores-config`). `tree` entries are ignored because the Trees API lists them and `ls-tree -r` doesn't. Paths are compared and sorted as bytes, so a path that isn't UTF-8 still hashes as git wrote it; the Worker's listings are JSON, and GitHub's paths are UTF-8. A renamed file is a removal plus an addition, as `--no-renames` would show it, so rename detection never enters. Entries hash file contents through their object IDs, so merging a `main` that changed a file the branch also changes changes the fingerprint even when the branch's own lines are the same.

On a branch that adds a change, `specs/` is derived from the merge base's `specs/` and the change's deltas and assets (`regenerate-dont-merge`), so its entries are skipped (`derived-specs-fingerprint`, `fingerprint-stable`). A branch that adds no change, an archive move among them, keeps `specs/` in the hash, since its `specs/` edits (an `impl/*` branch's `Activate` commit, a finish branch's `status:` line) are authored. What makes skipping safe is `land/derived-specs`: `clm land` refuses a head whose `specs/` isn't what regenerating gives, so a hand edit under `specs/` on a spec branch never lands under an approval that didn't cover it.

## Requirements

### Requirement: fingerprint
`clm fingerprint --base <ref>` SHALL print `fingerprint_of` over the entries of `git ls-tree -r -z <merge-base>` and `git ls-tree -r -z HEAD`, where `<merge-base>` is the merge base of `<ref>` and `HEAD`: the lowercase hex SHA-256 over each path whose entry differs between the two trees or exists in one only, sorted by its bytes, of the path, a NUL, its entry at the merge base as `<mode> <type> <id>` or `-` when absent, a NUL, its entry at `HEAD` the same way, and a NUL, ignoring `tree` entries and, when `HEAD` adds a change, every path under `specs/`. `HEAD` adds a change when it holds a `changes/<id>/change.md` the merge base lacks, or a `changes/archive/<id>/change.md` the merge base lacks while lacking `changes/<id>/change.md` too, so an archive move adds none; `<id>` is one path component and never `archive`, so `changes/archive/change.md` and `changes/x/deltas/change.md` add nothing. `clm-fingerprint` SHALL offer that test as `adds_change(base, head)` over the same entries, so `clm land` asks the same question of the same listings. It SHALL print `empty` when no path counts. `fingerprint_of` SHALL run no process and SHALL build for `wasm32-unknown-unknown`, and `clm-fingerprint` SHALL run `git` only through `clm-git` (`git-plumbing-layer`).

#### Scenario: content changed
- GIVEN a branch that adds no change and edits `specs/a/SPEC.md`, with one more commit changing a line in `specs/a/SPEC.md`
- WHEN `clm fingerprint --base main` runs before and after that commit
- THEN it prints a different fingerprint after it

#### Scenario: listings, not a process
- GIVEN two tree listings in which only `a.txt`'s blob ID differs, plus a `tree` entry present in one listing only
- WHEN `fingerprint_of` runs on them
- THEN it returns the SHA-256 of `a.txt`, NUL, `100644 blob <old id>`, NUL, `100644 blob <new id>`, NUL, as hex, and the same value with the `tree` entry removed

#### Scenario: listings of a branch that adds a change
- GIVEN two tree listings in which `specs/a/SPEC.md`'s blob ID differs and `changes/x/change.md` is in the second only
- WHEN `fingerprint_of` runs on them
- THEN it returns the SHA-256 of `changes/x/change.md`, NUL, `-`, NUL, `100644 blob <id>`, NUL, as hex, the same whatever `specs/a/SPEC.md`'s ID in the second

#### Scenario: listings of an archive move
- GIVEN two tree listings in which `specs/a/SPEC.md`'s blob ID differs, `changes/x/change.md` is in the first only and `changes/archive/x/change.md` in the second only
- WHEN `fingerprint_of` runs on them
- THEN it hashes all three paths

### Requirement: fingerprint-ignores-config
`clm fingerprint` SHALL NOT depend on git config: the same two commits SHALL give the same fingerprint whatever `core.abbrev`, `core.quotePath` and `diff.*` settings the repository has.

#### Scenario: git config ignored
- GIVEN a branch that renames a text file with a one-line edit, changes a binary file, and adds a file holding Latin-1 bytes
- WHEN `clm fingerprint --base main` runs once with no `core.abbrev` or `diff.*` settings, and once with `core.abbrev=16`, `core.quotePath=false`, `diff.renames=copies`, `diff.algorithm=patience`, `diff.noprefix=true` and `diff.context=8` in the repository's config
- THEN both print the same fingerprint

### Requirement: fingerprint-stable
The fingerprint SHALL be unchanged by rebasing onto a `<ref>` that touched only other files and, on a branch that adds a change, by a sync onto a `<ref>` that changed `specs/` after which `specs/` is regenerated from the same deltas and assets (`derived-specs-fingerprint`); it SHALL change when the content or mode of any other file the branch adds, modifies or deletes changes, a file `<ref>` also changed included.

#### Scenario: pure rebase
- GIVEN a branch that adds no change and edits `specs/a/SPEC.md`, and a new commit on `main` editing only `specs/b/SPEC.md`
- WHEN the branch is rebased onto `main`
- THEN `clm fingerprint --base main` prints the same fingerprint as before

#### Scenario: same file changed on main
- GIVEN a branch that adds no change and edits one line of `specs/a/SPEC.md`, and a new commit on `main` editing another line of `specs/a/SPEC.md`
- WHEN the branch merges `main` without conflict
- THEN `clm fingerprint --base main` prints a different fingerprint from before

#### Scenario: regenerated onto a main that changed the same file
- GIVEN a spec branch adding applied change `x`, whose delta adds requirement `a/r`, and a new commit on `main` adding requirement `a/s` to `specs/a/SPEC.md`
- WHEN the branch is rebased onto `main` and `specs/` is regenerated from `main`'s with `x`'s deltas applied, as `clm rebase` does, leaving `x`'s deltas as they were
- THEN `clm fingerprint --base main` prints the same fingerprint as before

#### Scenario: delta edited
- GIVEN the same branch with one more commit changing a line of `changes/x/deltas/a.md` and regenerating `specs/`
- WHEN `clm fingerprint --base main` runs
- THEN it prints a different fingerprint

#### Scenario: specs edited by hand on a spec branch
- GIVEN the same branch with one more commit changing a line of `specs/a/SPEC.md` and nothing else
- WHEN `clm fingerprint --base main` runs
- THEN it prints the same fingerprint as before (`land/derived-specs` is what refuses the branch)
