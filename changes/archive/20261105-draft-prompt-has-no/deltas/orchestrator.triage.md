---
component: orchestrator.triage
---

## MODIFIED Requirements

### Requirement: draft-answers <!-- base: ebde7319f9d30bcb -->
`clm draft --answers <file> <slug>...` SHALL read the file, a relative path against the directory it runs in, before fetching, taking a place or claiming any item, and when the read fails SHALL exit 1 naming the path, claiming nothing and leaving no place. When the file holds more than whitespace, the prompt SHALL hold its text, verbatim and trimmed, under `## The human's answers`, after the items and before the steps, and the steps SHALL say that the answers are the human's, given in the triage session; that the session designs from those that bear on its items, an answer outranking the text of the item it answers but never `ARCHITECTURE.md`, a decision or a requirement; that an item the answers make designable is designed, not left; that every answer it used is quoted in `change.md` under `## The human's answers`; that a `question` or `human-task` item the answers settle stays in `resolves`, the fact it waited on going into the specs the change edits; and that an item the answers only edit, leaving it open and still waiting on the human (such as a `human-task` item whose done steps the human has trimmed out), is left out of `resolves`, since a change deletes the items it resolves when it is archived: the session edits its `backlog/<slug>.md` on `spec/<id>` to what the answers leave of it, says in `## Intent` what it changed and that the item stays open, and takes the backlog-only route (`draft-backlog-only-change`) when nothing else is left to design. Without `--answers`, or when the file holds only whitespace, the prompt SHALL NOT contain `The human's answers`. `--answers` SHALL conflict with `--finish` and `--fold`.

#### Scenario: answers in the prompt
- GIVEN untaken `question` item `q` and a file `answers.md` holding `q: use the default`
- WHEN `clm draft --answers answers.md q` runs
- THEN it exits 0, and the prompt file contains `## The human's answers`, then `q: use the default`, then `## Steps`, in that order, and the text after `## Steps` contains `given in the triage session`, `outranking`, `designed, not left`, `quoted in` and `stays in`

#### Scenario: an answer that only edits an item
- GIVEN untaken `human-task` item `h` and a file `answers.md` holding `h: trim the done steps and keep the rest open`
- WHEN `clm draft --answers answers.md h` runs
- THEN it exits 0, and the text after `## Steps` contains `only edit`, `left out of`, `stays open` and `draft-backlog-only-change`

#### Scenario: unreadable answers
- GIVEN untaken item `a`, no places, and no file `missing.md`
- WHEN `clm draft --answers missing.md a` runs
- THEN it exits 1, stderr names `missing.md`, `origin` has no `propose/a`, and the clone has no place

#### Scenario: no answers
- GIVEN untaken item `a`
- WHEN `clm draft a` runs
- THEN the prompt file doesn't contain `The human's answers`

#### Scenario: whitespace-only answers
- GIVEN untaken item `a` and a file `answers.md` holding only blank lines
- WHEN `clm draft --answers answers.md a` runs
- THEN it exits 0 and the prompt file doesn't contain `The human's answers`

#### Needs
- model/parse-backlog
