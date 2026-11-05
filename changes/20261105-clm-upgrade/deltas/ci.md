---
component: ci
---

## ADDED Requirements

### Requirement: release-records-upgrade-corpus
When `release-prepare.sh` bumps the version, its bump commit SHALL also hold `crates/clm-upgrade/corpus/v<version>/`, written by `crates/clm-upgrade/corpus/make.sh <version>` with the `clm` built from the bumped tree, and one line `<id> <version> <sha256>` appended to `crates/clm-upgrade/migrations.lock` for each migration `clm-upgrade` holds that the lock doesn't yet list, the hash being its module source's. A failure of either SHALL fail `prepare`, publishing nothing.

#### Scenario: a release with a new migration
- GIVEN a temporary repository at version `0.2.0` whose `clm-upgrade` holds migrations `a` and `b`, with `a` already in the lock
- WHEN `release-prepare.sh 0.3.0` runs
- THEN the commit on `release/v0.3.0` holds `corpus/v0.3.0/` and a lock line for `b` naming `0.3.0`, and none added for `a`

#### Scenario: corpus entry fails
- GIVEN the same repository, with a `make.sh` that exits 1
- WHEN `release-prepare.sh 0.3.0` runs
- THEN it exits non-zero and pushes no `release/v0.3.0`

#### Needs
- upgrade/corpus-entry-script

#### Serves
- capability/upgrade-clm
- architecture/upgrades-carry-migrations
