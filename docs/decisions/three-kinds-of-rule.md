---
title: "A rule is checked, measured or a principle, and only a checked rule is enforced"
date: 2026-11-13
status: accepted
---

`deterministic-enforcement` said every rule that must hold is checked by a program. The audit of 2026-11-13 found about 130 requirements satisfied by a sentence existing in a prompt or a skill, several of them listed under `#### Enforced by`, and architectural constraints called enforced that no program decides. The word had come to cover three different things.

Now every rule is one of three kinds and says which.

- **Checked.** A program decides it: the CLI, a hook or the land checks. It is preferred wherever a program can decide, model behaviour included, since refusing the bad outcome beats measuring how often a model complies. "Enforced" only ever means checked.
- **Measured.** An eval against a threshold the human agreed, for behaviour that comes from a model or is statistical by nature. Using an eval for a requirement, and its threshold, need the human's yes, since it spends money on a recurring basis. An eval never holds a land: a result below its threshold files a bug on the component that owns the requirement. The method for evals is built in a later phase of `backlog/a-passing-check-proves-less-than-the-spec-claims`; until then no rule is measured.
- **Principle.** Design judgement, which a review or an audit may advise on, and which never gates.

In `ARCHITECTURE.md` the label is the first word of each entry's statement. `#### Enforced by` lists only programs: a requirement whose test scans the text of a prompt or a skill for a sentence is not enforcement, and none is listed. An entry that is part mechanism and part judgement takes the label of its rule and says under `#### Untested` what no program decides. A block for measured entries waits for the eval method.

Rejected: keeping one word for all three. It let a constraint such as `path-ownership` read as enforced when only a prompt kept it, which is the gap the plan exists to close.
