---
component: ci
base: {}
---

## ADDED Requirements

### Requirement: early-access-form
`.github/ISSUE_TEMPLATE/early-access.yml` SHALL be an issue form named `Request early access`, adding the label `early-access`, whose description says the web app is invite-only and in early access, and that CLM works fully without it by following the quickstart at `https://clm-app.dev/guide/quickstart`. Its body SHALL have one required field asking what the requester would like to use the web app for, and SHALL ask for no email address. A maintainer grants a request by adding the login to `web`'s invited list (`web/invited-users`); the form grants nothing.

#### Scenario: the form
- GIVEN this repository's `.github/ISSUE_TEMPLATE/early-access.yml`
- WHEN a test parses it as YAML
- THEN it parses, its `name` is `Request early access`, its `labels` hold `early-access`, its `description` holds `https://clm-app.dev/guide/quickstart`, its body holds exactly one field with `validations.required` true, and no text in it holds `email`

#### Serves
- architecture/invite-only-web
