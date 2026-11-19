---
title: "The propose and health skills predate the gate's list of cuts"
kind: deferred
components: [plugin]
origin: 20261118-the-pass-s-tooling
---

Two skills' fixed text says what `20261118-the-pass-s-tooling` changed, and no requirement states either (`prompt-text-is-source`), so each is a code fix under `clm claim plugin --fix backlog/the-propose-and-health-skills-predate-the-gate-s-list-of-cuts`, with no test:

- `plugin/skills/propose/SKILL.md` lists four cases in which a branch needs review. It should add the fifth, a cut (`gate/cuts`), and tell a designing session to keep a scenario's name when it rewords or moves it, to put both sides of a moved clause in one branch, to write the tests its builds delete into `changes/<id>/deleted-tests.txt`, each line a test that exists, and to name every sentence a requirement loses under "What the requirements no longer state" in `change.md`. Where it asks the human for approval (step 12), it should show the `gate/cut` lines `clm gate` printed. The triage skill and `clm-triage`'s fold prompt, which also ask for an approval, are `orchestrator.triage`'s and `plugin`'s to change the same way.
- `plugin/skills/health/SKILL.md` describes the signals and launches the `spec-slimmer` to propose cuts. It should describe `outside-contract`, `test-placement`, `design-exit` and `uncited-test`, launch the slimmer for the components they name, and say the slimmer returns four lists, of which only the cuts go to the human.
