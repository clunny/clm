---
title: "A passing check proves less than the spec claims, and the spec keeps growing"
kind: idea
components: [clm]
blocks: [capability/measure-by-evals]
---

## Intent

Bring CLM into line with its own philosophy: AI writes and maintains all code and tests, the human owns only intent and constraints, and a green check means the code does what the spec says. This item is the plan for that work. It is elaborated here across sessions; each phase is proposed from it as ordinary changes and built by the build loop.

A session working on any phase first reads `docs/the-clm-project.md`, the philosophy and goals every decision here is judged against: humans own intent only, two human roles, three layers, and the intended size of the human layers. It dates from 2026-10-29; where it differs from `ARCHITECTURE.md` or from this item, those are current, until phase 1 brings it up to date.

## Problem

An audit of `main` at `09b74b03` (2026-11-13) found the philosophy coherent and the mechanical core (land, gate, ledger, model, guard parsing) well tested, but four things wrong.

- **Proof is a marker.** A requirement reads as current when a `req:` string with the right hash exists in any non-Markdown file. Nothing checks that it sits in a test, that the test ran, or that it asserts anything: CLM's own capability test for `prove-by-tests` gets `current` from an empty test function. A `behind` requirement becomes current by bumping the hash. The safeguards are prompt text or switched off: mutation spot-checks are unrecorded, nothing records that the code review or the spec audit ran, and the `unexercised` signal has no tracefile.
- **About 130 requirements are satisfied by a sentence existing in a prompt or skill.** Their tests check for string fragments. Several architectural constraints list them under `#### Enforced by`, among them `path-ownership`, which no gate or land check enforces. This contradicts `deterministic-enforcement`.
- **The growth the 2026-10-29 re-founding meant to stop has recurred.** 932 requirements against 454 at the collapse, rising by about 28 a day with no day of decrease; 57 components against "no new components"; 24 of them over budget, the root's bundle and code too large for one session. Roughly 60 to 70% of the 183 changes since 2026-11-03 patch CLM's own process. The brakes are advisory and unused: every capability and constraint reads "never decided".
- **Mechanics have moved into the human layer.** `ARCHITECTURE.md` grew 74% in ten days and is prepended to every context bundle. Entries such as `path-ownership` (828 words) name files and exceptions the human must approve.

The documents also contradict themselves in places: the release workflow pushes `main` against `land-locally`; `[verify] mode = "pipeline"` runs checks on the host against `platform`; the guide's FAQ says Claude Code only; accepted decisions (`node-reconciliation`, `node-size-budgets`) say the opposite of constraints that cite them; nine superseded decisions are still cited from live specs and skills; the workaround limit is per requirement in one place and per capability in another.

Five root causes produce nearly all the workarounds: one-component builds against a compile-coupled workspace with a shared binary test crate; a spec/code branch split too strict for small fixes (eight routes onto `main` now exist); a guard that parses shell strings doing permission enforcement; marker-equals-proof; and parallel land checks on one loaded machine.

## Settled (2026-11-13, with the human)

