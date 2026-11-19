---
status: applied
resolves: [the-land-and-clm-coverage-can-disagree-about-a-renamed-test]
---

## Intent

`clm land` and `clm coverage --deleted-tests` give one verdict on whether a branch lost a citing test: the land reads every file of the two trees, as `clm coverage` does, not only the files the branch touches (`backlog/the-land-and-clm-coverage-can-disagree-about-a-renamed-test`). This reverses the edge case `20261118-renaming-a-citing-test` accepted ("a new name that only an untouched file's test carries at the base counts as new for the land and not for `clm coverage`").

## Problem

- `check.coverage/test-lists` counts a gone test as *renamed* when its file at the head holds a citing test whose bare name no marker-holding test of `base_files` carries. Which names count as new so depends on what the caller passes.
- `clm coverage --deleted-tests` passes the whole tree; `clm land` (`land/deleted-tests-approved`) passes only the files the branch adds, changes or deletes. So when untouched `tests/u.rs` holds a citing test `kept` and a branch deletes `two` (citing `a/x`) from `tests/t.rs` and adds a `kept` citing `a/x` there, `clm coverage` lists `tests/t.rs::two` as gone and `clm land` lands it with no `deleted-tests.txt` naming it.
- The land's Design reasons that an untouched file "holds the same tests on both sides", true for the count of each name and false since the rename match: an untouched file's names decide whether a touched file's name is new.
- No requirement states which files the land passes beyond "the files the branch adds, changes or deletes", which is what makes it disagree, so the fix is a spec change (`code-fix-needs-a-written-rule`). No change or code fix since 2026-11-12 touches it: the item was filed today from the build of `check.coverage/test-lists`, and `20261118-renaming-a-citing-test`, which made the match, accepted the difference.

## Approaches

- **Taken: the land reads every file of both trees**, as `clm coverage --deleted-tests` does. `deleted_tests` stays as it is, the two callers pass the same kind of input, and an untouched file adds the same tests to both sides, so it makes no test gone and only informs which names are new. One requirement's wording and one Design paragraph change; the code changes one filter in `crates/clm-land/src/deleted_tests.rs`.
- **Rejected: `deleted_tests` takes the base's names apart from the files it compares.** A third argument on a library function two callers use, for what passing the whole tree already gives.
- **Rejected: judging a new name by its own file only** (no marker-holding test of that file at the base carries it). Both callers would agree, but it opens a hole: a branch that moves `kept` from `tests/u.rs` into `tests/t.rs` and deletes `two` there would count `kept` as moved and as `two`'s new name at once, one head test standing for two base tests.
- **Rejected: accepting it again.** The case is rare, but the land is the check that gates and `clm coverage --deleted-tests` is how a session learns what the land will refuse; a branch the land passes while `clm coverage` lists its deleted test leaves the session no way to know what `removal-needs-a-yes` asks of it.

## Outcome

- `land/deleted-tests-approved` (modified): `deleted_tests` runs over the texts at the merge base and at the head of every file of the two trees; its refusal's advice says the new name is one "no citing test carried"; a scenario *a new name another file carries* is refused, as `clm coverage --deleted-tests` lists it.
- `land`'s Design: the **Deleted tests** paragraph reads both trees whenever the diff touches a file, and says why reading only the touched files was wrong.

## Constraints

- `deleted_tests`' signature, return and rules stay as they are; `clm coverage --deleted-tests` changes in nothing.
- Every scenario of `land/deleted-tests-approved` as it stands keeps its outcome.
- A branch whose diff touches nothing still reads nothing.

## Edge cases and workarounds

- Reading both trees costs reading every blob twice on each land and `--check-only` run (a few thousand files here, under a second), where the touched files took milliseconds. Accepted: `clm coverage` pays the same, and it runs before the land's lock, so it holds no other land up.
- A file that isn't UTF-8 is skipped on both sides, as now.

## Open questions

None.
