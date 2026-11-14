---
component: config
---

## MODIFIED Requirements

### Requirement: writer-setting <!-- base: 16b0cd7dfe6b8fa4 -->
Loading SHALL give `writer` the value of `CLM_WRITER` when set, otherwise of the clone's `clm.local.toml` when it sets it, otherwise of the user's `config.toml` when it sets it, otherwise none with layer `default`. It SHALL fail naming `clm.toml` and `writer` when the repository's `clm.toml` sets it, and a value that isn't a slug of lowercase letters, digits and `-` SHALL fail naming the variable or file and the key.

#### Scenario: from the variable
- GIVEN `CLM_WRITER=christopher`
- WHEN the settings are loaded
- THEN `writer` is `christopher` from the variable

#### Scenario: from the user's file
- GIVEN a user `config.toml` with `writer = "alice"`
- WHEN the settings are loaded with no variables set
- THEN `writer` is `alice` from the user's file

#### Scenario: from the local file
- GIVEN a user `config.toml` with `writer = "alice"` and a `clm.local.toml` with `writer = "bob"`
- WHEN the settings are loaded with no variables set
- THEN `writer` is `bob` from `clm.local.toml`

#### Scenario: from the repository
- GIVEN a repository `clm.toml` with `writer = "christopher"`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `writer`

#### Scenario: not a slug
- GIVEN `CLM_WRITER="Christopher L"`
- WHEN the settings are loaded
- THEN it fails naming `CLM_WRITER`

#### Scenario: unset
- GIVEN no files and no variables
- WHEN `clm config get writer` runs
- THEN it prints `(git user.name)` and `default`

#### Serves
- capability/work-concurrently
- architecture/git-is-truth
