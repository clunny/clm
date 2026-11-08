---
status: applied
resolves:
  - overlapping-lands-fails-under-the-full-suite-s-load-but-passes
---

## Intent

A land that fails to fetch `origin` only because another land in the same clone moved a remote-tracking ref under it is not offline: it tries the fetch again, and says it is offline only when `origin` can't be reached.

## Problem

`clm::land overlapping_lands` and `clm::land held_through_the_gate_on_the_merge` (`crates/clm/tests/land.rs`) failed under the full suite's load, each time with the second land printing `waiting for another land's check on .../.clm/land: 1 ahead` and then `clm land: offline: origin can't be reached; push and land when it can`. Both pass alone.

The fault is in `clm land`, not the fixture. Run 120 times at once under load with `Repo::fetch` printing the error it swallows, the test failed 4 times, each with:

```
git fetch origin: error: cannot lock ref 'refs/remotes/origin/clm/ledger/<person>/<clone>': is at <x> but expected <y>
```

Linked worktrees of one clone share its refs. Once the first land has pushed `main` it releases the lock (`land-target-lock`) and appends its `landed` record, pushing its ledger branch, which moves `refs/remotes/origin/clm/ledger/...`; then it deletes the landed branch on `origin`, which removes another remote-tracking ref. The waiting land takes the lock and re-runs step 1, whose `git fetch origin` read the old value of that ref and finds it moved when it comes to update it. Git refuses the update, the fetch exits non-zero, and `Repo::fetch` calls every fetch failure being offline. `origin` was reachable all along. The same race hits any two lands, drafts or builds landing from one clone, which is how this repository works every day; load only widens the window.

## Approaches

- **Try a failed fetch again, then ask whether `origin` answers (chosen).** `Repo::fetch` runs `git fetch origin` up to three times, waiting 1 second and then 2, the same waits `land-push-retry` uses for a refused push. A lost ref race clears once the other land's push is done, so the refetch reads the new value and succeeds. When the last try still fails, `git ls-remote origin refs/heads/main` decides the message: when it fails, the land is offline as now; when it succeeds, `origin` is reachable and the land exits 1 with git's own message, so a local fault (a corrupt or locked ref) is no longer misreported as being offline. The same path serves `clm reconnect`'s fetch. It costs a truly offline land 3 seconds before it says so.
- **Recognise git's `cannot lock ref` text and retry only that.** Rejected: it ties `clm land` to git's wording, which varies across versions and locales, and leaves other transient local failures reported as offline.
- **Serialise every ref-moving git call in the clone under a lock.** Rejected: the ledger push, the branch deletion and the fetch run in other crates and other commands (`clm branch push`, `clm claim`, plain `git`), and none of those would take the lock, so it couldn't close the race.
- **Fix the test fixture.** Rejected: nothing in the fixture causes the failure, and two real lands in one clone fail the same way.

## Outcome

- `land/offline-land` gains the retry and the narrower `offline`, with two scenarios: a fetch that fails once with `cannot lock ref` and then succeeds lands, and a fetch that keeps failing while `origin` answers exits 1 with git's message and no `offline`. Both run the `clm` binary with a fake `git` first on `PATH`, as `land-push-retry`'s *pushed though git failed* does.
- The Design's **Offline** paragraph says why a failed fetch is tried again and how the land then tells offline from a local failure.
- `overlapping_lands` and `held_through_the_gate_on_the_merge` need no change: they pass once the waiting land's fetch survives the race.

## Constraints

- The scenario *offline with a remote refuses* still holds, 3 seconds slower.
- `push_main`'s own fetch after a refused push (`land-push-retry`) is unchanged: it already fetches directly and reports the refused push.
- Edge case: a fetch that fails because `origin` dropped out between the fetch and `ls-remote` but answers again for `ls-remote` is reported with git's message rather than `offline`. Either message tells the caller to run `clm land` again.
