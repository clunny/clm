---
title: "A host's dependency alerts enter the backlog through clm intake, as issues do"
date: 2026-11-06
status: accepted
---

Resolves `backlog/dependabot-alerts-reach-the-backlog-only-if-someone-looks` (`changes/20261106-ci-check-does-not/`). Refines `github-repo-settings`, which turned Dependabot's pull requests off and said an alert goes through the backlog, and `untrusted-text-read-only-verifier`, whose "no host text" now names the alert fields as its one addition; extends `issues-enter-through-the-backlog`.

`clm intake` also lists the host's open dependency alerts, after the issues and pull requests, through `host` (`host-adapters`), and matches each to the backlog by a new backlog key, `alerts`, exactly as it matches an issue by `issues`: `new`, `imported` or `resolved`. `/clm:triage` already runs `clm intake` before it works the backlog, so an open alert shows in every triage without anyone remembering to look, and the human's yes turns it into a `kind: bug` item that a build fixes like any other. Once the updated dependency is on `main`, the host closes the alert itself, so nothing writes to the host.

An alert line carries only the alert's number, severity, ecosystem, package and manifest path: words from the project's own manifests and a fixed set, never the advisory's text, which isn't the project's (`untrusted-text-read-only-verifier`). So an alert needs no verifier; the human picks which to take in and at what priority.

When the login may read issues but not alerts (alerts off, or a token without the rights), `clm intake` prints `alerts unavailable` and goes on, so the issues still come in.

Rejected:
- **A dependency audit in `ci-check.sh` or a detector in `clm check` (`cargo audit`, `npm audit`).** It reads an advisory database over the network, so a land of unrelated code would fail the day an advisory is published, and `clm check` reads git alone (`git-is-truth`). The host already runs the audit; the gap was only that its findings never reached the backlog.
- **A scheduled workflow that opens an issue per alert.** A workflow's `GITHUB_TOKEN` can't read Dependabot alerts, so it would need a personal token held as a secret, a human's step and a standing credential, to reach the same backlog item one `gh api` call from the maintainer's own login reaches.
- **Turning Dependabot's pull requests back on.** They would be code branches written outside a build (`ai-writes-code`), as `github-repo-settings` says.
- **Reusing `issues` for alerts.** Alerts and issues are numbered apart on GitHub, so one number could name both.
