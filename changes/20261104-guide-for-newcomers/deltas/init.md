---
component: init
---

## ADDED Requirements

### Requirement: guide-entry-pages
The guide's entry pages SHALL stay written for a newcomer (`guide-entry-pages-for-newcomers`). `docs/guide/quickstart.md` SHALL hold `/plugin marketplace add clunny/clm` and `/plugin install clm@clm`, SHALL name `/clm:init`, `/clm:build`, `/clm:propose`, `/clm:triage` and `/clm:health`, and SHALL link `using-clm.md` and `how-it-works.md`. `docs/guide/using-clm.md` SHALL name `/clm:propose`, `/clm:build`, `/clm:triage` and `/clm:health`, and SHALL link `how-it-works.md`. Neither page SHALL hold, outside fenced code blocks, a code span whose text, as written, starts with `clm ` (a command of the `clm` CLI); a skill such as `/clm:build`, and `clm` alone, are not one. A test SHALL read both pages from `docs/guide/` and fail naming the page and what it lacks or holds.

#### Scenario: this repository's guide
- GIVEN the pages `docs/guide/quickstart.md` and `docs/guide/using-clm.md` of this repository
- WHEN the tests run
- THEN the entry-pages test passes

#### Scenario: a command on an entry page
- GIVEN a `using-clm` page that otherwise meets the rule and holds `` run `clm land <branch>` ``
- WHEN the rule runs on it
- THEN it reports `using-clm: names clm land <branch>`

#### Scenario: a workflow missing
- GIVEN a `quickstart` page that otherwise meets the rule but never names `/clm:triage`
- WHEN the rule runs on it
- THEN it reports `quickstart: lacks /clm:triage`

#### Scenario: a command in a fenced block
- GIVEN a `quickstart` page that meets the rule and holds a fenced block with `clm status` in it
- WHEN the rule runs on it
- THEN it reports nothing
