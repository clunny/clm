---
component: clm
---

## ADDED Requirements

### Requirement: init-host-values
`clm init --host <host>` SHALL accept `none`, `git`, `github` and `azure`, passing each to `clm-init` as the `host` setting of that name, and SHALL refuse any other value through its argument parser, exiting non-zero and writing nothing.

#### Scenario: azure given
- GIVEN an empty git repository whose `origin` is `https://github.com/o/r.git`
- WHEN `clm init --host azure` runs
- THEN `clm.toml` holds `host = "azure"`

#### Scenario: unknown host refused
- GIVEN an empty git repository
- WHEN `clm init --host gitlab` runs
- THEN it exits non-zero and no `clm.toml` exists

#### Serves
- capability/git-host

#### Needs
- config/host-setting
