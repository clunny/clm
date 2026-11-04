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
