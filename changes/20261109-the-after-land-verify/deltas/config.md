---
component: config
---

## ADDED Requirements

### Requirement: committed-settings
`clm_config::load_committed(root, text, source, env)` SHALL return the settings `load(Some(root), env)` returns, except that the repository's layer SHALL be read from `text`, the contents of a `clm.toml` committed somewhere, or be absent when `text` is `None`, and never from `<root>/clm.toml`; the clone's `clm.local.toml`, the variables and the user's file SHALL be read as `load` reads them. Each value the text sets SHALL have `source`, such as `origin/main:clm.toml`, as its layer, and wherever `load` fails for a `clm.toml` holding that text (an unknown key, an invalid value, a repository-only rule broken, text that isn't TOML) it SHALL fail naming `source` and the key. It exists for a command that judges a commit by the settings committed at it rather than by the checkout it runs in (`verify/settings-from-main`).

#### Scenario: the committed text, not the working file
- GIVEN a root whose `clm.toml` sets no `[verify]`, and a text setting `[verify]` with `check = "x"` and `mode = "local"`
- WHEN `load_committed` reads them with `source` `origin/main:clm.toml`
- THEN `verify` is set with `check = "x"`, and its layer names `origin/main:clm.toml`

#### Scenario: no committed file
- GIVEN a root whose `clm.toml` sets `[verify]`, and no text
- WHEN `load_committed` reads them
- THEN `verify` is unset, with layer `default`

#### Scenario: the clone's file still wins
- GIVEN a text setting `[verify]` with `check = "x"`, and a `clm.local.toml` setting `[verify]` with `check = "y"` and `mode = "local"`
- WHEN `load_committed` reads them
- THEN `verify` has `check = "y"`, and its layer is the `clm.local.toml`

#### Scenario: an error names the commit
- GIVEN a text setting `max_builds = 0`
- WHEN `load_committed` reads it with `source` `origin/main:clm.toml`
- THEN it fails naming `origin/main:clm.toml` and `max_builds`

#### Serves
- capability/slow-tests-after-land
