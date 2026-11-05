---
component: clm
---

## MODIFIED Requirements

### Requirement: session-commands <!-- base: 0f65d198ae9735b2 -->
`.claude/settings.json`'s `permissions.allow` SHALL allow, as Bash commands, only `cargo`, `npm ci`, `npm install --ignore-scripts`, `npm run`, `npm test` and `npx playwright test`, each with any arguments, `.github/scripts/ci-check.sh` both bare and with any arguments, as `Bash(.github/scripts/ci-check.sh)` and `Bash(.github/scripts/ci-check.sh *)`, and the rules for unattended `clm` commands that `clm init` seeds (`init/init-seeds-permissions`), each of them. It SHALL NOT allow `npm install` without `--ignore-scripts`, any other `npx` command, `rustup`, `git` or `clm approve`.

#### Scenario: allowed commands
- GIVEN the repository
- WHEN a test reads `.claude/settings.json`
- THEN every `permissions.allow` entry is one of those commands or one of `clm init`'s unattended rules, and each of them is allowed

#### Scenario: bare npx refused
- GIVEN a settings file whose `permissions.allow` holds `Bash(npx *)`
- WHEN the test's check runs on it
- THEN it reports that entry

#### Scenario: a rule for approve refused
- GIVEN a settings file whose `permissions.allow` holds `Bash(clm approve *)`
- WHEN the test's check runs on it
- THEN it reports that entry

#### Needs
- init/init-seeds-permissions
