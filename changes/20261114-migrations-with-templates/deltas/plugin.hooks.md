---
component: plugin.hooks
base: { frontmatter: "b3443a2a6f08dae3" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [config, model, sync, view, view.repo, host, vcs, upgrade, init, agent-harness]
```

## MODIFIED Requirements

### Requirement: session-upgrade-reminder <!-- base: 5a5a1830ba3b3228 -->
When `clm hook session-start` prints the status of a CLM repository with a pending migration, it SHALL print after it one line saying how many `required` and how many `advised` migrations are pending, whether commands are blocked (when a `required` one the clone doesn't skip is pending), and to run `/clm:upgrade`. A skipped `required` migration SHALL be counted as skipped, not as blocking. It SHALL read what is pending with `upgrade`'s `pending_in` over `clm-init`'s `migrations()` and `supersedes()` (`init/all-migrations`), so a migration `init` holds is counted. When that returns `TooNew`, the line SHALL say the project needs a newer `clm`. It SHALL print no such line when none is pending, and SHALL still exit 0. It SHALL print the line even when `clm status` fails.

#### Scenario: advised drift
- GIVEN a CLM repository with one pending `advised` migration
- WHEN `clm hook session-start` runs
- THEN its output holds a line naming `1 advised` and `/clm:upgrade`, not `blocked`, and it exits 0

#### Scenario: required pending
- GIVEN a CLM repository with one pending `required` migration
- WHEN `clm hook session-start` runs
- THEN the line names `1 required`, `blocked` and `/clm:upgrade`, and it exits 0

#### Scenario: required but skipped
- GIVEN the same repository, with the migration in the clone's `skip_migrations`
- WHEN `clm hook session-start` runs
- THEN the line names `skipped` and not `blocked`

#### Scenario: up to date
- GIVEN a repository `clm init` has just set up
- WHEN `clm hook session-start` runs
- THEN no line names `/clm:upgrade`

#### Needs
- init/all-migrations
- upgrade/pending-migrations
- upgrade/require-current
- init/init-stamps-upgraded

#### Serves
- capability/upgrade-clm
