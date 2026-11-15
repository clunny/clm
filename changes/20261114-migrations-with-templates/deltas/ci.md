---
component: ci
---

## MODIFIED Requirements

### Requirement: release-records-upgrade-corpus <!-- base: 211e6d4cac1148f2 -->
When `release-prepare.sh` bumps the version, it SHALL build `clm` from the bumped tree, run `crates/clm-upgrade/corpus/make.sh <version> <clm>` and `<clm> upgrade --lock <version>` with it, and add their output to the bump commit, before the tests run on that commit: the corpus entry, `crates/clm-upgrade/migrations.lock`, and `crates/clm-init/migrations.lock` when the lock step left one. A failure of either SHALL fail `prepare`, publishing nothing.

#### Scenario: a release with a new migration
- GIVEN a temporary repository at version `0.2.0`, with a fake `cargo` whose built `clm`'s `upgrade --lock` appends a line for migration `b`, and a `make.sh` that writes its entry
- WHEN `release-prepare.sh 0.3.0` runs
- THEN the commit on `release/v0.3.0` holds `corpus/v0.3.0.tar.gz` and the lock line for `b`

#### Scenario: a release with a migration of init's
- GIVEN the same repository, with a built `clm` whose `upgrade --lock` writes `crates/clm-init/migrations.lock` with a line for migration `c`
- WHEN `release-prepare.sh 0.3.0` runs
- THEN the commit on `release/v0.3.0` holds that file with the line for `c`

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
