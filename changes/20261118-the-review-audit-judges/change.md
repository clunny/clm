---
status: applied
resolves: [the-review-audit-judges-earlier-lands-by-today-s-rule-for-review]
---

## Intent

The review audit, `clm ledger verify --review`, judges each signed land by the gate's rules as they were when it landed, not as they are today, so a rule that makes more diffs need review, such as `gate/cuts`, no longer reports every honest earlier land as understated and unapproved. The gate's rules get a version, `clm land` names it in each merge commit, and the audit recomputes under it, never under a version lower than one `main` has already landed under.

## Problem

`ledger/verify-recomputes-review` recomputes a signed land's kind and review with the gate as it is now. `gate/cuts` (from `20261118-the-pass-s-tooling`) makes a branch that removes a requirement, a scenario or a stating sentence need review; once it is built, every earlier signed land that removed one and was rightly recorded `spec` is reported `ledger/understated-land` and `ledger/unapproved-land`: 26 of this repository's last 150 spec lands would be. This repository is unsigned, but an adopter with `docs/allowed_signers` meets it on the release that carries the rule, and the review mark holds at the oldest finding, so `clm wrap-up` ends `incomplete` until a human moves the mark by hand. The same happened when widening an exemption came to need review. The `landed` record doesn't say which rules judged the land, so the audit can't tell drift from a lie.

## Approaches

- **A rules version in the merge commit's trailer, with a floor (taken).** `clm_gate::RULES` and `evaluate_by(..., rules)`; `clm land` writes `Gate-Rules: <RULES>` as the merge commit's last trailer, beside `Change:` and `Checked:`; the audit recomputes under the higher of the land's trailer and the highest trailer of an earlier signed land on the chain. The signed `landed` record names the commit by hash, so the trailer is as attributable as the record. The floor stops a lander from writing an old version to escape a rule `main` already lands under.
- **A `gate_rules` field on the `landed` record.** Rejected: `Body::Landed` is built by struct literal in `land`, `land.approval`, `orchestrator.checkpoint` and `check.health`, so the field would need a constructor and a move in each first (`coupled-changes-sequence`), four components' work for what a trailer carries.
- **Tie the rules to the project's `upgraded` stamp.** Rejected: the live gate would then apply a new rule only once a project took a migration, and a project that never upgrades never meets it.
- **Acknowledge each drift report with a ledger record.** Rejected: an adopter would acknowledge dozens of honest lands by hand at every tightening.
- **Skip recomputing lands judged by older rules.** Rejected: a lander escapes the audit by writing an old version.
- **Accept the drift.** Rejected: a finding that is wrong for everyone teaches people to move the mark without reading, and `wrap-up` would stay `incomplete` on every signed adopter's clone.

## Outcome

`gate` (`deltas/gate.md`):
- `gate/rules-version` (added): `RULES` is 2; `evaluate_by(repo, base, head, approved, rules)`, with `evaluate_at` being it at `RULES`; version 1 decides a spec branch's review without `cuts`; a version of 0 or above `RULES` is an error naming both. `#### Needs` `check.coverage/test-lists`, as `gate/cuts`, on which it relies, has.
- Contract and Design: `RULES` and `evaluate_by`, and the rule for later tightenings: a change that makes the gate rate some diff higher raises `RULES` and keeps the older answer behind the version.

`land` (`deltas/land.md`):
- `land/gate-rules-trailer` (added): every merge commit, alone or batched, ends with `Gate-Rules: <RULES>` after any `Change:` or `Checked:` line. `#### Needs` `gate/rules-version`.
- Design step 4 names the trailer.

`ledger` (`deltas/ledger.md`):
- `ledger/verify-recomputes-review` (modified): recomputes with `evaluate_by` under the rules in force, the higher of the merge commit's `Gate-Rules` trailer (1 when absent or not a whole number) and the highest such value of an earlier signed land on the chain, read whatever `--since` says; four scenarios added: a removal before the rule, under the rule, a lower trailer after a higher one, and rules newer than this `clm` (reported as `ledger/unrated-land` through the gate's error). `#### Needs` changes from `gate/evaluate-at` to `gate/rules-version`.
- Design's **Recomputed review** says how the rules in force are read.

`docs/decisions/audit-by-the-rules-in-force.md` records the choice; `checkpoint-recomputes-new-lands`' **Rules drift** paragraph now names it and keeps only the drift no version covers.

No requirement and no scenario is removed, and no sentence is cut.

## Constraints

- Version 1 is the gate as it stood before `cuts` made a branch need review. A land made before widening an exemption came to need review, that widened one, is still reported under version 1 in a signed repository: no earlier version is invented for it, since no signed adopter is known to hold such a land, and reading every untrailed land by older rules would weaken the audit of every land since. It stays the hand-handled drift `checkpoint-recomputes-new-lands` describes.
- A clone running an older `clm` that lands after a newer one has landed is judged by the newer rules; if they would have asked for review, the land is reported. That is a real finding: the land got less than `main` already asked.
- The window between a release and its first land is open: a land in it that writes the older version is judged by the older rules, which some `clm` in use still applied.
- Only kind and review depend on the version; the cuts and the violations are today's at every version, since the audit reads neither.
- The build order is `gate/rules-version` (after `gate/cuts`, in the same build of `gate`), then `land/gate-rules-trailer` and `ledger/verify-recomputes-review`, each by its `#### Needs`.

## Open questions

None yet.
