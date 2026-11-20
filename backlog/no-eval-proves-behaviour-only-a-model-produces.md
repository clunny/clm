---
title: "No eval proves behaviour that only a model produces"
kind: idea
components: [clm]
blocks: [capability/measure-by-evals]
---

## Intent

Build evals as the method's second kind of proof. This was phase 6 of `backlog/a-passing-check-proves-less-than-the-spec-claims`, moved out of that plan on 2026-11-20 by the human, to be taken up after it is done: the method first, then selection, then CLM's own evals as its first user, then the one judge audit. `capability/measure-by-evals` waits for it.

## Settled (2026-11-13, with the human)

- **Evals are a second kind of proof in the method.** A requirement declares test or eval proof. The project names its eval command as it names its land check, or has none; CLM names no tool and carries no adapter. Using an eval for a requirement, and its threshold, need the human's yes, since it spends money on a recurring basis. The builder check asks of each such requirement why a program can't decide it. An eval never holds a land.
- **Eval selection follows what the eval reads.** A changed requirement makes the eval citing it behind, which is build work, as for a test. An eval is due to run only when a file it reads, its own definition or the model changed; otherwise its last result carries forward. Spec, capability and architecture text trigger nothing directly. Each run has a spend limit, orders what is due within it, adapts its trials to how borderline a result is, and says why each eval ran and what it cost. A full run before a release is the safeguard against an undeclared read.
- **Evals keep their own schedule.** Apart from verify: nightly for what is due, a full advisory run before a release, and on demand. A result below threshold files a bug on the owning component.
- **CLM's own evals.** `claude plugin eval` for the conversation skills (propose, init, triage) and the subagents. The build skill is measured from real builds: ownership refusals at land and mutation survivors in verify. One advisory judge audit to start: whether new requirements state observable behaviour.

## What the plan's phases left for it

- What no program can hold of the instructions phase 2 deleted waits for the skill evals; seven `idea` items name a program that could hold one.
- **Whether an AI audit should judge tests instead** was asked on 2026-11-14. The pass decides and can't be talked round, and never reads the requirement; an audit reads it and can't be trusted to gate. For phase 6: point the one judge audit at whether each scenario has a test that would fail if the scenario were false, and consider a break the AI chooses from the requirement and a program applies.
- No program can close the class of a test that cites a requirement which doesn't say what the test asserts. The judge audit can advise on it.

## Open questions

- **`proven-by-tests` names eval proof before phase 6 builds it.** Its `#### Untested` says so. Whether the clause should wait for phase 6 is open.
- **Adopters' entries carry no label** (`backlog/adopters-architecture-entries-carry-no-label`), which belongs with phase 6's block for measured entries.
- **Eval results.** Whether `claude plugin eval` returns a case's tags in its JSON isn't documented; the wrapper may have to map case names to requirement IDs.
