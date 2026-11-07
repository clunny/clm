---
component: plugin
---

## ADDED Requirements

### Requirement: init-skill-concurrent-tests
`skills/init/SKILL.md` SHALL, once `clm init` has written `scripts/clm-check.sh`, tell Claude to tell the human that CLM builds several components at once on one machine, so the project's tests, and its check script with any server the script starts, must pass while other runs of them overlap: no fixed port, no fixed path outside the checkout and the system's temporary directory, and no server or database two runs share, with a port or path that must be known in advance keyed on `CLM_SLOT`, the number CLM gives each build and its checks (`tests-run-concurrently`). For a repository with code, it SHALL tell Claude to look through the check script and the tests for fixed ports, fixed paths and shared servers, and, once the component map exists, record each it finds on the first spec branch as a `bug` backlog item whose body names `tests-run-concurrently` and whose `components` name the component whose `paths` hold the fix, as it records other bugs it finds.

#### Scenario: the shipped init skill
- GIVEN the plugin's `skills/init/SKILL.md`
- WHEN a test reads it
- THEN it contains `CLM_SLOT`, `tests-run-concurrently`, `fixed port` and `shared`

#### Serves
- architecture/tests-run-concurrently
- capability/adopt-clm
