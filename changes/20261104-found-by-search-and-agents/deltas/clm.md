---
component: clm
---

## MODIFIED Requirements

### Requirement: readme-front-page <!-- base: dcb1846ca4f6792a -->
`README.md` SHALL be the project's front page: what CLM is and who it is for, solo or as a team, with a link to the site at `https://clm-app.dev` before its first section, why it's useful, the commands that install it, a link to the quickstart at `docs/guide/quickstart.md`, a short account of how a change flows from intent to `main`, that CLM is young and changes quickly, how to contribute, with links to `CONTRIBUTING.md` and `.github/SECURITY.md`, and its licence, with a link to `LICENSE`. It SHALL keep its links to the guide, `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/`, `changes/`, `backlog/` and `docs/decisions/`, and to every work `docs/guide/why-clm.md` credits.

#### Scenario: a newcomer reads it
- GIVEN this repository's `README.md`
- WHEN a test reads it
- THEN it holds `/plugin marketplace add clunny/clm`, `/plugin install clm@clm`, `docs/guide/quickstart.md`, `CONTRIBUTING.md`, `.github/SECURITY.md`, `LICENSE`, `ARCHITECTURE.md`, `CAPABILITIES.md` and `arXiv:2606.27045`, and each relative link in it names a file or directory that exists

#### Scenario: site and credits
- GIVEN this repository's `README.md`
- WHEN a test reads it
- THEN a link to `https://clm-app.dev` comes before its first `##` heading, and it links to `https://kubernetes.io/docs/concepts/architecture/controller/`, `https://github.com/obra/superpowers` and `https://claude.com/blog/the-ai-native-sdlc-playbook`

#### Needs
- ci/security-policy

#### Serves
- capability/learn-clm

### Requirement: old-state-and-key-names-retired <!-- base: ab16e190ad5e562d -->
The old name of `clm workaround`, which `check.health/workaround-command` kept as a hidden subcommand, SHALL no longer run: `clm` given it SHALL fail as it fails for any unknown subcommand. `clm/retired-terms`'s list SHALL hold the old name `plain-terms-renames` gives for the requirement state and component status `unbuilt`, as a word of its own; the old name of the backlog kind `workaround` only after `clm` and a space and written as `kind: <old name>` inside backticks, never as a word of its own, since the bare word is plain English for what gets in a person's way and stays free to say so; and the old name of the kind `human-task` written as `kind: <old name>` inside backticks, since the bare word is the triage line `human` and plain English. The list SHALL NOT hold the old name of the key `human_burdens`: the word still names the burdens themselves, as the `effort` report's field and its pages' `Declared burdens:` line, so it can't be retired without retiring them too. No `clm:remove-by 20261102-rename-states-and-keys-contract` marker SHALL be left.

#### Scenario: the old subcommand is refused
- GIVEN component `a` with requirement `a/x`
- WHEN `clm` runs with the old name of `clm workaround` and `a/x --title "x"`
- THEN it exits non-zero naming the unknown subcommand, and writes no file under `backlog/`

#### Scenario: the old names retired
- GIVEN the repository
- WHEN the retired-terms test runs
- THEN its list holds the four forms above, and it finds none of them

#### Scenario: the plain word is free
- GIVEN a file the retired-terms test reads holding the old name of the kind `workaround` as a word of its own, in plain English, as in "where most <old name> after install lies"
- WHEN the test runs
- THEN it reports nothing for that line, and it still reports the old name after `clm` and a space and as `kind: <old name>` in backticks

#### Serves
- architecture/plain-terms
