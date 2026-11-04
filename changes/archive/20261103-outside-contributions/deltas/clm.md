---
component: clm
---

## ADDED Requirements

### Requirement: readme-front-page
`README.md` SHALL be the project's front page: what CLM is and who it is for, why it's useful, the commands that install it, a link to the quickstart at `docs/guide/quickstart.md`, a short account of how a change flows from intent to `main`, that CLM is young and changes quickly, how to contribute, with links to `CONTRIBUTING.md` and `.github/SECURITY.md`, and its licence, with a link to `LICENSE`. It SHALL keep its links to the guide, `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/`, `changes/`, `backlog/` and `docs/decisions/`, and to the works CLM draws on.

#### Scenario: a newcomer reads it
- GIVEN this repository's `README.md`
- WHEN a test reads it
- THEN it holds `/plugin marketplace add clunny/clm`, `/plugin install clm@clm`, `docs/guide/quickstart.md`, `CONTRIBUTING.md`, `.github/SECURITY.md`, `LICENSE`, `ARCHITECTURE.md`, `CAPABILITIES.md` and `arXiv:2606.27045`, and each relative link in it names a file or directory that exists

#### Needs
- ci/security-policy

#### Serves
- capability/learn-clm

### Requirement: contributing-intent-only
`CONTRIBUTING.md` SHALL open by saying that CLM takes contributions of intent, reviewed by hand by its maintainer, and that a pull request changing any file outside the spec paths, which it SHALL list, is closed, so a fix to `README.md` or to code arrives as an issue. It SHALL set out the levels (an issue, a pull request adding a short `backlog/<slug>.md` with `title` and `kind: idea`, an elaborated item with `## Intent`, `## Settled`, `## Approaches` and `## Open questions`, and a full change through `/clm:propose`), say to leave out `priority` and to agree an idea in an issue first, that a commit needs an email linked to the author's GitHub account to count for them, to leave "Allow edits by maintainers" on, that the maintainer rebases so a contributor needn't chase `main`, and point at `clm guide contributions`. It SHALL say that the setup `clm/contributing-doc` requires is needed only for a full change, and SHALL no longer tell a contributor to run the code checks before a pull request.

#### Scenario: a contributor reads it
- GIVEN this repository's `CONTRIBUTING.md`
- WHEN a test reads it
- THEN it names each spec path, `backlog/<slug>.md`, `kind: idea`, `priority`, `/clm:propose`, `Allow edits by maintainers` and `clm guide contributions`, and doesn't hold `cargo clippy`

#### Serves
- capability/contribute-from-outside

### Requirement: contribution-command
`clm contribution` SHALL give `host`'s contribution operations as subcommands: `parent` (printing `none` when there is no parent), `sync`, `open <branch> --title <title> --body-file <file>`, `show <number>`, `fetch <number> <branch>`, `comment <number> --body-file <file> [--close]` and `label <number> <label>`. `show` SHALL print each field on its own labelled line, each changed file on its own line and the body last. A subcommand SHALL exit 1 with the reason when the operation fails or there is no host.

#### Scenario: show
- GIVEN `host = "github"` and a fake `gh` reporting pull request 7 with changed file `backlog/x.md`
- WHEN `clm contribution show 7` runs
- THEN it prints the author, the head repository's URL and branch, and `backlog/x.md`, and exits 0

#### Scenario: no parent
- GIVEN `host = "none"`
- WHEN `clm contribution parent` runs
- THEN it prints `none` and exits 0

#### Scenario: no host
- GIVEN `host = "none"`
- WHEN `clm contribution comment 7 --body-file c.md` runs
- THEN it exits 1 saying there is no host

#### Needs
- host/contribution-operations

#### Serves
- capability/contribute-from-outside
