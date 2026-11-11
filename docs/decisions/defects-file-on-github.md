---
title: "clm defect files on CLM's GitHub repository whatever the project's host, and keeps a report it can't file"
date: 2026-11-10
status: accepted
---

With `defects = "ask"`, `clm defect` files its report as an issue on `clunny/clm` (`defect-reports`). That repository is on GitHub whatever the project's own host is, so the report goes through `clm-host`'s `github` adapter (`gh issue create --repo clunny/clm`), never through the adapter the project's `host` setting picks. `gh` reaches GitHub from any clone once it is installed and signed in; the project's remote plays no part. Filing through the project's adapter had a project on Azure DevOps, a bare git server or no host print every report and say there was nowhere to file it, though the report's place was never the project's host.

When the report can't be filed (no `gh`, `gh` not signed in, no access, or no URL printed), `clm defect` keeps it: it writes the issue's title and body to `clm/defects/<slug>.md` in the clone's common git directory, the slug made as a backlog report's is, and names that file, so the human can file it later by hand. The common git directory is shared by every worktree of the clone and outlives them, as the remote a `host = "none"` clone keeps there does (`none-keeps-its-own-remote`), and git never commits it, so nothing about CLM reaches the project's history. It still exits 1: the report wasn't filed, and a session lists it under `## CLM defects` with the file's path.

Rejected:
- **A backlog item in the project.** A CLM defect isn't the project's work: its components are CLM's, which the project's `clm check` rejects, and it would land on the project's `main` (`defect-reports`: issues are inbound reports, not project state).
- **The worktree's `.clm/`.** Ignored by git, but a session's worktree is removed when its draft or build ends, taking the report with it.
- **Filing on the project's host (an Azure Boards work item).** The reports are about CLM, not the project, as `azure-through-az-cli` already says.
