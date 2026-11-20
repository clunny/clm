---
title: "The brakes on the spec's growth only advise"
kind: idea
components: [clm]
---

## Intent

Make the brakes gate. This was phase 7 of `backlog/a-passing-check-proves-less-than-the-spec-claims`, moved out of that plan on 2026-11-20 by the human, to be taken up after it is done. As the plan held it: a component over budget takes no new requirement until it is slimmed or split; a workaround flag expires when its blocker is built; proposed cuts reach the human through triage, fed by sessions, the spec-slimmer, the health signals and the cost records.

## What was measured

- Phase 2 left 852 requirements on 2026-11-13; `main` held 940 on 2026-11-20, with the freeze on.
- 29 of 51 components are over a budget on 2026-11-20, so the brake as first written would stop every build. Phase 5's waves make the spec larger (wave 1: 16% in spec tokens, 10% in code tokens, two more components over budget), so the count rises before this is taken up.
- The plan's Done means asked for "No component is over budget, and the requirement count is well below 932" until 2026-11-20. Neither is the waves' to deliver. The count is no target: Settled says the number of stated behaviours stays about the same.
- After wave 1, `model` (about 21,000 spec tokens) and `config` (about 15,500) stay over 12,000. `model` splits along its record kinds, each needing a crate of its own; or the human sets a budget.

## Settled (2026-11-13, with the human)

- **The AI proposes cuts, and never drops or carries a burden in silence.** A session or a check that sees something should probably go (a duplicate, a scenario nothing uses, a requirement that keeps being rewritten or worked around, a capability whose cost outruns its use) surfaces it as a proposed cut: what it is, the evidence, what would be lost in plain words and what it would save. Triage offers the proposed cuts, grouped by capability and largest saving first, a few at a time. On a yes the AI drafts the removal and `no-dead-code` takes the code. A keep is recorded with what it was decided against, and the cut isn't proposed again until that evidence changes. Proposed cuts hold nothing up and are not shown as waiting on the human.

## Open questions

- Whether the budget of 12,000 spec tokens and 40,000 code tokens is the right one. No measurement backs it; the effort records now hold what each build costs, so what a build of a large component costs beside a small one's can be read from them.
- A form of the brake that works while most components are over budget, such as one that refuses growth of a component that is over, was suggested on 2026-11-20 and not designed.
- Wave 1 offered the human almost no cut of behaviour: every clause its readers found unstated was stated. The cut that makes the spec smaller is the human's, by capability, from the cost view.
- Each behaviour is stated up to three times, in a requirement's statement, in its scenarios and in the Contract's summary. Stating it once is a change of format, with a migration for every adopter.
- **What a proposed cut is on disk.** A `question` item would show as waiting on the human, which a proposed cut must not. Whether it is a backlog kind of its own, and where a keep is recorded, are open.
