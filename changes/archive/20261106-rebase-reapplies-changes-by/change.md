---
status: done
resolves: [rebase-reapplies-changes-by-id-not-dependency]
---

# clm rebase and clm land re-apply a change after the changes its bases build on

## Intent

When one spec branch carries two applied changes and the second's deltas record bases taken from the specs the first produces, `clm rebase`'s regenerate and `clm land`'s derived-specs check apply them in ID order, so a dependent change whose ID sorts first meets a tree without the other's edits and is reported stale, with `no text` for its base, though nothing on `main` moved. The only way out was to rename the change (issue 204). This change makes the regenerate pick the order from the bases the deltas already record: of the changes left, the first in ID order none of whose recorded bases is *unknown* (found at no stage of the tree so far and in no commit of `onto`'s history of the component's `SPEC.md`), falling back to the first in ID order when every change left has one. A change's ID then decides nothing but the order of changes that don't build on one another.

## Problem

`sync/reapply`, `sync.regenerate/regenerated-tree` and its Contract, and `land/derived-specs` all state ID order. IDs come from a date and a slug that `clm draft` or a session picks; nothing states that they must sort in dependency order, and the propose skill tells a session to put a contract step in a second change on the same branch. A session that applies one change and then drafts a second against the specs the first produced gets a stale report it can only clear by renaming.

## Approaches

- **Order by known bases (chosen).** `clm-regenerate` already keeps the stages and walks `origin/main`'s history to find a base's text (`stale-delta-names-movers`); a base neither finds is one no applied tree has produced yet, which is exactly a base another change on the branch will produce. Picking the first change with no such base needs no new key, no declaration to forget and no history of the branch. Every case today's tests cover (a single change, folds whose drafts all base on `main`, a change stale against `main`) keeps ID order, so the movers, the `Merge` commits and the fold prompt read as they did. Recorded as `reapply-order-follows-bases`.
- **The order the branch applied them in.** Read from the commits that set each `status: applied`; blurred by a change set back to `proposed` and applied again, by archive moves and by a fold's one commit per change, and `clm land` would walk the merge commit's history too.
- **An `after:` key in `change.md`.** A new frontmatter key, a validation and a glossary entry, and a session that forgets it hits the same report.
- **Keep ID order and document naming in the propose skill.** A workaround for a rule the tool can follow itself (`redesign-before-workaround`).

## Outcome

- `sync.regenerate`: the Contract, a Design paragraph **Order**, and `regenerated-tree` state the order; `regenerated` sorts the `ids` it is given itself, so `clm rebase` and `clm land` share one rule. New scenarios: a dependent change sorting first, a change modifying a requirement another adds, a change stale against `main` going before the one built on it, and independent changes keeping ID order.
- `sync`: `reapply` and step 4 of the Design point at that order, with a scenario through `clm rebase`.
- `land`: `derived-specs` says `regenerated` chooses the order, with a scenario landing a branch whose dependent change sorts first. `clm-land` already passes the IDs to `regenerated`; its build only cites the new version and adds the scenario's test.
- `stale-delta-diff` keeps printing movers in ID order: the set of movers is already defined by the stages, so by applied order, and only the order they are listed in uses IDs.

## Constraints

- The order is deterministic: the same branch and base give the same order in `clm rebase` and `clm land`, or the derived-specs comparison would fail on a branch `clm rebase` passed.
- The history walk runs only for a base no stage has, at most once per component per run.

Known edge cases:
- A dependent change whose base an earlier change produced before `main` moved the element: once that earlier change is merged with `main`, its output no longer hashes to the dependent change's base, so the dependent change is reported with `no text`, as `stale-delta-diff` already states for that case; the order can't fix a base no tree produces.
- Two changes each waiting on the other (a cycle, or a base typed by hand): every change left has an unknown base, so the first in ID order goes and is reported as today.
- `docs/decisions/stale-delta-names-movers.md` and `triage-folds-hard-drafts.md` say the regenerate applies changes in ID order; both describe folds, where the order is still ID order, so they are left as recorded.
