---
title: "A requirement states what the component's Contract shows, and nothing of how the code is laid out"
date: 2026-11-18
status: accepted
---

Measured on 2026-11-18, 371 of the 891 requirements in the components stated only behaviour. The rest named a function or type (352 by a reader's count, 128 by a program that counts only names the component's own Contract doesn't hold), a field only source shows (55), where a test sits (73) or where code lives (78), had a file's text as their subject (176), or stated only an absence (55). And 214 sentences of a Contract or Design stated an exit, a message, a refusal or a failure path that no requirement of the component states, 87 of them asserted by a test that cites a requirement which doesn't say it. Four such sentences were found one at a time in `check.affected` and `check`, each by a review of a build and each mended by a spec branch and a build of its own.

**The rule.** A requirement states what someone outside its component can observe: a command's arguments, output, exit status and effect on files and git; a file format's keys; or what a call the component's Contract offers returns for what it is given. It names no function, type or field the Contract doesn't offer, no test and no place where code or tests sit, and its subject is never a file's text. Another component's behaviour is named by its requirement's ID, not by the function that carries it.

**A requirement about a file's text** stays in two cases: an entry of `ARCHITECTURE.md` names it under `#### Enforced by`, which makes it a structural check, held once and not once per component; or it states a value a program puts into a prompt (`prompt-text-is-source`). Any other is a proposed cut, which the human reads (`removal-needs-a-yes`).

**The Contract and the Design.** A Contract may summarise what the component's commands print and how they exit, since a dependant's build receives the Contract and not the requirements (`ancestor-scoped-context`); each such statement is also a requirement's. A Design says how the component works and states no exit, message or refusal. A sentence of either that states behaviour no requirement states is mended one of three ways: it is relied on, so a scenario states it and its test cites it; a requirement already states it, so the sentence goes from the Design; or nothing relies on it, or it contradicts a requirement, so it is a proposed cut.

**How it is applied.** One pass per component: one delta that holds every reword, regroup, move, newly stated scenario and cut, and one build. A pass keeps a scenario's name when it rewords or moves it, so a program can list what a branch removes by name, and its change names every SHALL sentence a requirement loses, though no requirement or scenario goes. A test may reach the behaviour through any public call of its crate: the rule is about a requirement's wording. The rule reaches every project in the guide's spec-format page (`method-in-the-plugin`). It is a principle: a program counts what it can see of it, as advice (the names a Contract doesn't hold, the words that place a test, the exit statuses a Design states, the tests that cite nothing), and whether a requirement states behaviour is a reader's judgement.

Considered:
- **No code names at all, every requirement worded through a command.** Rejected: it rewrites about 350 requirements, and `vcs`, `model` and the ledger's fold hold behaviour no command shows. A library component's calls are what its dependants observe.
- **Working one fault class across every component.** Rejected: each component would be built four to six times, nearly every build a re-cite.
- **A program that re-cites a requirement whose scenarios are unchanged.** Rejected: it makes a requirement current by rewriting its marker, the route `markers-count-in-tests` closed, and no program can tell a shortened statement from a widened one.
- **Failing `clm check` on a test that cites nothing.** Rejected as a gate: it pushes a helper's unit test to cite a requirement that doesn't say what the test asserts, which is the fault this decision mends. The count is advice.