- **The freeze is on.** Until this plan is done, the build loop and triage work only on it. Defects are filed, and their default answer is "accept".
- **CLM has adopters.** Projects run published releases, so `upgrade` and its migrations stay, and every change in this plan that alters a file, key or format those projects hold ships its migration (`upgrades-carry-migrations`).
- **`clm land` stays fast.** It gains only checks that take milliseconds. Anything slow runs after the land.
- **Scope kept.** The web app, Azure DevOps, Codex, the site, blog and analytics, `right-model-per-task` and `watch-machine-load` are in use and stay.
- **Scope cut.** The `clm runner` command and its component `orchestrator.runner` go: the human runs unattended builds through `/clm:build` in a session. The capability `unattended-runner` folds into `build-from-spec`.
- **`pin-working-method` becomes an architectural constraint.** It states what must always hold, not something a person does. No behaviour changes.
- **Prompt-text requirements are deleted with their tests.** Skills and prompts are source files their component owns. A requirement stays only where it tests behaviour (the plugin packages, hooks are wired, the shim finds the binary). An instruction that must hold moves into a program. Edits to skill and prompt files are made visible in the pull request summary.
- **Three kinds of rule, labelled as such.** *Checked*: a program decides it; preferred wherever possible, including for model behaviour, since refusing the bad outcome beats measuring compliance. *Measured*: an eval with a threshold, for behaviour that comes from a model or is statistical by nature. *Principle*: design judgement, which may be audited for advice. "Enforced" only ever means checked.
- **Evals are a second kind of proof in the method.** A requirement declares test or eval proof. The project names its eval command as it names its land check, or has none; CLM names no tool and carries no adapter. Using an eval for a requirement, and its threshold, need the human's yes, since it spends money on a recurring basis. The builder check asks of each such requirement why a program can't decide it. An eval never holds a land.
- **Eval selection follows what the eval reads.** A changed requirement makes the eval citing it behind, which is build work, as for a test. An eval is due to run only when a file it reads, its own definition or the model changed; otherwise its last result carries forward. Spec, capability and architecture text trigger nothing directly. Each run has a spend limit, orders what is due within it, adapts its trials to how borderline a result is, and says why each eval ran and what it cost. A full run before a release is the safeguard against an undeclared read.
- **Evals keep their own schedule.** Apart from verify: nightly for what is due, a full advisory run before a release, and on demand. A result below threshold files a bug on the owning component.
- **CLM's own evals.** `claude plugin eval` for the conversation skills (propose, init, triage) and the subagents. The build skill is measured from real builds: ownership refusals at land and mutation survivors in verify. One advisory judge audit to start: whether new requirements state observable behaviour.
- **What is kept is decided by its kind, not by who asked.** Observable behaviour with a test stays, whether a person asked for it or an AI session proposed it. Implementation detail goes: function names, field names, scans of source text. So the prune regroups detail as scenarios under fewer requirements and shortens the text, and the number of stated behaviours stays about the same; behaviour is reduced at the capability level, by the human's decision.
- **Removing behaviour always needs the human's yes.** A scenario, a requirement, a capability or an architectural constraint that states behaviour is never dropped without it. This holds after the plan too. The human reads no pass over what is kept or added: their only pass is over cuts.
- **The AI proposes cuts, and never drops or carries a burden in silence.** A session or a check that sees something should probably go (a duplicate, a scenario nothing uses, a requirement that keeps being rewritten or worked around, a capability whose cost outruns its use) surfaces it as a proposed cut: what it is, the evidence, what would be lost in plain words and what it would save. Triage offers the proposed cuts, grouped by capability and largest saving first, a few at a time. On a yes the AI drafts the removal and `no-dead-code` takes the code. A keep is recorded with what it was decided against, and the cut isn't proposed again until that evidence changes. Proposed cuts hold nothing up and are not shown as waiting on the human.
- **A small thing found in use takes a light route.** When a person asks in a session for a detail they would mind losing, such as a colour or a view that fills the screen, the session adds one scenario to an existing requirement, with its test and code, on one branch that lands on checks; the person's asking is the intent. A program keeps the route small: the branch adds only scenarios, to requirements of one component, with its code in that component. Anything more is an ordinary change.
- **Signed approvals stay off here.** An ordinary key on disk is as reachable to a session as to the human, a touch key would make every unattended land wait for a hand, and a signed repository loses approval from the web app. The gap is accepted in writing: the guard prevents accidents, and `clm ledger verify` audits after the fact. `guard-sessions` is reworded to say so.

## Plan

**0. Baseline.** Record the counts of the day the freeze began, to measure the plan against.

**1. Rewrite the human layer by hand.** One spec branch, one approval.
- `ARCHITECTURE.md`: each constraint one or two sentences, labelled checked, measured or principle; mechanics move down into decisions.
- `CAPABILITIES.md`: fold the runner, move the pin, add the eval capability, reword scenarios the product's boundary can't observe.
- Reword `proven-by-tests` and `deterministic-enforcement` for the three kinds.
- Remove the contradictions listed under Problem.
- Last, once the human layer is settled: bring `docs/the-clm-project.md` up to date with it (the supported agents, the ways the AI layer reaches a person, the three kinds of rule, evals, what this plan changed and why), and revise the opening of `AGENTS.md`, "What CLM is for", added on 2026-11-13 so every session has the philosophy and goals in front of it, where the settled layer changed them (the three kinds of rule and evals among them). That opening stays short, since every session and every build loads the file, and it points at the article for the reasoning, never restating it. The plan's last phase removes its sentence naming this item.

**2. Delete.**
- `clm runner` and `orchestrator.runner`.
- The prompt-text requirements and their tests.
- The superseded decisions; fold the amended ones into one text each.
- Stale workaround flags and detector allowances, closing the test gaps behind them: `workaround-gate-classify-1` and `workaround-web.views-home-waiting-on-you-1` name blockers that are built, and `clm_upgrade::check_lock`'s stated condition is met.

**3. Make proof real.**
- `clm check`: a marker counts only inside a test, and only when that test passed in the check the build already ran on its branch head.
- `clm land`: a diff outside the files of the branch's component is refused.
- Verify: for each requirement changed or re-cited since the last run, mutate the changed code of its component and run its citing tests; when they kill nothing, the requirement returns to pending with a bug on the component. The coverage script runs in the same pass.

**4. Fix the repository's shape.** Each crate proves its requirements through its own library entry point, and the binary's tests cover dispatch only (`backlog/clm-s-cli-tests-prove-other-components-through-the-binary`). This retires `path-ownership`'s transitional exceptions and the decisions stacked on them.

**5. Prune the implementation spec.**
- One rule for a requirement: observable behaviour at its component's Contract, with no function names, field names or scans of source text.
- The spec-slimmer runs on every component against it.
- The eight routes onto `main` collapse to about three, the light route for one scenario among them.

**6. Add evals**, as Settled states them: the method first, then selection, then CLM's own evals as its first user, then the one judge audit.

