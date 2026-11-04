---
component: plugin.hooks
---

## MODIFIED Requirements

### Requirement: defect-rule <!-- base: 6840960fde4217b7 -->
When `clm hook session-start` prints the CLM state and the repository's `defects` setting isn't `off`, it SHALL add a paragraph that names `clm defect`, says what counts as a CLM defect and what doesn't, says a report describes only the process and never the project's files, code, specs or output, and says a session that can't ask the human lists defects under `## CLM defects` in its PR body. When the setting is `backlog` the paragraph SHALL also say that reports go to `backlog/` as `kind: bug` items, and that a small defect needing no spec change is fixed in the moment rather than left filed: folded into the current branch, unreported, when it is an `impl/*` branch and the fix lies in its components (`fold-small-defer-large`), and otherwise reported with `clm defect`, naming with `--component` the component the fix lies in, and, once the current branch has landed and brought the item to `main`, fixed before the session ends on its own branch with `clm claim <component> --fix backlog/<slug>` (`code-fix-resolves-bug`); that only a defect that is a large amount of work or needs a spec change is left in the backlog; and that an unattended session folds what fits and reports the rest. With `ask` it SHALL say none of that, and SHALL say to send the report as the body of a `<<'EOF'` heredoc on the `clm defect` line, the one form the guard shows the human before the report leaves the project (`defect-ask-shows-report`). When the setting is `off` it SHALL add nothing. When the settings can't be loaded it SHALL print their error instead, and still exit 0.


#### Scenario: asking
- GIVEN a CLM repository with no settings
- WHEN `clm hook session-start` runs
- THEN its output names `clm defect` and `## CLM defects` after the `claims` section, names `<<'EOF'`, and doesn't name `--fix`

#### Scenario: backlog
- GIVEN a CLM repository whose `clm.toml` sets `defects = "backlog"`
- WHEN `clm hook session-start` runs
- THEN its output names `clm defect`, `backlog/`, `fold-small-defer-large`, `--component`, `clm claim <component> --fix backlog/<slug>`, `landed` and `spec change`

#### Scenario: off
- GIVEN a CLM repository whose `clm.toml` sets `defects = "off"`
- WHEN `clm hook session-start` runs
- THEN its output doesn't mention `clm defect`

#### Scenario: bad settings
- GIVEN a CLM repository whose `clm.toml` sets `defects = "always"`
- WHEN `clm hook session-start` runs
- THEN its output names `clm.toml` and `defects`, and it exits 0

#### Serves
- capability/report-defects

