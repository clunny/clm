---
component: clm
base: { frontmatter: "633ac58ed77ef55a" }
---

## MODIFIED Frontmatter
The scan and its list are this repository's own tooling, beside `scripts/dev-install.sh`.

```yaml
paths: ["Cargo.toml", "Cargo.lock", "crates/clm/Cargo.toml", "crates/clm/build.rs", "crates/clm/src/**", "crates/clm/tests/*.rs", "rust-toolchain.toml", ".claude/settings.json", ".gitattributes", "scripts/dev-install.sh", "scripts/retired-terms.sh", "scripts/retired-terms.txt", "README.md", "CONTRIBUTING.md", "LICENSE"]
```

## MODIFIED Requirements
### Requirement: retired-terms <!-- base: c9fe5ba8d8c573b5 -->
`scripts/retired-terms.txt` SHALL hold the list of retired terms, one per line, a line that is blank or starts with `#` holding none, each the old name a completed rename replaced (`plain-terms-renames`) in a form ordinary English never takes: a command after `clm` and a space, a command's outcome line, a flag, a key, a `kind:` written in backticks, a coined term of several words, or a coined word that isn't an English word. A word of plain English SHALL NOT join the list on its own, whatever it once named, since people keep using it in its everyday sense; its retired name is listed in the forms above instead. The scan SHALL be `scripts/retired-terms.sh`, a POSIX `sh` script that needs only `sh` and `grep`: run as `sh scripts/retired-terms.sh [--list <file>] [<path>...]` from the repository root, it reads the list from `<file>` (`scripts/retired-terms.txt` by default) and searches each `<path>`, a file or a directory read recursively, skipping binary files, or with no path the text CLM ships or works from: every file, recursively, under `docs/guide/`, `plugin/`, `specs/` and `web/src/`, and `docs/spec-format.md`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `README.md`. It SHALL print one line `<path>:<line>: <term>` for each line that holds a term as a whole word and exit 1 when it printed any, exit 0 when it printed none, and exit 2 naming the problem when the list can't be read. Matching ignores case on both sides; a whole word is one with no letter, digit or `_` directly before or after it, so a hyphen is a boundary (`frob` matches in `frob-scoped` and not in `frobnicate`), and a term of several words matches with any run of spaces between them. A line holding the text `clm:remove-by` anywhere SHALL be exempt, so an alias may keep an old name until its contract change removes it. History and proposals SHALL be exempt by not being read: `changes/`, archived or live, `docs/decisions/`, `backlog/` and the ledger keep the names they were written with, since a change that describes a rename has to name the old word; what a live change's deltas say reaches `specs/` when it is applied, and is read there. `clm check` runs the scan as a detector (`config/retired-terms-detector`), so a spec branch whose text names a retired term fails before its land. A test in `crates/clm/tests/` SHALL run the scan with no path on this repository, and on the help `clm --help` and `clm <command> --help` print for every command and subcommand, each written to a file of its own named for its command, failing with the scan's lines when it prints any. The change that completes a rename adds the old names it retires, in those forms.

#### Scenario: a retired term in a skill
- GIVEN a list file holding `grommet stub` and a skill's `SKILL.md` line reading `make it a grommet stub`
- WHEN the scan runs with that list on the skill's directory
- THEN it prints that file, its line number and `grommet stub`, and exits 1

#### Scenario: an alias kept for expand-contract
- GIVEN a list file holding `clm frob` and a line reading `` `clm frob` still runs `clm twiddle` // clm:remove-by 20261201-x ``
- WHEN the scan runs with that list on its file
- THEN it prints nothing for that line

#### Scenario: history is left alone
- GIVEN a fixture repository whose `scripts/retired-terms.txt` holds `frobbed`, and whose archived change, live change's `change.md` and decision use it
- WHEN the scan runs with no path at the fixture's root
- THEN it prints nothing and exits 0

#### Scenario: every named path is read
- GIVEN a fixture repository whose `scripts/retired-terms.txt` holds `frobbed`, and a file using it under each directory the scan names and in each of the files it names
- WHEN the scan runs with no path at the fixture's root
- THEN it prints one line for each of them

#### Scenario: whole words only
- GIVEN a list file holding `frob`
- WHEN the scan runs with it on a guide page reading `frobnicate`, and on one reading `the frob-scoped hook`
- THEN it prints nothing for the first and a line for the second

#### Scenario: a multi-word term across spaces
- GIVEN a list file holding `grommet stub`
- WHEN the scan runs with it on a page reading `GROMMET   stub`
- THEN it prints that line

#### Scenario: no list
- GIVEN a list path that names no file
- WHEN the scan runs with it
- THEN it exits 2 naming that path

#### Scenario: no plain English word on the list
- GIVEN the list check, a function over a list of terms, run once on a fixture list holding `clm frob`, `--frob` and the bare word `frob`, and once on `scripts/retired-terms.txt`
- WHEN it judges each term
- THEN on the fixture it fails naming `frob` alone, and on `scripts/retired-terms.txt` it passes: every term holds a space, a hyphen, an underscore or a backtick, or is the old name of the status `unbuilt`

#### Serves
- architecture/plain-terms
