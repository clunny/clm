---
component: ci
---

## ADDED Requirements

### Requirement: security-policy
`.github/SECURITY.md` SHALL tell a person to report a vulnerability privately through the repository's security advisories at `https://github.com/clunny/clm/security/advisories/new`, never in a public issue, SHALL say that the latest release is the supported version, and SHALL name as in scope the `clm` binary, the plugin and the web app.

#### Scenario: a reporter reads it
- GIVEN this repository's `.github/SECURITY.md`
- WHEN a test reads it
- THEN it holds that URL and the words `latest release`, `clm`, `plugin` and `web app`

#### Serves
- capability/contribute-from-outside

### Requirement: code-of-conduct
`.github/CODE_OF_CONDUCT.md` SHALL be the Contributor Covenant version 2.1, taken from `https://www.contributor-covenant.org/version/2/1/code_of_conduct/code_of_conduct.md`, unchanged but for its contact, which SHALL be `conduct@clm-app.dev`.

#### Scenario: a contributor reads it
- GIVEN this repository's `.github/CODE_OF_CONDUCT.md`
- WHEN a test reads it
- THEN it holds `Contributor Covenant`, `version 2.1` and `conduct@clm-app.dev`, and doesn't hold `[INSERT`

#### Serves
- capability/contribute-from-outside

### Requirement: issue-forms
`.github/ISSUE_TEMPLATE/` SHALL hold the issue forms `bug.yml` and `idea.yml`, the second saying that a suggestion may be one line and that a maintainer's `accepted` label is the yes to take it further, and `config.yml`, which SHALL set `blank_issues_enabled: false` and link to the security advisories page and to `CONTRIBUTING.md`. `clm defect`'s `gh issue create` isn't a web form, so these SHALL leave it working.

#### Scenario: the forms
- GIVEN this repository's `.github/ISSUE_TEMPLATE/`
- WHEN a test parses its three files as YAML
- THEN each parses, `bug.yml` and `idea.yml` each hold `name`, `description` and `body`, `idea.yml` holds `accepted`, and `config.yml` sets `blank_issues_enabled` to false and holds `security/advisories/new` and `CONTRIBUTING.md`

#### Serves
- capability/contribute-from-outside

### Requirement: pull-request-template
`.github/PULL_REQUEST_TEMPLATE.md` SHALL tell the author that a pull request changing any file outside the spec paths, which it SHALL list, is closed without review, SHALL point at `CONTRIBUTING.md`, and SHALL ask for the issue the work comes from.

#### Scenario: an author reads it
- GIVEN this repository's `.github/PULL_REQUEST_TEMPLATE.md`
- WHEN a test reads it
- THEN it names `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `CONTRIBUTING.md` and `closed`

#### Serves
- capability/contribute-from-outside
