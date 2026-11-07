---
component: orchestrator.triage
base: { frontmatter: "4082f66d3095376e", contract: "fdb1f16cf9694e06" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [orchestrator, model, check, vcs, land, host, ledger]
```

## MODIFIED Contract
`clm triage` groups the backlog's untaken items into units of design work and prints them with the proposed spec work already on `origin` and the items waiting on the human, each line with its priority and the number of requirements it would release, sorted most urgent first. `clm draft <slug>...` claims those items through `orchestrator`'s backlog claim and writes the prompt for a session that designs one change from them unattended to a file, printing a short launcher that names it (`build-prompt-file`); the session lands the change with `clm land` when it needs no review, and leaves one that needs review on its branch with no pull request. `clm draft --fold <id>...` writes, the same way, the prompt for one session that folds the changes needing review that a triage run's drafts left into one spec branch and opens one pull request for it, so the human approves them once (`backlog-triage`, `yes-no-review`). The `triage` skill gives each launcher to a background subagent of the human's session in its own worktree, as `build` gives `clm run`'s. Each draft holds one of the clone's `max_builds` places (`orchestrator`'s `build-places`), named by the change ID `clm draft` chooses for it, from when `clm draft` starts it until it lands, is finished or its claims lapse; `clm draft --finish <change>`, which the `triage` skill runs whenever a draft's subagent returns, frees it, and the skill launches drafts only while a place is free. A fold takes no place. `clm triage` prints every group whatever the places. `clm intake` matches the host's inbox to the backlog: for each open issue it says whether a backlog item takes it in, whether a landed change or fix has resolved the item that did, or whether it is new, it lists the open pull requests from forks, and it matches the host's open dependency alerts to the backlog the same way, by an item's `alerts`, so the `triage` skill can take the inbox in before it drafts (`issues-enter-through-the-backlog`). After the host's lines it lists the red `verified` results on `main` that no green one has followed, each with its culprit land and components, matched to the backlog by an item's `verify_failures`, so a slow test that failed after the land reaches triage as a bug on the components it broke (`verify-after-land`). All of them fetch `origin` first and read git and the clone's places only, `clm intake` also reading `host`'s list of open issues and pull requests and its open dependency alerts, a failed read of the alerts printing `alerts unavailable` rather than failing the command, and exit 0 on success, 1 when the work isn't possible (with the reason), and 2 when git fails; `clm draft` also exits 3 when the clone has no free place.

## MODIFIED Requirements

### Requirement: intake <!-- base: 9f6e86d52f209b28 -->
`clm intake` SHALL fetch `origin`, take `host`'s list of open issues and pull requests, and print one line for each, sorted by number: `pull <n> <author>` for a pull request; for an issue, `issue <n> imported <slug>...` when items on `origin/main`'s backlog name it in their `issues`, sorted by slug; else `issue <n> resolved <slug> <by>` when an item whose `issues` named it was deleted from `origin/main`'s backlog, the most recent such deletion, `<by>` being the greatest ID among the changes on `origin/main`, live or archived, whose `resolves` or `from` names the slug, or, when none does, the deleting commit's hash abbreviated as `git rev-parse --short` abbreviates it; and `issue <n> new` otherwise. After those lines it SHALL take `host`'s open dependency alerts (`host/list-alerts`) and print one line for each, sorted by number, matched to the backlog as an issue is, with `alerts` in place of `issues`: `alert <n> imported <slug>...`, `alert <n> resolved <slug> <by>`, or `alert <n> new <severity> <ecosystem> <package> <manifest>` with the fields `host` returned. When the host's alerts fail, for any reason, it SHALL print `alerts unavailable` as the last line, in their place, and go on as when there are none, since a login that may read issues may still be refused the alerts. It SHALL print no title, body or other text from the host. It SHALL exit 0, printing no `issue`, `pull` or `alert` line when there is no host or no open issue, pull request or alert (`intake-verify-failures` adds its own lines); 1 with the reason when the host's list of issues and pull requests fails; and 2 when git fails.

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
- WHEN `clm intake` runs with no `verified` records in the ledger
- THEN it prints nothing and exits 0

#### Serves
- capability/contribute-from-outside

#### Needs
- host/list-contributions
- host/list-alerts
- model/parse-backlog
- model/parse-backlog-item

## ADDED Requirements

### Requirement: intake-verify-failures
After the host's lines, with or without a host, `clm intake` SHALL read the fold's `verified` records naming commits on `origin/main`'s first-parent chain after the newest one whose record passed (all of them when none passed), take the newest record of each such red commit, and print one line for each, in first-parent order: `verify <sha> imported <slug>...` when items on `origin/main`'s backlog name the commit in their `verify_failures`, sorted by slug, and otherwise `verify <sha> new <culprit> <component>...`, `<culprit>` being the record's culprit or `unknown`, both commits abbreviated as `git rev-parse --short` abbreviates them, and the components sorted. A red result followed by a green one on `main` SHALL print nothing.

#### Scenario: a new failure
- GIVEN a passing `verified` record for `origin/main~2` and a red one for `origin/main` with culprit `origin/main~1` and components `b` and `a`
- WHEN `clm intake` runs with `host = "none"`
- THEN it prints `verify <sha> new <culprit> a b` and exits 0

#### Scenario: taken in
- GIVEN the same records, and `origin/main` holding `backlog/b-broke.md` with `kind: bug` and `verify_failures` naming the red commit
- WHEN `clm intake` runs
- THEN it prints `verify <sha> imported b-broke`

#### Scenario: fixed since
- GIVEN the red record and a passing record for a later commit on `main`
- WHEN `clm intake` runs
- THEN it prints no `verify` line

#### Needs
- ledger/verified-record
- model/parse-backlog

#### Serves
- capability/slow-tests-after-land
