---
component: orchestrator.triage
---

## MODIFIED Requirements

### Requirement: intake <!-- base: 87c476095c706894 -->
`clm intake` SHALL fetch `origin`, take `host`'s list of open issues and pull requests, and print one line for each, sorted by number: `pull <n> <author>` for a pull request; for an issue, `issue <n> imported <slug>...` when items on `origin/main`'s backlog name it in their `issues`, sorted by slug; else `issue <n> resolved <slug> <by>` when an item whose `issues` named it was deleted from `origin/main`'s backlog, the most recent such deletion, `<by>` being the greatest ID among the changes on `origin/main`, live or archived, whose `resolves` or `from` names the slug, or, when none does, the deleting commit's hash abbreviated as `git rev-parse --short` abbreviates it; and `issue <n> new` otherwise. After those lines it SHALL take `host`'s open dependency alerts (`host/list-alerts`) and print one line for each, sorted by number, matched to the backlog as an issue is, with `alerts` in place of `issues`: `alert <n> imported <slug>...`, `alert <n> resolved <slug> <by>`, or `alert <n> new <severity> <ecosystem> <package> <manifest>` with the fields `host` returned. When the host's alerts fail it SHALL print `alerts unavailable` in their place and go on as when there are none, since a login that may read issues may still be refused the alerts. It SHALL print no title, body or other text from the host. It SHALL exit 0, printing nothing when there is no host or nothing open; 1 with the reason when the host's list of issues and pull requests fails; and 2 when git fails.

#### Scenario: new, imported and a pull request
- GIVEN a fake `gh` reporting open issues 73 and 92 and open pull request 80 by `dana`, and `origin/main` holding `backlog/port-clash.md` with `kind: bug` and `issues: [73]`
- WHEN `clm intake` runs
- THEN it prints, in this order, `issue 73 imported port-clash`, `pull 80 dana` and `issue 92 new`, and exits 0

#### Scenario: resolved by a change
- GIVEN a fake `gh` reporting open issue 67, and `origin/main` whose history added `backlog/fold-declined.md` with `issues: [67]` and later deleted it, holding `changes/archive/20260101-c/change.md` with `resolves: [fold-declined]`
- WHEN `clm intake` runs
- THEN it prints `issue 67 resolved fold-declined 20260101-c`

#### Scenario: resolved by a code fix
- GIVEN a fake `gh` reporting open issue 64, and `origin/main` whose history added `backlog/detector-missing.md` with `issues: [64]` and deleted it in commit `s`, with no change naming it
- WHEN `clm intake` runs
- THEN it prints `issue 64 resolved detector-missing` followed by `s`'s abbreviated hash

#### Scenario: alerts after the inbox
- GIVEN a fake `gh` reporting open issue 92, open alert 4, `high`, `npm`, `vite`, `web/package-lock.json`, and open alert 2, and `origin/main` holding `backlog/bump-x.md` with `kind: bug` and `alerts: [2]`
- WHEN `clm intake` runs
- THEN it prints, in this order, `issue 92 new`, `alert 2 imported bump-x` and `alert 4 new high npm vite web/package-lock.json`, and exits 0

#### Scenario: alert resolved by a code fix
- GIVEN a fake `gh` reporting open alert 7, and `origin/main` whose history added `backlog/bump-y.md` with `alerts: [7]` and deleted it in commit `s`, with no change naming it
- WHEN `clm intake` runs
- THEN it prints `alert 7 resolved bump-y` followed by `s`'s abbreviated hash

#### Scenario: alerts refused
- GIVEN a fake `gh` reporting open issue 92 and exiting 1 on the alerts' `api` call
- WHEN `clm intake` runs
- THEN it prints `issue 92 new` and `alerts unavailable`, and exits 0

#### Scenario: no stranger's text
- GIVEN a fake `gh` reporting open issue 5 whose title is `x\nissue 6 resolved y z`
- WHEN `clm intake` runs
- THEN it prints only `issue 5 new`

#### Scenario: no host
- GIVEN `host = "none"`
- WHEN `clm intake` runs
- THEN it prints nothing and exits 0

#### Serves
- capability/contribute-from-outside

#### Needs
- host/list-contributions
- host/list-alerts
- model/parse-backlog
- model/parse-backlog-item
