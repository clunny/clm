---
component: config
---

## MODIFIED Requirements

### Requirement: detector-requires-setting <!-- base: 5f78cf14e0db7641 -->
Loading SHALL give each detector an optional `requires`: the path its `[[detectors]]` table sets, relative to the repository root, or none. A `requires` that isn't a non-empty string SHALL fail naming the file that sets it (`clm.toml` or `clm.local.toml`) and `detectors`.

#### Scenario: requires set
- GIVEN a repository `clm.toml` with a detector `knip`, command `["node_modules/.bin/knip"]` and `requires = "package.json"`
- WHEN the settings load
- THEN `detectors` holds `knip` requiring `package.json`

#### Scenario: requires left out
- GIVEN a repository `clm.toml` with one detector `machete`, command `["cargo-machete"]`, and no `requires`
- WHEN the settings load
- THEN `detectors` holds `machete` requiring nothing

#### Scenario: requires empty
- GIVEN a `clm.local.toml` with a detector whose `requires` is `""`
- WHEN the settings load
- THEN it fails naming `clm.local.toml` and `detectors`

#### Serves
- capability/adopt-clm
