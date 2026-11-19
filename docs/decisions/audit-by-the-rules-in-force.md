---
title: "The review audit judges a land by the gate's rules in force when it landed, named in its merge commit and never lower than main's"
date: 2026-11-18
status: accepted
---

**The problem.** `clm ledger verify --review` recomputes whether each signed land needed review, to catch a lander who recorded less than the gate gave (`checkpoint-recomputes-new-lands`). It recomputed with today's gate. A rule that makes more diffs need review, such as `gate/cuts`, then reports every honest land made before it that the rule would catch as understated and unapproved: 26 of this repository's last 150 spec lands would be, and a signed adopter would meet them all on the release that carries the rule. `checkpoint-recomputes-new-lands` called this *rules drift* and left a human to move the review mark past each report by hand. A finding that is right for nobody teaches people to move the mark without reading, which is the opposite of what the audit is for.

**The decision.** The gate's rules have a version, `clm_gate::RULES`, and `evaluate_by(..., rules)` gives the answer each earlier version gave (`gate/rules-version`). Version 1 is the gate before cuts made a branch need review; version 2 adds them. `clm land` writes the version it judged a land by into the merge commit as the trailer `Gate-Rules: <n>` (`land/gate-rules-trailer`), and the audit recomputes each land under that version (`ledger/verify-recomputes-review`). A change that makes the gate rate some diff higher raises `RULES` and keeps the older answer behind the version; one that rates diffs lower needs nothing, since only understatement is reported.

**Why a lander can't pick laxer rules.** The trailer is the lander's word, as the record is. So the audit judges a land by the higher of its own trailer and the highest trailer of any earlier signed land on `main`'s chain, its *floor*. Once any land has landed under version 2, no later land is judged by version 1, whatever it writes. The only window is between a release and the first land made with it, and a land in that window was judged by rules some `clm` in use still applied: that is the rule in force. A land made by a clone with an older `clm` after a newer one landed is reported if the newer rule would have asked for review; that is a real finding, since the land got less than `main` already asked.

**Why the merge commit and not the `landed` record.** The record is the natural home, but `Body::Landed` is built by struct literal in `land`, `land.approval`, `orchestrator.checkpoint` and `check.health`, so a new field would need a constructor and a move in each first (`coupled-changes-sequence`). The merge commit already carries `Change:` and `Checked:` trailers, `clm land` alone writes it, a signed `landed` record names it by hash, so the trailer can't change without the record's signature failing to name it, and the audit already reads one trailer (`Clm-By-Hand`) the same way.

**What stays.** Lands from before the trailer read as version 1. In a signed repository, a land made before widening an exemption came to need review that widened one is still reported under version 1; those are the drift `checkpoint-recomputes-new-lands` describes, handled by hand as it says. No version 0 is invented for them, since no signed adopter is known to have such a land, and reading every untrailed land by the oldest rules would weaken the audit of every land made since.

Rejected:
- **A `gate_rules` field on the `landed` record.** Same audit, but it costs a constructor and a move in four components before the field can land.
- **The `clm` version that landed, and running that gate.** The audit can't run an old binary, and a version string says nothing about which rules changed.
- **Tying rules to the project's `upgraded` stamp.** It would make the live gate's rules wait on a project taking a migration, so a project that never upgrades never meets a new rule.
- **An acknowledgement record for each drift report.** Every signed adopter would acknowledge dozens of honest lands by hand on every tightening.
- **Skipping the recomputation for lands under older rules.** A lander would then escape the audit by writing an old version.
