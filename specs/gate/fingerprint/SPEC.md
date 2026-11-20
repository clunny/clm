---
title: Fingerprint
status: active
owners: ["@clunny"]
depends_on: [vcs]
paths: ["crates/clm-fingerprint/**"]
serves: [capability/approve-intent, architecture/approval-policy, architecture/hash-based-staleness]
---

## Contract
`clm fingerprint --base <ref>` prints the fingerprint of the changes `HEAD` makes relative to `<ref>`, so an approval can be pinned to the diff it was given for (`approve-pins-the-review`). It depends on the two commits alone, never on git config or the host that computes it (`tree-diff-fingerprint`). Crate `clm-fingerprint` holds it:
- `fingerprint(repo, base)` is the command's answer, or an error, as text, when `base` and `HEAD` have no merge base or a tree can't be listed. It lists both trees with `git ls-tree -r -z` run in `repo`, so the paths it hashes are relative to `repo`.
- `fingerprint_of(base, head)` computes it from the two trees' entries with no process, so the `web` Worker computes the same value from the Trees API's listings. Those list the whole repository, so the two values are equal when the CLM repository's root is the top of its git repository.
- `Entry` is one entry of a listing, with the public fields `mode`, `kind` (its type: `blob`, `tree` or `commit`), `id` (the full object ID) and `path` (bytes), which a caller that has no listing to parse builds itself, and `parse_ls_tree(listing)` reads the entries of `git ls-tree -r -z` output.
- `added_changes(base, head)` names the `change.md` paths by which `head` adds a change, which `land/derived-specs` and `rename/replay` read, and `adds_change(base, head)` says whether there is any, which is when the fingerprint skips `specs/`.

`clm-gate` re-exports all six (`gate/fingerprint-reexported`). The crate builds for `wasm32-unknown-unknown`. The command's arguments, stream and exit status are the root's (`crates/clm` routes it and prints what `fingerprint` returns).

## Design
`ls-tree -z` prints full object IDs and unquoted paths whatever `core.abbrev` or `core.quotePath` say, and no `diff.*` setting reaches it, which is why the listings and not a diff are hashed (`fingerprint-ignores-config`). `tree` entries are ignored because the Trees API lists them and `ls-tree -r` doesn't. The Worker's listings are JSON, and GitHub's paths are UTF-8. A renamed file is a removal plus an addition, as `--no-renames` would show it, so rename detection never enters. Entries stand for file contents through their object IDs, so no blob is read.

On a branch that adds a change, `specs/` is derived from the merge base's `specs/` and the change's deltas and assets (`regenerate-dont-merge`), which is why its entries are skipped (`derived-specs-fingerprint`, `fingerprint-stable`). On a branch that adds none, its `specs/` edits (an `impl/*` branch's `Activate` commit, a finish branch's `status:` line) are authored. What makes skipping safe is `land/derived-specs`.

## Requirements

### Requirement: fingerprint
`fingerprint(repo, base)` SHALL return, and `clm fingerprint --base <ref>` SHALL print, `fingerprint_of` over the entries of `git ls-tree -r -z <merge-base>` and `git ls-tree -r -z HEAD`, both run in `repo`, where `<merge-base>` is the merge base of `<ref>` and `HEAD`. `fingerprint_of(base, head)` SHALL return the lowercase hex SHA-256 of these bytes, for each path that counts, in ascending order of the paths' bytes: the path's bytes as listed, a NUL byte, the path's entry in `base` as the text `<mode> <type> <id>` with single spaces, or the one byte `-` when `base` has no entry for it, a NUL byte, its entry in `head` the same way, and a NUL byte. A path counts when that text differs between the two listings or one listing lacks the path, leaving out every entry whose type is `tree` and, when `head` adds a change, every entry whose path begins with the bytes `specs/`. It SHALL return `empty`, which the command prints, when no path counts. `added_changes(base, head)` SHALL return, in `head`'s order, the path of each entry of `head` whose type isn't `tree`, that `base` has no such entry for, and that is `changes/<id>/change.md` with `<id>` other than `archive`, or `changes/archive/<id>/change.md` while `base` has no entry `changes/<id>/change.md`, so an archive move adds none; `<id>` is one non-empty path component, so `changes/archive/change.md`, `changes/x/deltas/change.md` and `changes//change.md` add nothing. `adds_change(base, head)` SHALL be true exactly when `added_changes` returns a path, and `head` adds a change exactly then. `parse_ls_tree` SHALL split the listing at each NUL byte and read each piece that holds a tab, and whose bytes before its first tab are UTF-8 holding at least two spaces, into an `Entry`: the mode, type and ID are the text before the first space, between the first two spaces, and from there to the next space or the tab, and the path is every byte after the first tab, unchanged, though it be empty. It SHALL skip every other piece, the empty one after the last NUL among them, and SHALL read a last piece that no NUL ends as it reads any other. `fingerprint(repo, base)` SHALL return an error when `<ref>` and `HEAD` have no merge base or a tree can't be listed. `fingerprint_of` SHALL run no process, and `clm-fingerprint` SHALL build for `wasm32-unknown-unknown`.

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

