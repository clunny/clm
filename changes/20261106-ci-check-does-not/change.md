---
status: applied
resolves: [ci-check-does-not-install-web-site-deps-for-capabilities-web, dependabot-alerts-reach-the-backlog-only-if-someone-looks]
---

## Intent

Bring the host's open dependency alerts into the backlog through the step every triage already runs. `clm intake` lists Dependabot's open alerts after the issues and pull requests, matched to the backlog by a new backlog key, `alerts`, as issues are by `issues`, and `/clm:triage` offers each new alert to the human as a `kind: bug` item. So the route the decision `github-repo-settings` names, an alert going through the backlog, happens without anyone remembering to look.

`ci-check-does-not-install-web-site-deps-for-capabilities-web` is already fixed: commit `ae663c3` (`Resolves: backlog/web-build-still-builds-the-site`), landed ten minutes after the item was filed, made `capabilities/web/global-setup.ts` run `npm ci --ignore-scripts` in `web/site/` itself before it builds the site, so a selection holding only `capabilities/web/` files gets the site's dependencies. The change deletes the item.

## Problem

`github-repo-settings` turns Dependabot's security pull requests off, since they would be code branches no build wrote (`ai-writes-code`), and says an alert goes through the backlog instead. Nothing does that: no command, check or workflow reads the alerts, so open alerts stayed out of the backlog until the human asked about them.

## Approaches

- **A. `clm intake` lists the alerts; `/clm:triage` takes them in (chosen).** `/clm:triage` already runs `clm intake` first, and `intake` already matches the host's issues to backlog items deterministically and reports them `new`, `imported` or `resolved` (`issues-enter-through-the-backlog`). Alerts reuse that whole path: one host call, one backlog key, one more kind of line. No new command, no network in any check or land.
- **B. A dependency audit in `ci-check.sh` or a detector in `clm check`.** Reads an advisory database over the network, so a land of unrelated code fails the day an advisory appears, and `clm check` reads git alone (`git-is-truth`). The host already runs the audit. Rejected.
- **C. A scheduled workflow opening an issue per alert.** A workflow's `GITHUB_TOKEN` can't read Dependabot alerts, so it needs a personal token as a secret: a human step and a standing credential for what one `gh api` call from the maintainer's login does. Rejected.
- **D. Accept: GitHub emails the alerts.** That is the "someone looks" the item reports as failing. Rejected.

## Design

The decision `dependency-alerts-through-intake` records the choice.

- `host` (`list-alerts`): `alerts` returns each open alert's number, severity, ecosystem, package and manifest path, from `gh api --paginate repos/{owner}/{repo}/dependabot/alerts?state=open`. No advisory text. A field with whitespace or a control character has those replaced by `?`, so every printed field is one word. `git` and `none` return nothing.
- `model` (`parse-backlog`): a backlog item may carry `alerts`, a list of positive whole numbers, as it carries `issues`. A separate key, since GitHub numbers alerts and issues apart.
- `orchestrator.triage` (`intake`): after the issue and pull lines, one line per open alert, sorted by number: `alert <n> imported <slug>...`, `alert <n> resolved <slug> <by>`, or `alert <n> new <severity> <ecosystem> <package> <manifest>`, matched with the same lookup as issues. When the alerts call fails (alerts off, a login without the rights), `alerts unavailable` and exit 0: the issues still come in.
- `plugin` (`triage-contributions`, `triage-menu`): the alert lines count under *Contributions*. New alerts are shown together, with no verifier (they carry no stranger's text), and the human picks which to take in and at what priority; each becomes a `kind: bug` item with `alerts: [<n>]`, the components whose `paths` hold its manifest, on the same `intake/<YYYYMMDD-HHMM>` branch. A `resolved` alert is shown with what resolved it: the host closes an alert only once the updated dependency is on `main`, so it is offered again once that change is archived.
- Nothing writes to the host: the alert closes itself when the dependency is updated.

Edge cases the change already knows:
- A login that may read issues but not alerts: `alerts unavailable`, not a failed intake.
- An alert whose manifest lies in no component's `paths` (the root `Cargo.lock`): the item gets the components the human names, or none.

## Open questions

None yet.
