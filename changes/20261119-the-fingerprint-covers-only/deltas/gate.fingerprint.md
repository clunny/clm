---
component: gate.fingerprint
---

## MODIFIED Requirements

### Requirement: fingerprint <!-- base: 230296cd57f84bad -->
`fingerprint(repo, base)` SHALL return, and `clm fingerprint --base <ref>` SHALL print, `fingerprint_of` over the entries of `git ls-tree -r -z <merge-base>` and `git ls-tree -r -z HEAD`, both run in `repo`, where `<merge-base>` is the merge base of `<ref>` and `HEAD`. Those listings SHALL hold the entries under `repo` alone, each path relative to `repo`, so in a CLM repository that is a subdirectory of its git repository a path outside that subdirectory never counts, and `specs/` and `changes/` below are `repo`'s own (`fingerprint-covers-the-clm-root`). `fingerprint_of(base, head)` SHALL return the lowercase hex SHA-256 of these bytes, for each path that counts, in ascending order of the paths' bytes: the path's bytes as listed, a NUL byte, the path's entry in `base` as the text `<mode> <type> <id>` with single spaces, or the one byte `-` when `base` has no entry for it, a NUL byte, its entry in `head` the same way, and a NUL byte. A path counts when that text differs between the two listings or one listing lacks the path, leaving out every entry whose type is `tree` and, when `head` adds a change, every entry whose path begins with the bytes `specs/`. It SHALL return `empty`, which the command prints, when no path counts. `added_changes(base, head)` SHALL return, in `head`'s order, the path of each entry of `head` whose type isn't `tree`, that `base` has no such entry for, and that is `changes/<id>/change.md` with `<id>` other than `archive`, or `changes/archive/<id>/change.md` while `base` has no entry `changes/<id>/change.md`, so an archive move adds none; `<id>` is one non-empty path component, so `changes/archive/change.md`, `changes/x/deltas/change.md` and `changes//change.md` add nothing. `adds_change(base, head)` SHALL be true exactly when `added_changes` returns a path, and `head` adds a change exactly then. `parse_ls_tree` SHALL split the listing at each NUL byte and read each piece that holds a tab, and whose bytes before its first tab are UTF-8 holding at least two spaces, into an `Entry`: the mode, type and ID are the text before the first space, between the first two spaces, and from there to the next space or the tab, and the path is every byte after the first tab, unchanged, though it be empty. It SHALL skip every other piece, the empty one after the last NUL among them, and SHALL read a last piece that no NUL ends as it reads any other. `fingerprint(repo, base)` SHALL return an error when `<ref>` and `HEAD` have no merge base or a tree can't be listed. `fingerprint_of` SHALL run no process, and `clm-fingerprint` SHALL build for `wasm32-unknown-unknown`.

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

#### Scenario: CLM root in a subdirectory
- GIVEN a git repository whose CLM root is its subdirectory `tool/`, and a branch that adds no change and changes `tool/a.txt` and `b.txt`
- WHEN `fingerprint` runs for `tool/` and `main`, and again after one more commit on the branch changing only `b.txt`
- THEN both times it returns the SHA-256 of `a.txt`, NUL, `100644 blob <old id>`, NUL, `100644 blob <new id>`, NUL, as hex

#### Scenario: no merge base
- GIVEN a repository whose branch `lonely` shares no commit with `HEAD`
- WHEN `fingerprint(repo, "lonely")` runs
- THEN it returns an error

#### Scenario: builds for the web
- GIVEN the workspace with the `wasm32-unknown-unknown` target installed
- WHEN `clm-fingerprint` is built for that target
- THEN the build succeeds
