---
title: "CLM's terms keep their usual meaning or say what they name; the agreed renames"
date: 2026-11-01
status: accepted
---

`plain-terms` asks that CLM's names read right to a seasoned engineer and to a newcomer. Read against it, these terms fail, and are renamed:

| Today | New | Why |
|---|---|---|
| `clm sync` | `clm rebase` | Its core is the rebase onto `main`; regenerating `specs/` and running the checks are what make that rebase correct. "Sync" suggests a pull and push. |
| `clm checkpoint` | `clm wrap-up` | It cleans up landed work and says whether the session can end; "checkpoint" suggests a save point. The capability is already `wrap-up-session`. |
| page (`clm page`) | pull request (`clm pr`, or a name the rename change settles) | What GitHub, Bitbucket, Gitea and Azure DevOps call it; the glossary notes GitLab's "merge request". Without a host there is none, so the term never names anything else. "Branch page" was weighed and rejected: still CLM's own word. |
| status `specd` | `unbuilt` | Not a word; it means specified, not yet proven by a test. |
| spine | ancestors | Graph jargon for the path from the root down to a component. |
| `friction` (command and backlog kind) | `workaround` | CLM already calls these "open workarounds" (`clm capabilities decide`). |
| `burdens` | `human_burdens` | It lists what a change asks of a human. "Human costs" was rejected: it reads as a count of humans. |
| `kind: human` | `kind: human-task` | `human` reads as a person, not a step only a person can take. |
| architecture requirement, `### Requirement:` in `ARCHITECTURE.md` | architectural constraint, `### Constraint:` | Frees "requirement" to mean a component's rule alone. The file name already says "architectural", so the heading is the bare word. |
| frontier stub | placeholder component | Plain words. |
| node | component | The standard architecture term (C4, UML) for a unit with its own spec, Contract, Design and owned paths, in every project CLM is used in. IDs such as `check.coverage` don't change. |

Kept, since they already carry their usual meaning or say what they do: land, gate, claim, apply, delta, change, ledger, capability, Contract, Design, current, behind, obsolete, pending, triage, health.

How the renames are carried out (`20261101-plain-terms`): the rule, the glossary and this map land first; each rename is a follow-up change of its own (`rename-commands`, `rename-states-and-keys`, `rename-node-to-component`, `rename-architecture-constraints`), waiting where it helps for `clm rename` (`clm-rename`), which renames what CLM owns and re-points the tests that cite it. Rejected:
- all renames in one change: a very large spec diff that turns most requirements `behind` at once and collides with builds in flight;
- each rename argued afresh later: reopens what the human has decided;
- a hand edit outside CLM: rewording requirements changes their versions and leaves `main` red, `ARCHITECTURE.md` needs approval, and only `clm land` pushes `main`.

No project but CLM uses CLM yet, so no adopter needs a migration and the glossary holds only current terms, with no list of old names. CLM's own sessions, skills and release binary still call commands by name, so a command rename keeps the old name as an alias, marked `clm:remove-by`, until its contract change. The retired-terms test (`clm/retired-terms`) keeps its own list of old names, filled as each rename completes, and isn't a `clm check` rule, since an adopter's project may have its own "sync" or "spine".

(Amended 2026-11-04, `20261104-retired-terms-plain-words`: the retired-terms list keeps an old name only in a form ordinary English never takes (a command after `clm `, an outcome line, a flag, a key, a `kind:` in backticks, a coined term of several words, or a coined non-word such as the old name of the status `unbuilt`). The two old names that are plain English words, for the `workaround` kind and for ancestors, stay retired only in those forms, so people can keep using the words in their everyday sense. The test no longer reads `changes/`, since a change that describes a rename has to name the old word.)
