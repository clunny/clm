---
component: ci
---

## MODIFIED Requirements
### Requirement: detector-tools <!-- base: 156131ed66585924 -->
`.github/scripts/detector-tools.sh` SHALL install the tools behind `clm.toml`'s detectors: today only `cargo-machete`, with `cargo install --locked --version <x.y.z> cargo-machete`. Pin the newest release at build time. A detector whose command's first word is `clm` SHALL have no install line: it runs the clone's own `clm`, or the binary under test in `this_repository_is_clean` (`clm/clean-check-runs-own-clm`), and its hint names how a clone gets `main`'s `clm`. Nor SHALL a detector whose command is `sh` and the path of a file in the repository, such as `retired-terms` (`config/retired-terms-detector`): the script needs only `sh` and `grep`, which every machine that runs the checks has. `.github/scripts/ci-check.sh` SHALL run the script before its first `cargo` line, and the release workflow's `prepare` job SHALL run it before `release-prepare.sh`. No other file SHALL install a detector's tool.

#### Scenario: detector tool installed
- GIVEN `.github/scripts/detector-tools.sh`
- WHEN its text is read
- THEN a line installs `cargo-machete` with `--locked` and an exact `--version`

#### Scenario: every detector's tool
- GIVEN this repository's `clm.toml` and `.github/scripts/detector-tools.sh`
- WHEN both are read
- THEN every detector whose command doesn't start with `clm`, nor with `sh` followed by the path of a file that exists in the repository, has a hint ending in its command's first word that is a line of the script, and no line of the script names `clm` or `sh` as the tool it installs

#### Scenario: check script installs the tools
- GIVEN `.github/scripts/ci-check.sh`
- WHEN its text is read
- THEN a line runs `.github/scripts/detector-tools.sh` before the first `cargo` line, and no line runs `cargo install`

#### Scenario: release installs the tools
- GIVEN the release workflow file
- WHEN its `prepare` job is read
- THEN a step runs `.github/scripts/detector-tools.sh`, after the toolchain step and before the step that runs `release-prepare.sh`

#### Serves
- capability/no-dead-weight
- architecture/no-dead-code
