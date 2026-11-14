---
component: land
---

## ADDED Requirements

### Requirement: landed-change-releases-claims
After a successful push of a `spec/*` branch, `clm land` SHALL delete on `origin`, through `host`'s `branch delete` leased on the tip its last fetch read, `propose/<slug>`, once per slug, for each item that a change the head adds names in `from` (as `backlog/<slug>`) or `resolves`, a change the head adds being one of the land's `changes` with neither `changes/<id>/change.md` nor `changes/archive/<id>/change.md` at the merge base, reading each such change's `change.md` from the merged tree at `changes/<id>/change.md`, or `changes/archive/<id>/change.md` when that is absent, through `model`'s change frontmatter parser, and write `released propose/<slug>` to stderr for each claim it deleted. It SHALL skip a slug with no `origin/propose/<slug>` on the fetched refs, a `from` that doesn't start `backlog/`, and a `change.md` whose frontmatter doesn't parse; a delete that fails SHALL warn as `landed-branch-cleanup`'s does, and SHALL NOT fail the land. It SHALL release no other claim, and none after a land of a branch that isn't `spec/*`.

#### Scenario: claims on the change's items released
- GIVEN `origin/propose/q`, `origin/propose/r` and `origin/propose/s`, and a branch `spec/c` adding `changes/c/change.md` with `from: backlog/q` and `resolves: [r]`, where `backlog/q.md`, `backlog/r.md` and `backlog/s.md` are on `origin/main` and the branch deletes none of them
- WHEN `clm land spec/c` runs
- THEN it prints `landed <sha> spec`, `origin` has neither `propose/q` nor `propose/r` and still has `propose/s`, and stderr holds `released propose/q` and `released propose/r`

#### Scenario: moved claim kept
- GIVEN the same branch and a check command that pushes a new commit to `origin/propose/q` while the land runs
- WHEN `clm land spec/c` runs
- THEN `origin/propose/q` is still that commit, it printed a warning naming `propose/q`, and it exits 0

#### Scenario: an edit of a landed change releases nothing
- GIVEN `changes/c/change.md` with `from: backlog/q` on `origin/main`, `origin/propose/q`, and a branch `spec/d` that only modifies `changes/c/change.md`
- WHEN `clm land spec/d` runs
- THEN it lands, and `origin/propose/q` is unchanged

#### Scenario: a code land releases nothing
- GIVEN `origin/propose/q` and a branch `impl/a` whose tree holds `changes/c/change.md` naming `backlog/q` in `from`
- WHEN `clm land impl/a` lands it
- THEN `origin/propose/q` is unchanged

#### Serves
- capability/work-concurrently
