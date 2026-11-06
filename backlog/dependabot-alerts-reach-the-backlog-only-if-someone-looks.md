---
title: "Dependabot alerts reach the backlog only if someone looks"
kind: bug
components: [ci]
---

The decision github-repo-settings turns Dependabot's pull requests off and says an alert goes through the backlog, but no CLM step does that. No command, check or workflow reads a repository's alerts or files an item for them, and nothing in CI or `clm check` runs a dependency audit, so open alerts stayed out of the backlog until a human asked about them.

Expected: the route the decision names happens without anyone having to remember it, by a step that files alerts as backlog items or a check that reports open ones.
