---
component: orchestrator.triage
base: { frontmatter: "89939996df3395d7", contract: "70efd400cfda7c65" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [orchestrator, model, check, vcs, land, host, ledger, verify]
```

## MODIFIED Contract
`clm triage` groups the backlog's untaken items into units of design work and prints them with the proposed spec work already on `origin` and the items waiting on the human, each line with its priority and the number of requirements it would release, sorted most urgent first. `clm draft <slug>...` claims those items through `orchestrator`'s backlog claim and writes the prompt for a session that designs one change from them unattended to a file, printing a short launcher that names it (`build-prompt-file`); the session lands the change with `clm land` when it needs no review, and leaves one that needs review on its branch with no pull request. `clm draft --fold <id>...` writes, the same way, the prompt for one session that folds the changes needing review that a triage run's drafts left into one spec branch and opens one pull request for it, so the human approves them once (`backlog-triage`, `yes-no-review`). The `triage` skill gives each launcher to a background subagent of the human's session in its own worktree, as `build` gives `clm run`'s. Each draft holds one of the clone's `max_builds` places (`orchestrator`'s `build-places`), named by the change ID `clm draft` chooses for it, from when `clm draft` starts it until it lands, is finished or its claims lapse; `clm draft --finish <change>`, which the `triage` skill runs whenever a draft's subagent returns, frees it, and the skill launches drafts only while a place is free. A fold takes no place. `clm triage` prints every group whatever the places. `clm intake` matches the host's inbox to the backlog: for each open issue it says whether a backlog item takes it in, whether a landed change or fix has resolved the item that did, or whether it is new, it lists the open pull requests from forks, and it matches the host's open dependency alerts to the backlog the same way, by an item's `alerts`, so the `triage` skill can take the inbox in before it drafts (`issues-enter-through-the-backlog`). After the host's lines it lists the red `verified` results on `main` that no green one has followed, each with its culprit land and components, matched to the backlog by an item's `verify_failures`, and, after each such result no item has taken in, the path of the log its run kept when this clone holds one (`verify`'s `kept_log`), so a slow test that failed after the land reaches triage as a bug on the components it broke, with the failure's own output for the human to judge it by (`verify-after-land`). All of them fetch `origin` first and read git and the clone's places only, `clm intake` also reading the clone's kept verify logs and `host`'s list of open issues and pull requests and its open dependency alerts, a failed read of the alerts printing `alerts unavailable` rather than failing the command, and exit 0 on success, 1 when the work isn't possible (with the reason), and 2 when git fails; `clm draft` also exits 3 when the clone has no free place.

## MODIFIED Requirements

### Requirement: intake-verify-failures <!-- base: 540c3ede2d4c9870 -->
After the host's lines, with or without a host, `clm intake` SHALL take the fold's `unresolved_reds` over `origin/main`'s first-parent chain (`ledger/verified-record`) and print one line for each, in first-parent order: `verify <sha> imported <slug>...` when items on `origin/main`'s backlog name the commit in their `verify_failures`, sorted by slug, and otherwise `verify <sha> new <culprit> <component>...`, `<culprit>` being the record's culprit or `unknown`, both commits abbreviated as `git rev-parse --short` abbreviates them, and the components sorted. After each `verify <sha> new` line it SHALL print `verify <sha> log <path>`, `<sha>` as on the line before it, when `verify/kept-log` returns a path for the red commit, `<path>` being that path, and nothing more when it returns none; an `imported` line SHALL get no `log` line. A red result followed by a green one on `main` SHALL print nothing.

#### Scenario: a new failure
- GIVEN a passing `verified` record for `origin/main~2` and a red one for `origin/main` with culprit `origin/main~1` and components `b` and `a`, and no log kept for it
- WHEN `clm intake` runs with `host = "none"`
- THEN it prints `verify <sha> new <culprit> a b`, no `verify <sha> log` line, and exits 0

#### Scenario: a kept log
- GIVEN the same records, and `.clm/verify-logs/<sha>.log` in the clone's main worktree, `<sha>` the red commit abbreviated
- WHEN `clm intake` runs with `host = "none"` in a linked worktree of the clone
- THEN it prints `verify <sha> new <culprit> a b` and, on the next line, `verify <sha> log <path>` with that file's absolute path

#### Scenario: taken in
- GIVEN the same records and kept log, and `origin/main` holding `backlog/b-broke.md` with `kind: bug` and `verify_failures` naming the red commit
- WHEN `clm intake` runs
- THEN it prints `verify <sha> imported b-broke` and no `verify <sha> log` line

#### Scenario: fixed since
- GIVEN the red record and a passing record for a later commit on `main`
- WHEN `clm intake` runs
- THEN it prints no `verify` line

#### Needs
- ledger/verified-record
- model/parse-backlog
- verify/kept-log

#### Serves
- capability/slow-tests-after-land