#### Scenario: nothing differs
- GIVEN a branch with no commit of its own
- WHEN `clm fingerprint --base main` runs
- THEN it prints `empty`

#### Scenario: main ahead of the fork point
- GIVEN a branch that adds no change and edits `specs/a/SPEC.md`, and a later commit on `main` editing only `specs/b/SPEC.md`, the branch not rebased
- WHEN `clm fingerprint --base main` runs before and after that commit
- THEN it prints the same fingerprint both times

#### Scenario: change IDs are one component
- GIVEN an empty first listing and a second holding one blob entry
- WHEN `adds_change` runs for the path `changes/x/change.md`, and for `changes/archive/x/change.md`, `changes/archive/change.md`, `changes/x/deltas/change.md` and `changes//change.md`
- THEN it is true for the first two and false for the other three

#### Scenario: added changes named
- GIVEN a first listing holding `changes/z/change.md`, and a second holding `changes/x/change.md`, `changes/archive/y/change.md`, `changes/archive/z/change.md` and a `tree` entry `changes/w/change.md`
- WHEN `added_changes` runs on them
- THEN it returns `changes/x/change.md` and `changes/archive/y/change.md`, in that order, and nothing else

#### Scenario: paths are bytes
- GIVEN the listing `100644 blob aaa`, a tab, the bytes `caf\xe9.txt`, NUL, `040000 tree ttt`, a tab, `dir`, NUL, `junk`, NUL
- WHEN `parse_ls_tree` reads it, and `fingerprint_of` runs on an empty listing and its entries
- THEN it gives two entries, the first with mode `100644`, type `blob`, ID `aaa` and the path bytes `caf\xe9.txt`, and the fingerprint is the SHA-256 of `caf\xe9.txt`, NUL, `-`, NUL, `100644 blob aaa`, NUL, as hex

#### Scenario: no merge base
- GIVEN a repository whose branch `lonely` shares no commit with `HEAD`
- WHEN `fingerprint(repo, "lonely")` runs
- THEN it returns an error

#### Scenario: builds for the web
- GIVEN the workspace with the `wasm32-unknown-unknown` target installed
- WHEN `clm-fingerprint` is built for that target
- THEN the build succeeds

### Requirement: fingerprint-ignores-config
`clm fingerprint` SHALL NOT depend on git config: the same two commits SHALL give the same fingerprint whatever `core.abbrev`, `core.quotePath` and `diff.*` settings the repository has.

#### Scenario: git config ignored
- GIVEN a branch that renames a text file with a one-line edit, changes a binary file, and adds a file holding Latin-1 bytes
- WHEN `clm fingerprint --base main` runs once with no `core.abbrev` or `diff.*` settings, and once with `core.abbrev=16`, `core.quotePath=false`, `diff.renames=copies`, `diff.algorithm=patience`, `diff.noprefix=true` and `diff.context=8` in the repository's config
- THEN both print the same fingerprint

### Requirement: fingerprint-stable
The fingerprint SHALL be unchanged by rebasing onto a `<ref>` that touched only other files and, on a branch that adds a change, by a sync onto a `<ref>` that changed `specs/` after which `specs/` is regenerated from the same deltas and assets (`derived-specs-fingerprint`); it SHALL change when the mode, type or object ID at `HEAD` changes for any path that counts as `gate.fingerprint/fingerprint` defines it (a path outside `specs/` on a branch that adds a change, any path on one that adds none), a path `<ref>` also changed included.

#### Scenario: pure rebase
- GIVEN a branch that adds no change and edits `specs/a/SPEC.md`, and a new commit on `main` editing only `specs/b/SPEC.md`
- WHEN the branch is rebased onto `main`
- THEN `clm fingerprint --base main` prints the same fingerprint as before

#### Scenario: content changed
- GIVEN a branch that adds no change and edits `specs/a/SPEC.md`, with one more commit changing a line in `specs/a/SPEC.md`
- WHEN `clm fingerprint --base main` runs before and after that commit
- THEN it prints a different fingerprint after it

#### Scenario: mode changed
- GIVEN a branch that adds `other.txt`, with one more commit that makes `run.sh` executable and changes nothing else
- WHEN `clm fingerprint --base main` runs before and after that commit
- THEN it prints a different fingerprint after it

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
