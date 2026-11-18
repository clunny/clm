---
component: clm
---

## MODIFIED Requirements

### Requirement: glossary-covers-terms <!-- base: 0cecca4939fb45df -->
The guide's glossary, `docs/guide/glossary.md`, SHALL hold as a code span (backticks around exactly that text, anywhere on the page) every top-level `clm` command but `help`, written `clm <command>`; every requirement state, component status, change status and backlog kind, written as the word itself (such as `unbuilt`, `active`, `applied` and `bug`); and every key a live change's `change.md` frontmatter may set, written as the key (such as `human_burdens`). A test in `crates/clm/tests/` SHALL take the commands from the `Commands:` section of the help `clm --help` prints, the requirement states from `check.coverage/state-names` and the statuses, kinds and keys from `model/term-lists`, and fail naming each one the glossary doesn't hold, so a term added to any of them without a glossary entry fails the land's checks. The check is a function over the glossary's text and the term lists, so the test feeds it fixtures.

#### Scenario: every term defined
- GIVEN the repository
- WHEN the test reads `docs/guide/glossary.md`
- THEN every command, state, status, kind and key appears in it as a code span

#### Scenario: a command without an entry
- GIVEN a glossary that holds every term but `clm rebase`
- WHEN the test's check runs on it
- THEN it reports `clm rebase` as missing from the glossary

#### Needs
- model/term-lists
- check.coverage/state-names

#### Serves
- architecture/plain-terms
- capability/learn-clm
