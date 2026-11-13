---
title: "A spec change needs review or it doesn't; nothing is reviewed after it lands"
date: 2026-10-29
status: accepted
---

Refines `humans-review-intent` (`docs/the-clm-project.md`, `changes/archive/20261029-refounding/design.md` §5). The three review tiers had humans read every change to the implementation rules: a modified requirement waited for approval (`HARD`), a new one was reviewed after it landed (`SOFT`), and a declared floor, a significance rubric and a list of significant nodes grew around them. Most of that review was of rules the human never needed to see, and what went unreviewed was what each feature cost.

So review is yes or no. A spec branch **needs review** exactly when:
- its diff touches `ARCHITECTURE.md` or `CAPABILITIES.md`, the human layers, or `docs/allowed_signers`, the keys a signed repository trusts (`signed-ledger-records`);
- it changes the `approval` setting in `clm.toml` (`approval-setting-is-spec`);
- a `change.md` it adds or modifies declares `burdens`, or can't be read: what the change spends of the human's money or time, or can't undo, one short line each.

`clm gate` prints `spec review` for such a branch, which lands only after a human's `clm approve` (`approval-policy`), and `spec` for any other spec branch, which lands on checks. Nothing is reviewed after it lands: there is no `clm review`, no `reviewed` record and no review queue. A spec branch that adds a `kind: human` backlog item and declares no burden is blocked (`gate/undeclared-burden`), since a step only the human can take spends their time; builds still file `question` and `human` items freely, and those block only their own requirements.

A `landed` record carries `review`, a bool, in place of `tier`; older records read `HARD` as true and every other tier as false. The audit recomputes the bool for each signed land, on demand with `clm ledger verify --review` and in the checkpoint, as `recomputed-land-tiers` did for the tier. Triage lands the drafts that need no review and folds those that do into one branch for one approval, as `triage-folds-hard-drafts` did for `HARD` drafts.

It amends `approval-policy`, `per-writer-ledgers` and `backlog-triage` where they speak of tiers, `## Significant`, `clm review` or `reviewed` records; the rest of them stands. This supersedes `computed-gate-tiers`, `declared-gate-floor`, `significance-rubric`, `significant-nodes`, `web-marks-soft-lands-reviewed`, `modified-delta-tiers-by-element`, `recomputed-land-tiers` and `triage-folds-hard-drafts`; `soft-review-after-merge` was already superseded by `per-writer-ledgers`, whose `reviewed` records go with it.

Rejected: keeping `SOFT` as a lighter review (it is the after-the-fact queue that piled up 31 unread items); keeping the tiers and only removing the rubric (the human would still review rules one by one); having a model judge whether a change needs review (`deterministic-enforcement`).

(Amended 2026-11-13, `20261113-temporary-exemptions`: a spec branch also needs review when it adds or widens a component's exemption from a code rule; removing or narrowing one doesn't (`temporary-exemptions`).)

## The rule in full

`ARCHITECTURE.md` stated this as the entry `humans-review-intent` until 2026-11-13, when the entry was cut to its rule and the triggers moved here (`human-layer-entries-stay-short`). The text still holds:

Humans SHALL review intent, not code. A spec branch needs a human's approval, given with `clm approve` before it lands, exactly when it edits `ARCHITECTURE.md`, `CAPABILITIES.md` or the trusted keys in `docs/allowed_signers`, changes `clm.toml`'s `approval` setting, adds or widens a component's exemption from a code rule (`exemptions-end`), or adds or edits a change that declares `human_burdens` or whose `change.md` can't be read, which fails closed. A branch whose whole diff `clm rename` reproduces from the base and the renames its change records is a spec branch, its rewritten test markers included, and its replayed edits to `ARCHITECTURE.md` and `CAPABILITIES.md` don't need review, since renaming an ID changes no intent; one that swaps a term does, since only a human can judge that a swapped word keeps a rule's meaning (`rename-by-replay`). Every other change lands on checks, and nothing is reviewed after it lands (`yes-no-review`).

**Scenario: capability edit**
- GIVEN a spec branch that edits only `CAPABILITIES.md`
- WHEN `clm gate` judges it
- THEN its first line is `spec review`, and `clm land` refuses it until a human has approved it

**Scenario: node rule edit**
- GIVEN a spec branch that only rewords a component's requirement and declares no `human_burdens`
- WHEN `clm gate` judges it
- THEN its first line is `spec`, and it lands on checks

**Scenario: exemption added**
- GIVEN a spec branch that only adds an exemption to a component's frontmatter, with the backlog item that ends it
- WHEN `clm gate` judges it
- THEN its first line is `spec review`

**Scenario: rename of an architecture entry**
- GIVEN a branch holding only what `clm rename` produced from a change renaming an `ARCHITECTURE.md` entry's slug
- WHEN `clm gate` judges it
- THEN its first line is `spec`, and it lands on checks
