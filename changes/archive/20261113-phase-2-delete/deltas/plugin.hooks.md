---
component: plugin.hooks
---

## MODIFIED Requirements

### Requirement: defect-rule <!-- base: bf9aa4249c2274f7 -->
When `clm hook session-start` prints the CLM state and the repository's `defects` setting isn't `off`, it SHALL add a paragraph that names `clm defect`, after the claims. When the setting is `backlog` the paragraph SHALL also name the backlog route: `backlog/` and `clm claim <component> --fix backlog/<slug>`. With `ask` it SHALL name none of that route, and SHALL name the `<<'EOF'` heredoc form of the `clm defect` line. When the setting is `off` it SHALL add nothing. When the settings can't be loaded it SHALL print their error instead, and still exit 0.

#### Scenario: asking
- GIVEN a CLM repository with no settings
- WHEN `clm hook session-start` runs
- THEN its output names `clm defect` after the `claims` section, names `<<'EOF'`, and doesn't name `--fix`

#### Scenario: backlog
- GIVEN a CLM repository whose `clm.toml` sets `defects = "backlog"`
- WHEN `clm hook session-start` runs
- THEN its output names `clm defect`, `backlog/` and `clm claim <component> --fix backlog/<slug>`

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
