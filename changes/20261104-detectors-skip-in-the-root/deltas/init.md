---
component: init
---

## MODIFIED Requirements

### Requirement: init-without-manifest <!-- base: 40b1e0197a605c2a -->
For each `--lang`, the `scripts/clm-check.sh` that `clm init` writes SHALL first change to the CLM root (the parent of the script's own directory), wherever it is run from, and then run each language's line only when its manifest exists there: `cargo test` when `Cargo.toml` does, `npm ci && npm test` when `package.json` does, and `pytest` when `pyproject.toml` or `setup.py` does; with none of them it SHALL exit 0. Each detector SHALL have a `requires` naming its language's manifest: `Cargo.toml` for `rust`, `package.json` for `typescript` and `pyproject.toml` for `python`. The `typescript` detector SHALL have the command `["node_modules/.bin/knip"]`, `requires = "package.json"` and the hint `npm ci (or npm install --save-dev knip when package.json doesn't list it)`, so a repository without a `package.json` skips it and one whose dependencies aren't installed reports `could not run` with that hint rather than `failed`. In a repository with no code, the result of `clm init --lang rust --lang typescript --lang python` SHALL pass both `clm check` and `scripts/clm-check.sh`.

#### Scenario: empty repository, every language
- GIVEN an empty git repository, and on `PATH` a `cargo`, `npm` and `pytest` that each exit 1
- WHEN `clm init --lang rust --lang typescript --lang python` runs, then `clm check` and `scripts/clm-check.sh`
- THEN both exit 0, none of the three stubs ran, and `clm check`'s stderr names each detector as skipped

#### Scenario: manifest added later
- GIVEN that repository once it holds a `package.json`, and on `PATH` an `npm` that records its arguments and exits 0
- WHEN `scripts/clm-check.sh` runs
- THEN the stub recorded `ci` and then `test`

#### Scenario: dependencies not installed
- GIVEN a repository holding a `package.json` and no `node_modules/`
- WHEN `clm init --lang typescript` runs and then `clm check`
- THEN `clm check` exits 1 with a line starting `clm.toml: check/detectors: typescript-dead-code could not run:` and ending `; install with: npm ci (or npm install --save-dev knip when package.json doesn't list it)`

#### Serves
- capability/adopt-clm

#### Needs
- clm/detector-requires-skip
- check/detector-relative-command
