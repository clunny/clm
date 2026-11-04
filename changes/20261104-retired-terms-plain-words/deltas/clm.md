---
component: clm
---

## MODIFIED Requirements

### Requirement: retired-terms <!-- base: 7ddad81bec2dc2ef -->
A test in `crates/clm/tests/` SHALL hold a list of retired terms, each the old name a completed rename replaced (`plain-terms-renames`) in a form ordinary English never takes: a command after `clm` and a space, a command's outcome line, a flag, a key, a `kind:` written in backticks, a coined term of several words, or a coined word that isn't an English word. A word of plain English SHALL NOT join the list on its own, whatever it once named, since people keep using it in its everyday sense; its retired name is listed in the forms above instead. The test SHALL fail, naming the file, line and term, when one appears as a whole word in the text CLM ships or works from: every file, recursively, under `docs/guide/`, `plugin/`, `specs/` and `web/src/`; `docs/spec-format.md`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `README.md`; and the help `clm --help` and `clm <command> --help` print for every command and subcommand. Matching ignores case on both sides; a whole word is one with no letter, digit or `_` directly before or after it, so a hyphen is a boundary (`frob` matches in `frob-scoped` and not in `frobnicate`), and a term of several words matches with any run of spaces between them. A line holding the text `clm:remove-by` anywhere SHALL be exempt, so an alias may keep an old name until its contract change removes it. History and proposals SHALL be exempt by not being read: `changes/`, archived or live, `docs/decisions/`, `backlog/` and the ledger keep the names they were written with, since a change that describes a rename has to name the old word; what a live change's deltas say reaches `specs/` when it is applied, and is read there. The change that completes a rename adds the old names it retires, in those forms. The check is a function over the list and each file's path and text, so the scenarios below run on fixtures.

#### Scenario: a retired term in a skill
- GIVEN the list holds `grommet stub` and a skill's `SKILL.md` line reads `make it a grommet stub`
- WHEN the test's check runs on it
- THEN it reports that file, its line number and `grommet stub`

#### Scenario: an alias kept for expand-contract
- GIVEN the list holds `clm frob` and a line reads `` `clm frob` still runs `clm twiddle` // clm:remove-by 20261201-x ``
- WHEN the test's check runs on it
- THEN it reports nothing for that line

#### Scenario: history is left alone
- GIVEN the list holds `frobbed`, and a fixture repository whose archived change, live change's `change.md` and decision use it
- WHEN the test runs on that fixture
- THEN none of them is reported

#### Scenario: whole words only
- GIVEN the list holds `frob`
- WHEN a guide page reads `frobnicate`
- THEN nothing is reported

#### Scenario: no plain English word on the list
- GIVEN the list check, a function over a list of terms, run once on a fixture list holding `clm frob`, `--frob` and the bare word `frob`, and once on the test's own list
- WHEN it judges each term
- THEN on the fixture it fails naming `frob` alone, and on the test's own list it passes: every term holds a space, a hyphen, an underscore or a backtick, or is the old name of the status `unbuilt`

#### Serves
- architecture/plain-terms

### Requirement: node-names-retired <!-- base: 0f17de3853fbd398 -->
The old names of the flags and keys `component-flags`, `model/component-keys`, `config/component-size-setting` and `init/survey-components` renamed SHALL no longer work: `clm defect` and `clm proposals` given their old flag, two hyphens and the word for one component the rename retired, and `clm survey` given its old flag, two hyphens and the plural, SHALL fail as clap fails for an unknown argument. `clm/retired-terms`'s list SHALL hold both old flags and the setting's old name, so none comes back (`plain-terms`), and the old name `plain-terms-renames` replaced with placeholder component, also in its plural; this text names them all only by description, so the list doesn't flag it. The bare words for a component and for components stay off the list (`component-words`), and so does the old word for ancestors, a plain English word (`clm/retired-terms`). No `clm:remove-by 20261102-rename-node-to-component-contract` marker SHALL be left.

#### Scenario: an old flag is refused
- GIVEN a CLM repository with `defects = "backlog"` and component `gate`
- WHEN `clm defect --title "x"` runs with the old flag naming `gate`
- THEN it exits non-zero naming the unknown argument, and writes nothing

#### Scenario: the old names retired
- GIVEN the repository
- WHEN the retired-terms test runs
- THEN its list holds the two old flags, the setting's old name and the old name of placeholder component in its two forms, holds no bare word for ancestors, and it finds none of them

#### Needs
- plugin.hooks/defect-backlog
- check.health/workaround-command
- check.health/reconsider-score
- sync/file-proposal-items
- sync/file-reconsider-item
- rename/component-key
- gate/item-components
- model/component-keys
- config/component-size-setting
- init/survey-components

#### Serves
- architecture/plain-terms
