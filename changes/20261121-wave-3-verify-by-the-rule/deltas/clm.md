---
component: clm
---

## MODIFIED Requirements

### Requirement: verify-command <!-- base: 57b06e540424ce99 -->
`clm` SHALL dispatch `clm verify [--due [--event work-done] [--dry-run] | --commit <sha> | --import | --bisect <sha>]` to `clm-verify`, for the CLM repository found from the current directory (`root-from-cwd`), and `clm --help` SHALL list it. It SHALL call `clm-verify` with its standard call (`verify/mutation-pass`), whose stop request it sets when `clm verify` gets `SIGINT` or `SIGTERM` once it has printed an outcome line starting `verify passed ` until `clm-verify`'s call returns, the mutation pass and the filing of what it found (`verify/files-unproven`) together, a further such signal in that time changing nothing; once the call returns after such a signal, `clm verify` SHALL exit with 128 plus the signal's number. A signal at any other time ends it as it would any process.

#### Scenario: dispatch
- GIVEN a repository `clm init` has just set up
- WHEN `clm verify --due` runs in a subdirectory of it
- THEN it prints `verify off: no [verify] in origin/main:clm.toml` and exits 0

#### Scenario: stopped by hand
- GIVEN a repository whose `[verify]` sets `mutate` to a command that writes one requirement's verdict and a file, and then sleeps, with two requirements due
- WHEN `clm verify --due` gets `SIGTERM` once that file appears
- THEN the command no longer runs, the ledger's new `mutated` record holds one requirement's verdict and the other under `left`, stderr holds `mutation: stopped`, stdout holds `verify passed`, and it exits 143

#### Needs
- verify/verify-due
- verify/verify-run
- verify/verify-import
- verify/verify-commit
- verify/verify-bisect
- verify/settings-from-main
- verify/mutation-pass

### Requirement: readme-front-page <!-- base: 83830808470dead5 -->
`README.md` SHALL be the project's front page: what CLM is and who it is for, solo or as a team, with a link to the site at `https://clm-app.dev` before its first section, why it's useful, the commands that install it, a link to the quickstart at `docs/guide/quickstart.md`, a short account of how a change flows from intent to `main`, that CLM is young and changes quickly, how to contribute, with links to `CONTRIBUTING.md` and `.github/SECURITY.md`, and its licence, with a link to `LICENSE`. It SHALL keep its links to the guide, `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/`, `changes/`, `backlog/` and `docs/decisions/`, and to every work `docs/guide/why-clm.md` credits.

#### Scenario: a newcomer reads it
- GIVEN this repository's `README.md`
- WHEN a test reads it
- THEN it holds `/plugin marketplace add clunny/clm`, `/plugin install clm@clm`, `docs/guide/quickstart.md`, `CONTRIBUTING.md`, `.github/SECURITY.md`, `LICENSE`, `ARCHITECTURE.md`, `CAPABILITIES.md` and `arXiv:2606.27045`, and each relative link in it names a file or directory that exists

#### Scenario: site and credits
- GIVEN this repository's `README.md`
- WHEN a test reads it
- THEN a link to `https://clm-app.dev` comes before its first `##` heading, and it links to `https://kubernetes.io/docs/concepts/architecture/controller/`, `https://github.com/obra/superpowers` and `https://claude.com/blog/the-ai-native-sdlc-playbook`

#### Serves
- capability/learn-clm
