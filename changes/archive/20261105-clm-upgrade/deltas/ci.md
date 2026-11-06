---
component: ci
---

## MODIFIED Requirements

### Requirement: release <!-- base: 7224f07932e74bd5 -->
The release workflow SHALL run only on `workflow_dispatch`, with a `version` input. It SHALL fail, publishing nothing and leaving `main` unchanged, unless all of these hold: it runs from `main`; `version` is `MAJOR.MINOR.PATCH`; `version` is not lower than the workspace version; no release or tag `v<version>` exists; every target `plugin/bin/clm` downloads an asset for is a target of the workflow's build matrix; and, when `version` equals the workspace version, `crates/clm-upgrade/corpus/v<version>.tar.gz` exists, since a release without a bump records no corpus entry (`release-records-upgrade-corpus`). Otherwise it SHALL:
- when `version` is higher than the workspace version, commit a bump of the workspace version, `Cargo.lock`, `plugin/.claude-plugin/plugin.json` and `plugin-panel/.claude-plugin/plugin.json` to `version`, with the release's upgrade corpus entry and lock lines (`release-records-upgrade-corpus`), and run the tests on that commit;
- build `clm` from that commit for `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin` and `aarch64-apple-darwin`, and for no Windows target (`no-windows-support`), every target on a Linux runner with `cargo zigbuild` and none on a macOS runner (`cross-built-macos-binaries`), each macOS target built for macOS `11.0` or later, failing unless each Linux binary is statically linked;
- publish the release `v<version>` at that commit with `clm-<target>.tar.gz`, holding `clm`, and `clm-<target>.tar.gz.sha256` for every target;
- only after publishing, bring the bump commit into `main`, with a commit message starting `Release v`.

It and the site workflow (`site-deploy`) SHALL be the only workflows in `.github/workflows/`.

#### Scenario: lower version
- GIVEN workspace version `0.2.0`
- WHEN the release workflow's prepare step runs with `version` `0.1.0`
- THEN it fails, calls no `gh release create`, and pushes nothing

#### Scenario: existing release
- GIVEN workspace version `0.1.0` and a `gh` that reports a release `v0.2.0`
- WHEN the prepare step runs with `version` `0.2.0`
- THEN it fails and pushes nothing

#### Scenario: shim names an unbuilt asset
- GIVEN workspace version `0.1.0`, a build matrix without `x86_64-unknown-linux-gnu`, and a `plugin/bin/clm` that maps `Linux-x86_64` to `x86_64-unknown-linux-gnu`
- WHEN the prepare step runs with `version` `0.2.0`
- THEN it fails naming `x86_64-unknown-linux-gnu`, and pushes nothing

#### Scenario: workflow shape
- GIVEN the release workflow file
- WHEN it is parsed
- THEN it triggers only on `workflow_dispatch` with a required `version` input, its build matrix lists exactly the four targets above, with no `-gnu` and no `windows` target and no `windows-latest` runner, every target builds on `ubuntu-latest` with `cargo zigbuild` and no build step runs plain `cargo build`, no job runs on a `macos-*` runner, the Linux targets check the binary is statically linked, the two macOS builds set `MACOSX_DEPLOYMENT_TARGET` to `11.0` and the Linux targets' builds set none, the matrix rows hold only `target`, and the job that pushes to `main` runs `gh release create` before the push

#### Scenario: only workflows
- GIVEN `.github/workflows/`
- WHEN a test lists it
- THEN it holds `release.yml` and `site.yml` and nothing else

#### Scenario: same version
- GIVEN workspace version `0.1.0`, no release or tag `v0.1.0`, and `crates/clm-upgrade/corpus/v0.1.0.tar.gz`
- WHEN the prepare step runs with `version` `0.1.0`
- THEN it succeeds with `sha` set to the dispatched commit, and pushes nothing

#### Scenario: higher version
- GIVEN workspace version `0.1.0` and no release `v0.2.0`
- WHEN the prepare step runs with `version` `0.2.0`
- THEN `release/v0.2.0` on the remote holds one new commit setting `0.2.0` in `Cargo.toml`, `Cargo.lock`, `plugin/.claude-plugin/plugin.json` and `plugin-panel/.claude-plugin/plugin.json`, and the remote's `main` is unchanged

#### Scenario: same version without a corpus entry
- GIVEN workspace version `0.1.0`, no release or tag `v0.1.0`, and no `crates/clm-upgrade/corpus/v0.1.0.tar.gz`
- WHEN the prepare step runs with `version` `0.1.0`
- THEN it fails naming the corpus entry, and pushes nothing

#### Serves
- capability/install-and-release

## ADDED Requirements

### Requirement: release-records-upgrade-corpus
When `release-prepare.sh` bumps the version, it SHALL build `clm` from the bumped tree, run `crates/clm-upgrade/corpus/make.sh <version> <clm>` and `<clm> upgrade --lock <version>` with it, and add their output to the bump commit, before the tests run on that commit. A failure of either SHALL fail `prepare`, publishing nothing.

#### Scenario: a release with a new migration
- GIVEN a temporary repository at version `0.2.0`, with a fake `cargo` whose built `clm`'s `upgrade --lock` appends a line for migration `b`, and a `make.sh` that writes its entry
- WHEN `release-prepare.sh 0.3.0` runs
- THEN the commit on `release/v0.3.0` holds `corpus/v0.3.0.tar.gz` and the lock line for `b`

#### Scenario: corpus entry fails
- GIVEN the same repository, with a `make.sh` that exits 1
- WHEN `release-prepare.sh 0.3.0` runs
- THEN it exits non-zero and pushes no `release/v0.3.0`

#### Needs
- upgrade/corpus-entry-script
- upgrade/shipped-migrations-frozen
- clm/upgrade-command

#### Serves
- capability/upgrade-clm
- architecture/upgrades-carry-migrations
