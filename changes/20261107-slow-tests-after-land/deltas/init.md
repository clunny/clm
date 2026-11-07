---
component: init
---

## ADDED Requirements

### Requirement: init-verify-comment
The `clm.toml` that `clm init` writes SHALL end with a commented-out `[verify]` section, every line of it starting `# `, naming `check`, `mode` with its three values, `triggers` with its three values, `every` and `workflow`, and the guide page `ci-and-deployment`, so a project sees where slow tests after the land are set and `upgrade`'s offer of them (`upgrade/migration-offer-verify`) is done. Uncommented, its lines SHALL load as a valid `[verify]` table.

#### Scenario: written commented
- GIVEN an empty git repository
- WHEN `clm init --lang rust` runs
- THEN `clm.toml` holds the line `# [verify]` and no uncommented `[verify]`, loads with no `verify` setting, and names `ci-and-deployment`

#### Scenario: uncommented it loads
- GIVEN that `clm.toml` with the `[verify]` section's leading `# ` removed from each of its lines
- WHEN the settings are loaded
- THEN they hold a `verify` setting

#### Needs
- config/verify-setting

#### Serves
- capability/slow-tests-after-land
