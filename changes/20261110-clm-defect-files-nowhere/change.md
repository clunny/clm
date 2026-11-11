---
status: applied
resolves:
  - clm-defect-files-nowhere-when-the-project-s-host-is-azure-devops
---

## Intent

With `defects = "ask"`, `clm defect` files its report on CLM's own GitHub repository through `gh` whatever the project's `host` setting, so a project on Azure DevOps, a bare git server or no host files reports as a GitHub project does. A report it can't file (no `gh`, not signed in, no access) is kept in `clm/defects/<slug>.md` under the clone's git common directory, which every worktree shares and outlives, and `clm defect` names the file, so nothing is left only in a session's output. Decision `defects-file-on-github`.

## Problem

`clm defect` filed through the adapter the project's `host` setting picks. On `azure`, `git` and `none` that adapter's `issue` prints the report and says there is nowhere to file it, though the report's place is `clunny/clm` on GitHub whatever the project's host (`defect-reports`; `host`'s Contract already said as much). A session on an Azure DevOps project reported ten defects this way and had to keep them in a file by hand, and on any host a report `gh` couldn't file was only printed.

## Approaches

Where a report is filed:
- **Always through the `github` adapter (chosen).** `clm_host::adapter(Host::Github).issue(...)` runs `gh issue create --repo clunny/clm`, which needs only `gh` signed in, not the project's remote. No new host code (`reuse-first`); the guard already asks on every host when `defects = "ask"`, so nothing leaves the project unapproved.
- **Give `azure`, `git` and `none` an `issue` that runs `gh`.** Puts GitHub calls in three adapters that otherwise have no GitHub, and keeps `clm defect` tied to a setting that has nothing to do with where its reports go. Rejected.

Where an unfiled report is kept:
- **`clm/defects/` in the git common directory (chosen).** Shared by every worktree of the clone and outliving them, never committed, as `none-keeps-its-own-remote`'s remote lives there.
- **A backlog item in the project.** Rejected: CLM's components fail the project's `clm check`, and CLM's defects would land on the project's `main`.
- **The worktree's `.clm/`.** Rejected: a draft's or build's worktree is removed when it ends, taking the report with it.

## Outcome

- `plugin.hooks/defect-issue` (modified): files through the `github` adapter on every host; keeps an unfiled report in `clm/defects/<slug>.md` under the git common directory, naming it, and exits 1; outside git it prints the report as before.
- `plugin.hooks/defect-rule` (modified): with `ask`, the session-start paragraph says a kept report goes under `## CLM defects` with its path.
- `plugin.hooks`' Design says how; `host`'s Contract says `clm defect` uses the `github` adapter whatever the setting. `host`'s `issue` operation and its requirements are unchanged: `clm issue` on `azure`, `git` and `none` still says there is nowhere to file.
- `docs/guide/settings.md`'s `defects` section says where an unfiled report is kept.
- Decision `defects-file-on-github`, which supersedes in part `host-adapters`' "file an issue" through the `host` setting; `host-adapters` says it is refined by it.

## Constraints

- The guard's ask (`plugin.hooks.guard/defect-approval`) is unchanged: it asks whenever `defects = "ask"`, on every host.
- Edge case known: nothing lists or re-files kept reports; the human files each by hand from the path printed (`yagni` until kept reports pile up).
- The no-host scenario of `defect-issue` changes meaning: on `host = "git"` with `gh` signed in the report is now filed, so its test changes with it.