**7. Brakes that gate.** A component over budget takes no new requirement until it is slimmed or split. A workaround flag expires when its blocker is built. Proposed cuts reach the human through triage, as Settled states them, fed by sessions, the spec-slimmer, the health signals and the cost records. This follows the pruning, or it would stop every build today.

**8. Validate on a second project.** A small project outside this repository, to tell the method's defects from those of this repository's shape.

## How it is executed

- **This item is the plan of record.** It lives in git, so no session's memory or conversation holds anything the next one needs. A session that settles something rewrites the sections here.
- **Each phase is one or a few ordinary changes that names this item in its `## Intent`**, and their code is built by `/clm:build`. No change but the last phase's names it in `from` or `resolves`: `clm apply` deletes the item a change's `from` names (`apply-deletes-promoted-items`), which would delete the plan with its first phase. Progress is CLM's own state: `clm status` shows which changes are applied, pending or archived. No checklist is kept by hand; when a phase's changes have all landed, its paragraph under Plan is deleted.
- **A fresh session per phase.** It starts from this item and `clm status`, proposes that phase, and ends when the phase's spec branch has landed. Builds then run in the loop, unattended.
- **Phase 1 is the exception in kind, not in route.** It is written by hand with the human in the conversation, since only they can settle the human layer, on one spec branch through the same `clm land`.
- **Order between phases is by `#### Needs` and priority, as for any work.** A change that relaxes a check lands before one that relies on it (`checks-relax-before-use`).
- **Where CLM can't carry a change through itself, that is a finding.** It is reported with `clm defect`, and the step is done by hand on a branch that still lands through `clm land`.

## Safeguards against losing behaviour

This plan deletes requirements on purpose, and code that serves no written requirement is removed by CLM's own rules (`no-dead-code`). The 2026-10-29 re-founding lost behaviour that way. These hold for every phase.

1. **A baseline before anything is deleted.** The commit the freeze began on is tagged, and `.github/scripts/coverage.sh` is run on it once, so `clm health`'s `unexercised` signal lists the code no test runs today. Before phase 5, each run of such lines gets one of three answers: it is behaviour someone relies on, so a scenario states it and a test proves it; it is dead, so it is deleted knowingly; or it is plumbing, such as an error path, that the component's Design covers. On the baseline day 4,257 Rust test functions stood against 4,296 markers, with 15 Rust files and 4 of 54 TypeScript test files holding tests that cite nothing, so the unstated part lies in code no test runs, not in uncited tests. The script ran on 2026-11-13 at the code of `09b74b03`: of 41,148 executable lines in 275 files, 2,565 (6.2%) are run by no test, in 1,274 runs of which 14 are ten lines or longer. Over a third lie in the web app's entry file `web/src/main.ts` (568) and the panel's `plugin-panel/hooks/register.tsx` (190); the web replica and platform crates hold 429 more. The suite's nested WebAssembly builds aren't instrumented, and a line a test runs is not a behaviour a test proves, which phase 3's mutation job measures.
2. **A test outlives its requirement's wording.** In phases 2 and 5 a test of real behaviour is re-cited to the requirement that survives, never deleted because the text got shorter. Only a test that solely scans prompt or source text goes.
3. **Every branch that deletes tests lists them.** A program produces the list from the diff, and the human reads it before the land.
4. **A change that removes behaviour needs the human's approval**, as Settled states, from the plan's first deletion on. Such a change lands on checks today. One approval per phase, with the list from point 3.
5. **A mixed requirement is split before deletion.** The part that states behaviour stays a requirement; only the check that a sentence exists goes.
6. **A use checklist, run before and after each phase.** What the human actually uses: approving from the phone in the web app, the panel, Azure DevOps, Codex, the overnight build. Capability tests fake the model and would miss some losses.

## Done means

- An empty test no longer reads as proven.
- No requirement is satisfied by a scan of prompt text, and no constraint lists one as enforcement.
- No component is over budget, and the requirement count is well below 932.
- `ARCHITECTURE.md` is under a third of its size on 2026-11-13, every entry labelled.
- Every workaround flag is settled, and the documents hold none of the contradictions listed under Problem.

## Open questions

- **Mutation tool.** Whether `cargo-mutants`' diff-scoped mode fits the verify job hasn't been checked.
- **Eval results.** Whether `claude plugin eval` returns a case's tags in its JSON isn't documented; the wrapper may have to map case names to requirement IDs.
- **How a program tells a removal of behaviour from a rewording.** The gate can see a removed requirement, a requirement with fewer scenarios and a deleted test, but not whether a merged scenario still states what two did. Which of these makes a branch need review is open.
- **What a proposed cut is on disk.** A `question` item would show as waiting on the human, which a proposed cut must not. Whether it is a backlog kind of its own, and where a keep is recorded, are open.
- **The light route's gate rule.** A branch holding spec and code is `mixed` today and refused. The exact bound (how many scenarios, whether a Contract may change) is open.
- **A hard approval boundary.** A separate touch key for approvals alone, with an answer for the web app, is left for when others join the project.
